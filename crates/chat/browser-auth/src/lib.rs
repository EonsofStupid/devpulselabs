//! System-browser handoff for named provider authorization.
//!
//! Human authentication must never reuse DevPULSE's CDP browser driver. This
//! crate validates the provider-authored device prompt and dispatches its
//! exact HTTPS URL through the operating system's default-browser service on
//! a bounded worker. It cannot inspect, automate, or close the browser.

use dpl_ai_core::{DeviceAuthorizationPrompt, ProviderId};
#[cfg(test)]
use dpl_ai_core::{GuidedValue, GuidedValueSensitivity};
use std::fmt;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

const GROK_DEVICE_PREFIX: &str = "https://accounts.x.ai/oauth2/device?user_code=";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserAuthEvent {
    Dispatched {
        provider: ProviderId,
    },
    Failed {
        provider: ProviderId,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserAuthError {
    InvalidPrompt,
    DispatchInProgress(ProviderId),
    Worker(String),
}

impl fmt::Display for BrowserAuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrompt => {
                formatter.write_str("the provider authorization target was rejected")
            }
            Self::DispatchInProgress(provider) => {
                write!(
                    formatter,
                    "a system-browser handoff is already running for {provider}"
                )
            }
            Self::Worker(reason) => {
                write!(
                    formatter,
                    "the system browser could not be requested: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for BrowserAuthError {}

trait SystemBrowserOpener: fmt::Debug + Send + Sync {
    fn open(&self, url: &str) -> Result<(), String>;
}

#[derive(Debug, Default)]
struct DefaultSystemBrowserOpener;

impl SystemBrowserOpener for DefaultSystemBrowserOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        webbrowser::open(url).map_err(|error| error.to_string())
    }
}

struct ActiveDispatch {
    provider: ProviderId,
    events: Receiver<BrowserAuthEvent>,
    worker: JoinHandle<()>,
}

pub struct BrowserAuthCoordinator {
    automatic_prompt: Option<DeviceAuthorizationPrompt>,
    active: Option<ActiveDispatch>,
    opener: Arc<dyn SystemBrowserOpener>,
}

impl BrowserAuthCoordinator {
    pub fn new() -> Self {
        Self {
            automatic_prompt: None,
            active: None,
            opener: Arc::<DefaultSystemBrowserOpener>::default(),
        }
    }

    /// Dispatches a newly received prompt once.
    ///
    /// `Ok(false)` means this exact prompt has already been handed to the
    /// system browser. The explicit Reopen action uses [`Self::reopen`].
    pub fn open_once(
        &mut self,
        prompt: DeviceAuthorizationPrompt,
    ) -> Result<bool, BrowserAuthError> {
        validate_prompt(&prompt)?;
        if self.automatic_prompt.as_ref() == Some(&prompt) {
            return Ok(false);
        }
        self.start_dispatch(&prompt)?;
        self.automatic_prompt = Some(prompt);
        Ok(true)
    }

    /// Explicitly dispatches the current prompt again.
    pub fn reopen(&mut self, prompt: DeviceAuthorizationPrompt) -> Result<bool, BrowserAuthError> {
        validate_prompt(&prompt)?;
        self.start_dispatch(&prompt)?;
        Ok(true)
    }

    /// Clears automatic-dispatch deduplication after the provider ceremony
    /// finishes. The external browser remains entirely operator-owned.
    pub fn reset(&mut self, provider: ProviderId) {
        if self
            .automatic_prompt
            .as_ref()
            .is_some_and(|prompt| prompt.provider_id == provider)
        {
            self.automatic_prompt = None;
        }
    }

    pub fn try_event(&mut self) -> Option<BrowserAuthEvent> {
        let active = self.active.as_ref()?;
        let event = match active.events.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => BrowserAuthEvent::Failed {
                provider: active.provider,
                reason: "the system-browser handoff worker disconnected".to_owned(),
            },
        };
        self.finish_active();
        Some(event)
    }

    fn start_dispatch(
        &mut self,
        prompt: &DeviceAuthorizationPrompt,
    ) -> Result<(), BrowserAuthError> {
        if let Some(active) = self.active.as_ref() {
            return Err(BrowserAuthError::DispatchInProgress(active.provider));
        }
        let provider = prompt.provider_id;
        let url = prompt.verification_url.value.clone();
        let opener = Arc::clone(&self.opener);
        let (event_tx, event_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name(format!("devpulse-system-browser-{}", provider.0))
            .spawn(move || dispatch_worker(provider, &url, opener.as_ref(), &event_tx))
            .map_err(|error| BrowserAuthError::Worker(error.to_string()))?;
        self.active = Some(ActiveDispatch {
            provider,
            events: event_rx,
            worker,
        });
        Ok(())
    }

    fn finish_active(&mut self) {
        if let Some(active) = self.active.take() {
            let _ = active.worker.join();
        }
    }

    #[cfg(test)]
    fn with_opener(opener: Arc<dyn SystemBrowserOpener>) -> Self {
        Self {
            automatic_prompt: None,
            active: None,
            opener,
        }
    }
}

impl Default for BrowserAuthCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BrowserAuthCoordinator {
    fn drop(&mut self) {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.worker.is_finished())
        {
            self.finish_active();
        }
    }
}

fn dispatch_worker(
    provider: ProviderId,
    url: &str,
    opener: &dyn SystemBrowserOpener,
    events: &SyncSender<BrowserAuthEvent>,
) {
    let event = match opener.open(url) {
        Ok(()) => BrowserAuthEvent::Dispatched { provider },
        Err(reason) => BrowserAuthEvent::Failed {
            provider,
            reason: public_reason(&reason),
        },
    };
    let _ = events.try_send(event);
}

fn validate_prompt(prompt: &DeviceAuthorizationPrompt) -> Result<(), BrowserAuthError> {
    if prompt.provider_id.0 != "grok" || !valid_user_code(&prompt.user_code.value) {
        return Err(BrowserAuthError::InvalidPrompt);
    }
    let Some(code) = prompt
        .verification_url
        .value
        .strip_prefix(GROK_DEVICE_PREFIX)
    else {
        return Err(BrowserAuthError::InvalidPrompt);
    };
    if code != prompt.user_code.value {
        return Err(BrowserAuthError::InvalidPrompt);
    }
    Ok(())
}

fn valid_user_code(code: &str) -> bool {
    (6..=32).contains(&code.len())
        && code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

fn public_reason(reason: &str) -> String {
    reason.chars().take(240).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    const GROK: ProviderId = ProviderId("grok");

    fn prompt(url: &str, code: &str) -> DeviceAuthorizationPrompt {
        DeviceAuthorizationPrompt {
            provider_id: GROK,
            verification_url: GuidedValue::provider_authoritative(
                url.to_owned(),
                false,
                GuidedValueSensitivity::TransientSecret,
            ),
            user_code: GuidedValue::provider_authoritative(
                code.to_owned(),
                true,
                GuidedValueSensitivity::TransientSecret,
            ),
        }
    }

    #[derive(Debug)]
    struct RecordingOpener {
        calls: Arc<Mutex<Vec<String>>>,
        failure: Option<String>,
    }

    impl SystemBrowserOpener for RecordingOpener {
        fn open(&self, url: &str) -> Result<(), String> {
            self.calls
                .lock()
                .expect("record calls")
                .push(url.to_owned());
            self.failure
                .as_ref()
                .map_or(Ok(()), |reason| Err(reason.clone()))
        }
    }

    fn next_event(coordinator: &mut BrowserAuthCoordinator) -> BrowserAuthEvent {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if let Some(event) = coordinator.try_event() {
                return event;
            }
            assert!(
                Instant::now() < deadline,
                "system-browser worker did not answer"
            );
            thread::yield_now();
        }
    }

    #[test]
    fn only_the_exact_grok_device_endpoint_is_accepted() {
        assert!(validate_prompt(&prompt(
            "https://accounts.x.ai/oauth2/device?user_code=AB12-CD34",
            "AB12-CD34"
        ))
        .is_ok());
        for rejected in [
            "http://accounts.x.ai/oauth2/device?user_code=AB12-CD34",
            "https://accounts.x.ai.evil.test/oauth2/device?user_code=AB12-CD34",
            "https://accounts.x.ai/oauth2/device?user_code=DIFFERENT",
            "https://accounts.x.ai/oauth2/device?user_code=AB12-CD34&next=https://evil.test",
        ] {
            assert!(
                validate_prompt(&prompt(rejected, "AB12-CD34")).is_err(),
                "{rejected}"
            );
        }
    }

    #[test]
    fn user_codes_are_bounded_and_display_safe() {
        for rejected in ["abc-123", "AB 12", "A", &"A".repeat(33)] {
            assert!(!valid_user_code(rejected), "{rejected:?}");
        }
        assert!(valid_user_code("H275-NM42"));
    }

    #[test]
    fn automatic_dispatch_is_deduplicated_but_reopen_is_explicit() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let opener = Arc::new(RecordingOpener {
            calls: Arc::clone(&calls),
            failure: None,
        });
        let mut coordinator = BrowserAuthCoordinator::with_opener(opener);
        let device_prompt = prompt(
            "https://accounts.x.ai/oauth2/device?user_code=AB12-CD34",
            "AB12-CD34",
        );

        assert!(coordinator
            .open_once(device_prompt.clone())
            .expect("first dispatch"));
        assert_eq!(
            next_event(&mut coordinator),
            BrowserAuthEvent::Dispatched { provider: GROK }
        );
        assert!(!coordinator
            .open_once(device_prompt.clone())
            .expect("deduplicated dispatch"));
        assert!(coordinator.reopen(device_prompt).expect("explicit reopen"));
        assert_eq!(
            next_event(&mut coordinator),
            BrowserAuthEvent::Dispatched { provider: GROK }
        );
        assert_eq!(calls.lock().expect("calls").len(), 2);
    }

    #[test]
    fn system_dispatch_failures_are_bounded_and_retryable() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let opener = Arc::new(RecordingOpener {
            calls,
            failure: Some("x".repeat(300)),
        });
        let mut coordinator = BrowserAuthCoordinator::with_opener(opener);
        let device_prompt = prompt(
            "https://accounts.x.ai/oauth2/device?user_code=AB12-CD34",
            "AB12-CD34",
        );

        coordinator
            .open_once(device_prompt.clone())
            .expect("start failed dispatch");
        let BrowserAuthEvent::Failed { provider, reason } = next_event(&mut coordinator) else {
            panic!("expected failed system-browser handoff");
        };
        assert_eq!(provider, GROK);
        assert_eq!(reason.len(), 240);
        coordinator.reopen(device_prompt).expect("retry dispatch");
        assert!(matches!(
            next_event(&mut coordinator),
            BrowserAuthEvent::Failed { .. }
        ));
    }
}

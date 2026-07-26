//! App-owned visible Chromium sessions for named provider authorization.
//!
//! This is intentionally not a general URL launcher. Every accepted endpoint
//! is compiled against one named provider and validated again before the
//! browser worker starts.

use dpl_chat_core::{DeviceAuthorizationPrompt, ProviderId};
use dpl_engine_browser::{Browser, BrowserConfig};
use std::fmt;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const GROK_DEVICE_PREFIX: &str = "https://accounts.x.ai/oauth2/device?user_code=";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserAuthEvent {
    Opened {
        provider: ProviderId,
        browser: String,
    },
    Closed {
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
    AlreadyOpen(ProviderId),
    Worker(String),
}

impl fmt::Display for BrowserAuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrompt => {
                formatter.write_str("the provider authorization target was rejected")
            }
            Self::AlreadyOpen(provider) => {
                write!(
                    formatter,
                    "an authorization window is already open for {provider}"
                )
            }
            Self::Worker(reason) => {
                write!(
                    formatter,
                    "the authorization browser could not start: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for BrowserAuthError {}

#[derive(Debug, Clone, Copy)]
enum BrowserCommand {
    Close,
}

struct ActiveBrowser {
    provider: ProviderId,
    prompt: DeviceAuthorizationPrompt,
    commands: SyncSender<BrowserCommand>,
    events: Receiver<BrowserAuthEvent>,
    worker: Option<JoinHandle<()>>,
}

pub struct BrowserAuthCoordinator {
    active: Option<ActiveBrowser>,
}

impl BrowserAuthCoordinator {
    pub const fn new() -> Self {
        Self { active: None }
    }

    pub fn is_open_for(&self, provider: ProviderId) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.provider == provider)
    }

    /// Starts a visible app-owned Chromium worker without blocking egui.
    ///
    /// `Ok(false)` means the exact prompt is already open.
    pub fn open(&mut self, prompt: DeviceAuthorizationPrompt) -> Result<bool, BrowserAuthError> {
        validate_prompt(&prompt)?;
        if let Some(active) = self.active.as_ref() {
            if active.provider == prompt.provider_id && active.prompt == prompt {
                return Ok(false);
            }
            return Err(BrowserAuthError::AlreadyOpen(active.provider));
        }

        let (command_tx, command_rx) = mpsc::sync_channel(2);
        let (event_tx, event_rx) = mpsc::sync_channel(8);
        let worker_prompt = prompt.clone();
        let worker = thread::Builder::new()
            .name(format!("devpulse-auth-browser-{}", prompt.provider_id.0))
            .spawn(move || browser_worker(worker_prompt, command_rx, event_tx))
            .map_err(|error| BrowserAuthError::Worker(error.to_string()))?;
        self.active = Some(ActiveBrowser {
            provider: prompt.provider_id,
            prompt,
            commands: command_tx,
            events: event_rx,
            worker: Some(worker),
        });
        Ok(true)
    }

    pub fn close(&mut self, provider: ProviderId) -> Result<bool, BrowserAuthError> {
        let Some(active) = self.active.as_ref() else {
            return Ok(false);
        };
        if active.provider != provider {
            return Err(BrowserAuthError::AlreadyOpen(active.provider));
        }
        active
            .commands
            .try_send(BrowserCommand::Close)
            .map_err(|error| BrowserAuthError::Worker(error.to_string()))?;
        Ok(true)
    }

    pub fn try_event(&mut self) -> Option<BrowserAuthEvent> {
        let event = match self.active.as_ref()?.events.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => BrowserAuthEvent::Failed {
                provider: self.active.as_ref()?.provider,
                reason: "the authorization browser worker disconnected".to_owned(),
            },
        };
        if matches!(
            event,
            BrowserAuthEvent::Closed { .. } | BrowserAuthEvent::Failed { .. }
        ) {
            self.finish_active();
        }
        Some(event)
    }

    fn finish_active(&mut self) {
        if let Some(mut active) = self.active.take() {
            if let Some(worker) = active.worker.take() {
                let _ = worker.join();
            }
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
        if let Some(active) = self.active.as_ref() {
            let _ = active.commands.try_send(BrowserCommand::Close);
        }
        self.finish_active();
    }
}

fn browser_worker(
    prompt: DeviceAuthorizationPrompt,
    commands: Receiver<BrowserCommand>,
    events: SyncSender<BrowserAuthEvent>,
) {
    let chromium = match dpl_tools_deps::resolve_current(dpl_tools_deps::CHROMIUM) {
        Ok(chromium) => chromium,
        Err(reason) => {
            let _ = events.try_send(BrowserAuthEvent::Failed {
                provider: prompt.provider_id,
                reason: public_reason(&reason.to_string()),
            });
            return;
        }
    };
    let profile =
        dpl_engine_browser::profile_dir(&format!("provider-auth-{}", prompt.provider_id.0));
    let config = BrowserConfig {
        window: (960, 720),
        executable: Some(chromium.path().to_owned()),
        profile: Some(profile),
        headless: false,
    };
    let mut browser = match Browser::launch(&config) {
        Ok(browser) => browser,
        Err(reason) => {
            let _ = events.try_send(BrowserAuthEvent::Failed {
                provider: prompt.provider_id,
                reason: public_reason(&reason),
            });
            return;
        }
    };
    let browser_identity = {
        let (path, source) = browser.binary();
        format!("{} ({source})", path.display())
    };
    let navigation = browser
        .new_page()
        .and_then(|mut page| page.navigate(&prompt.verification_url));
    if let Err(reason) = navigation {
        let _ = browser.close();
        let _ = events.try_send(BrowserAuthEvent::Failed {
            provider: prompt.provider_id,
            reason: public_reason(&reason),
        });
        return;
    }
    let _ = events.try_send(BrowserAuthEvent::Opened {
        provider: prompt.provider_id,
        browser: browser_identity,
    });

    loop {
        match commands.try_recv() {
            Ok(BrowserCommand::Close) => {
                let _ = browser.close();
                let _ = events.try_send(BrowserAuthEvent::Closed {
                    provider: prompt.provider_id,
                });
                return;
            }
            Err(TryRecvError::Disconnected) => {
                let _ = browser.close();
                return;
            }
            Err(TryRecvError::Empty) => {}
        }
        match browser.try_closed() {
            Ok(true) => {
                let _ = events.try_send(BrowserAuthEvent::Closed {
                    provider: prompt.provider_id,
                });
                return;
            }
            Ok(false) => {}
            Err(reason) => {
                let _ = events.try_send(BrowserAuthEvent::Failed {
                    provider: prompt.provider_id,
                    reason: public_reason(&reason),
                });
                return;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn validate_prompt(prompt: &DeviceAuthorizationPrompt) -> Result<(), BrowserAuthError> {
    if prompt.provider_id.0 != "grok" || !valid_user_code(&prompt.user_code) {
        return Err(BrowserAuthError::InvalidPrompt);
    }
    let Some(code) = prompt.verification_url.strip_prefix(GROK_DEVICE_PREFIX) else {
        return Err(BrowserAuthError::InvalidPrompt);
    };
    if code != prompt.user_code {
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

    const GROK: ProviderId = ProviderId("grok");

    fn prompt(url: &str, code: &str) -> DeviceAuthorizationPrompt {
        DeviceAuthorizationPrompt {
            provider_id: GROK,
            verification_url: url.to_owned(),
            user_code: code.to_owned(),
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
}

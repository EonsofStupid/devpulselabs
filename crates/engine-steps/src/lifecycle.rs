//! Serialized recorder lifecycle and bounded, session-scoped capture work.
use super::*;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};

const QUEUE_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Generation(u64);

#[derive(Clone, Copy)]
struct QueuedTrigger {
    generation: Generation,
    trigger: Trigger,
}

enum QueueError {
    Idle,
    Full,
    Disconnected,
}

trait CaptureBackend: Send + Sync {
    fn capture(&self, path: &Path, excluded_pid: u32) -> Result<engine::StepShot, String>;

    fn settle(&self) {
        std::thread::sleep(Duration::from_millis(SETTLE_MS));
    }

    fn checkpoint(&self, session: &Session) -> Result<(), String> {
        storage::checkpoint(session)
    }
}

struct NativeCapture;

impl CaptureBackend for NativeCapture {
    fn capture(&self, path: &Path, excluded_pid: u32) -> Result<engine::StepShot, String> {
        engine::capture_focused_window_step(path, STEP_QUALITY, excluded_pid)
    }
}

pub struct StepsRecorder {
    inner: Mutex<Option<Session>>,
    /// Lifecycle -> capture -> inner is the only nested lock order.
    lifecycle: Mutex<()>,
    capture: Mutex<()>,
    active: AtomicU64,
    next_generation: AtomicU64,
    dropped: AtomicU64,
    triggers: SyncSender<QueuedTrigger>,
    backend: Arc<dyn CaptureBackend>,
    install_hook_on_start: bool,
    allow_self: AtomicBool,
    sink: EventSink,
}

impl StepsRecorder {
    pub fn new(sink: EventSink) -> Arc<Self> {
        Self::with_backend(sink, Arc::new(NativeCapture), true)
    }

    fn with_backend(
        sink: EventSink,
        backend: Arc<dyn CaptureBackend>,
        install_hook_on_start: bool,
    ) -> Arc<Self> {
        let (triggers, receiver) = sync_channel(QUEUE_CAPACITY);
        let recorder = Arc::new(Self {
            inner: Mutex::new(None),
            lifecycle: Mutex::new(()),
            capture: Mutex::new(()),
            active: AtomicU64::new(0),
            next_generation: AtomicU64::new(1),
            dropped: AtomicU64::new(0),
            triggers,
            backend,
            install_hook_on_start,
            allow_self: AtomicBool::new(false),
            sink,
        });
        // Waiting for input must not keep a recorder (and its sender) alive.
        let weak = Arc::downgrade(&recorder);
        std::thread::spawn(move || {
            while let Ok(work) = receiver.recv() {
                let Some(recorder) = weak.upgrade() else {
                    break;
                };
                recorder.capture_step(work);
            }
        });
        recorder
    }

    /// DRILL-ONLY: include this process's own focused window.
    pub fn set_allow_self(&self, on: bool) {
        self.allow_self.store(on, Ordering::Relaxed);
    }

    fn emit_state(&self, recording: bool, count: u32) {
        (self.sink)(StepsEvent::State { recording, count });
    }

    fn emit_skip(&self, reason: &str) {
        (self.sink)(StepsEvent::Skip(reason.to_owned()));
    }

    fn is_active(&self, generation: Generation) -> bool {
        generation.0 != 0 && self.active.load(Ordering::Acquire) == generation.0
    }

    /// Start a fresh, exclusively reserved source. Pending work must first
    /// be saved or kept archived; even invalid metadata never removes it.
    pub fn start(self: &Arc<Self>, inbox: &Path, meta: CaptureMeta) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().expect("steps lifecycle lock");
        if self.active.load(Ordering::Acquire) != 0 {
            return Err("already recording steps".into());
        }
        if self.has_pending() {
            return Err(
                "save or keep the pending recording archived before starting another".into(),
            );
        }
        normalize_meta_auto(meta, "guide")?;
        let generation = self
            .next_generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| "recording generation exhausted".to_string())?;
        // The first accepted trigger cannot publish count 1 before the
        // initial recording event has been delivered.
        let _capture = self.capture.lock().expect("steps capture lock");
        let session = storage::create(inbox)?;
        *self.inner.lock().expect("steps lock") = Some(session);
        if self.install_hook_on_start {
            ensure_hook(self.clone());
        }
        self.dropped.store(0, Ordering::Relaxed);
        self.active.store(generation, Ordering::Release);
        self.emit_state(true, 0);
        Ok(())
    }

    fn try_enqueue(&self, trigger: Trigger) -> Result<(), QueueError> {
        let generation = Generation(self.active.load(Ordering::Acquire));
        if generation.0 == 0 {
            return Err(QueueError::Idle);
        }
        match self.triggers.try_send(QueuedTrigger {
            generation,
            trigger,
        }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                Err(QueueError::Full)
            }
            Err(TrySendError::Disconnected(_)) => Err(QueueError::Disconnected),
        }
    }

    fn enqueue(&self, trigger: Trigger) -> Result<(), String> {
        self.try_enqueue(trigger).map_err(|error| match error {
            QueueError::Idle => "not recording".into(),
            QueueError::Full => "capture queue is full; wait for the pending steps".into(),
            QueueError::Disconnected => "capture worker is unavailable".into(),
        })
    }

    /// OS hook path: only atomics and a nonblocking bounded queue operation.
    pub(super) fn submit_hook(&self, trigger: Trigger) {
        let _ = self.try_enqueue(trigger);
    }

    /// A marked moment is retained even when the image is unchanged.
    pub fn mark(&self) -> Result<(), String> {
        self.enqueue(Trigger::Marked)
    }

    /// Invalidate queued work, then acknowledge only after capture and its
    /// checkpoint/event publication have quiesced. Call off the paint thread.
    pub fn stop(&self) -> Result<StopResult, String> {
        let _lifecycle = self.lifecycle.lock().expect("steps lifecycle lock");
        self.active.store(0, Ordering::Release);
        let _capture = self.capture.lock().expect("steps capture lock");
        let result = self.pending_result()?;
        self.report_pressure();
        self.emit_state(false, result.count);
        Ok(result)
    }

    fn pending_result(&self) -> Result<StopResult, String> {
        let guard = self.inner.lock().expect("steps lock");
        let session = guard.as_ref().ok_or("no pending recording")?;
        Ok(StopResult {
            count: session.steps.len() as u32,
            seed_title: session
                .steps
                .first()
                .map(|step| step.title.clone())
                .unwrap_or_default(),
        })
    }

    pub fn has_pending(&self) -> bool {
        self.inner.lock().expect("steps lock").is_some()
    }

    pub fn pending_save_meta(&self) -> Result<Option<CaptureMeta>, String> {
        let _lifecycle = self.lifecycle.lock().expect("steps lifecycle lock");
        let session = self.inner.lock().expect("steps lock").clone();
        session
            .as_ref()
            .map(storage::prepared_meta)
            .transpose()
            .map(Option::flatten)
    }

    /// Publish a copy. Retain the same source and retry handle on every error.
    pub fn save(&self, inbox: &Path, meta: CaptureMeta) -> Result<String, String> {
        self.save_with_path(inbox, meta).map(|saved| saved.name)
    }

    /// Publish a copy and return its authoritative destination after any
    /// storage-root rebasing. Hosts should index this path, not reconstruct it.
    pub fn save_with_path(&self, inbox: &Path, meta: CaptureMeta) -> Result<SavedGuide, String> {
        let _lifecycle = self.lifecycle.lock().expect("steps lifecycle lock");
        if self.active.load(Ordering::Acquire) != 0 {
            return Err("stop recording before saving the guide".into());
        }
        let _capture = self.capture.lock().expect("steps capture lock");
        let meta = normalize_meta(meta)?;
        let session = self
            .inner
            .lock()
            .expect("steps lock")
            .clone()
            .ok_or("no guide to save")?;
        if session.steps.is_empty() {
            return Err(
                "no steps captured; keep this source archived to start another recording".into(),
            );
        }
        let saved = storage::save_with_path(&session, inbox, &meta)?;
        *self.inner.lock().expect("steps lock") = None;
        self.emit_state(false, 0);
        (self.sink)(StepsEvent::Saved {
            name: saved.name.clone(),
        });
        Ok(saved)
    }

    /// Detach only after durable archival. Original source bytes are retained.
    pub fn discard(&self) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().expect("steps lifecycle lock");
        self.active.store(0, Ordering::Release);
        let _capture = self.capture.lock().expect("steps capture lock");
        let session = self.inner.lock().expect("steps lock").clone();
        if let Some(session) = session {
            if let Err(error) = storage::archive(&session) {
                self.emit_state(false, session.steps.len() as u32);
                return Err(error);
            }
            *self.inner.lock().expect("steps lock") = None;
        }
        self.emit_state(false, 0);
        Ok(())
    }

    pub fn discover_pending(inbox: &Path) -> Result<storage::RecoveryScan, String> {
        storage::discover(inbox)
    }

    /// Restore durable completed steps into a stopped session; never installs
    /// hooks or begins recording as a consequence of recovering files.
    pub fn recover_pending(&self, dir: &Path) -> Result<StopResult, String> {
        let _lifecycle = self.lifecycle.lock().expect("steps lifecycle lock");
        let _capture = self.capture.lock().expect("steps capture lock");
        if self.active.load(Ordering::Acquire) != 0 || self.has_pending() {
            return Err("resolve the current recording before recovering another".into());
        }
        let session = storage::recover(dir)?;
        *self.inner.lock().expect("steps lock") = Some(session);
        let result = self.pending_result()?;
        self.emit_state(false, result.count);
        Ok(result)
    }

    pub fn status(&self) -> StepsState {
        let guard = self.inner.lock().expect("steps lock");
        StepsState {
            recording: self.active.load(Ordering::Acquire) != 0,
            count: guard
                .as_ref()
                .map(|session| session.steps.len() as u32)
                .unwrap_or(0),
        }
    }

    fn report_pressure(&self) {
        let dropped = self.dropped.swap(0, Ordering::Relaxed);
        if dropped != 0 {
            self.emit_skip(&format!(
                "capture queue full: {dropped} triggers were not recorded"
            ));
        }
    }

    /// Storage failures pause capture until the operator resolves the
    /// pending source. Retrying OS capture errors does not use this path.
    fn pause_for_storage_error(&self, generation: Generation, error: &str) {
        let previous =
            self.active
                .compare_exchange(generation.0, 0, Ordering::AcqRel, Ordering::Acquire);
        if matches!(previous, Ok(_) | Err(0)) {
            self.emit_state(false, self.status().count);
        }
        self.emit_skip(&format!("recording paused; source retained: {error}"));
    }

    fn capture_step(&self, work: QueuedTrigger) {
        let _capture = self.capture.lock().expect("steps capture lock");
        if !self.is_active(work.generation) {
            return;
        }
        self.report_pressure();
        let mut reserved = match self.inner.lock().expect("steps lock").clone() {
            Some(session) => session,
            None => return,
        };
        let attempt = storage::reserve_attempt(&mut reserved);
        *self.inner.lock().expect("steps lock") = Some(reserved);
        let (path, result_path) = match attempt {
            Ok(paths) => paths,
            Err(error) => {
                self.pause_for_storage_error(work.generation, &error);
                return;
            }
        };
        if !self.is_active(work.generation) {
            return;
        }
        let our_pid = if self.allow_self.load(Ordering::Relaxed) {
            0
        } else {
            std::process::id()
        };
        if let Err(error) = storage::capture_target(&path) {
            self.pause_for_storage_error(work.generation, &error);
            return;
        }
        let shot = match self.backend.capture(&path, our_pid) {
            Ok(shot) => shot,
            Err(error) => {
                self.emit_skip(if error.contains("no external window") {
                    "switch to the app you're demonstrating"
                } else {
                    &error
                });
                return;
            }
        };
        if !self.is_active(work.generation) {
            return;
        }
        let mut result_file = None;
        let mut total_bytes = shot.bytes;
        if !work.trigger.is_marked() {
            self.backend.settle();
            if !self.is_active(work.generation) {
                return;
            }
            if let Err(error) = storage::capture_target(&result_path) {
                self.pause_for_storage_error(work.generation, &error);
                return;
            }
            if let Ok(done) = self.backend.capture(&result_path, our_pid) {
                if !self.is_active(work.generation) {
                    return;
                }
                if engine::fingerprint_distance(shot.hash, done.hash) == 0 {
                    // Unchanged/cancelled attempt bytes remain in the source.
                    self.emit_skip("nothing changed");
                    return;
                }
                result_file = Some(file_name(&result_path));
                total_bytes += done.bytes;
            }
        }
        if !self.is_active(work.generation) {
            return;
        }
        let mut candidate = match self.inner.lock().expect("steps lock").clone() {
            Some(session) => session,
            None => return,
        };
        let click = work
            .trigger
            .click_point()
            .and_then(|(x, y)| relative_click(x, y, &shot));
        candidate.steps.push(StepRec {
            order: candidate.steps.len() as u32 + 1,
            file: file_name(&path),
            result_file,
            action: match work.trigger {
                Trigger::Marked => "marked",
                Trigger::KeyAction => "key",
                Trigger::Click { .. } => "click",
            }
            .into(),
            click: click.map(|(x, y)| [x, y]),
            app: shot.app,
            title: shot.title,
            width: shot.width,
            height: shot.height,
            bytes: total_bytes,
        });
        // This is the commit boundary. Stop may invalidate while checkpoint
        // IO completes, but waits on this gate until the durable count and
        // corresponding state event are reflected in memory.
        if !self.is_active(work.generation) {
            return;
        }
        if let Err(error) = self.backend.checkpoint(&candidate) {
            self.pause_for_storage_error(
                work.generation,
                &format!("step checkpoint failed: {error}"),
            );
            return;
        }
        let count = candidate.steps.len() as u32;
        *self.inner.lock().expect("steps lock") = Some(candidate);
        self.emit_state(self.is_active(work.generation), count);
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .expect("reserved capture filename")
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests;

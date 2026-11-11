use super::*;
use std::io::Write;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PauseAt {
    Capture,
    Settle,
    Checkpoint,
    Never,
}

struct FakeCapture {
    calls: AtomicUsize,
    pause_at: PauseAt,
    entered: Sender<()>,
    release: Mutex<Receiver<()>>,
    fail_checkpoint: AtomicBool,
}

impl CaptureBackend for FakeCapture {
    fn capture(&self, path: &Path, _: u32) -> Result<engine::StepShot, String> {
        let call = self.calls.fetch_add(1, Ordering::AcqRel);
        let bytes = b"captured test bytes";
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if call == 0 && self.pause_at == PauseAt::Capture {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
        Ok(engine::StepShot {
            app: "test".into(),
            title: "Test window".into(),
            width: 10,
            height: 10,
            origin_x: 0,
            origin_y: 0,
            bytes: bytes.len(),
            hash: call as u64,
        })
    }

    fn settle(&self) {
        if self.pause_at == PauseAt::Settle {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
    }

    fn checkpoint(&self, session: &Session) -> Result<(), String> {
        if self.pause_at == PauseAt::Checkpoint {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
        if self.fail_checkpoint.load(Ordering::Acquire) {
            Err("injected checkpoint publication failure".into())
        } else {
            storage::checkpoint(session)
        }
    }
}

struct Harness {
    recorder: Arc<StepsRecorder>,
    backend: Arc<FakeCapture>,
    inbox: PathBuf,
    events: Receiver<StepsEvent>,
    entered: Receiver<()>,
    release: Sender<()>,
}

impl Harness {
    fn new(pause_at: PauseAt) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let inbox = std::env::temp_dir().join(format!(
            "dpl-recorder-lifecycle-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&inbox).unwrap();
        let (event_tx, events) = channel();
        let (entered_tx, entered) = channel();
        let (release, release_rx) = channel();
        let backend = Arc::new(FakeCapture {
            calls: AtomicUsize::new(0),
            pause_at,
            entered: entered_tx,
            release: Mutex::new(release_rx),
            fail_checkpoint: AtomicBool::new(false),
        });
        let recorder = StepsRecorder::with_backend(
            Arc::new(move |event| {
                let _ = event_tx.send(event);
            }),
            backend.clone(),
            false,
        );
        Self {
            recorder,
            backend,
            inbox,
            events,
            entered,
            release,
        }
    }

    fn start(&self) {
        self.recorder.start(&self.inbox, meta()).unwrap();
    }

    fn await_count(&self, wanted: u32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let event = self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("recorder event before timeout");
            if matches!(event, StepsEvent::State { count, .. } if count == wanted) {
                return;
            }
            if let StepsEvent::Skip(reason) = event {
                panic!("unexpected skipped capture: {reason}");
            }
        }
    }

    fn source(&self) -> PathBuf {
        self.recorder
            .inner
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .dir
            .clone()
    }
}

fn meta() -> CaptureMeta {
    CaptureMeta {
        library: true,
        show: String::new(),
        episode: String::new(),
        name: "my-guide".into(),
        tags: vec!["test".into()],
    }
}

fn stop_in_background(recorder: &Arc<StepsRecorder>) -> Receiver<Result<StopResult, String>> {
    let (sender, receiver) = channel();
    let worker = recorder.clone();
    std::thread::spawn(move || {
        sender.send(worker.stop()).unwrap();
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while recorder.status().recording {
        assert!(
            Instant::now() < deadline,
            "Stop invalidates active work promptly"
        );
        std::thread::yield_now();
    }
    receiver
}

fn drop_recorder(recorder: Arc<StepsRecorder>) {
    // Only used after Stop in single-trigger tests: the one item has left
    // the capture gate and the queue is empty, with no remaining producers.
    // Let that worker release its temporary Arc before the synchronous drop
    // of the final owner releases the source's OS file lease.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Arc::strong_count(&recorder) != 1 {
        assert!(
            Instant::now() < deadline,
            "worker releases its completed recording"
        );
        std::thread::yield_now();
    }
    drop(recorder);
}

#[test]
fn stop_waits_for_capture_and_old_generations_cannot_enter_a_new_session() {
    let h = Harness::new(PauseAt::Capture);
    h.start();
    let old_generation = Generation(h.recorder.active.load(Ordering::Acquire));
    h.recorder.mark().unwrap();
    h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let source = h.source();
    let stopped = stop_in_background(&h.recorder);
    assert!(matches!(
        stopped.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    h.release.send(()).unwrap();
    assert_eq!(
        stopped
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
            .count,
        0
    );
    assert!(source.read_dir().unwrap().any(|entry| entry
        .unwrap()
        .path()
        .extension()
        .is_some_and(|ext| ext == "webp")));
    let mut acknowledged = false;
    for event in h.events.try_iter() {
        if let StepsEvent::State { recording, .. } = event {
            assert!(
                !acknowledged || !recording,
                "recording event after Stop acknowledgment"
            );
            acknowledged |= !recording;
        }
    }
    assert!(acknowledged);
    h.recorder.discard().unwrap();
    assert!(source.is_dir(), "archival preserves source");
    h.start();
    h.recorder.capture_step(QueuedTrigger {
        generation: old_generation,
        trigger: Trigger::Marked,
    });
    assert_eq!(
        h.backend.calls.load(Ordering::Acquire),
        1,
        "stale work never reaches backend"
    );
    h.recorder.mark().unwrap();
    h.await_count(1);
    assert_eq!(h.recorder.stop().unwrap().count, 1);
}

#[test]
fn bounded_queue_reports_pressure_without_waiting_for_capture() {
    let h = Harness::new(PauseAt::Capture);
    h.start();
    h.recorder.mark().unwrap();
    h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
    for _ in 0..QUEUE_CAPACITY {
        h.recorder.mark().unwrap();
    }
    assert!(h.recorder.mark().unwrap_err().contains("queue is full"));
    // Hook pressure must not invoke callbacks synchronously or block.
    h.recorder.submit_hook(Trigger::Marked);
    assert_eq!(h.recorder.dropped.load(Ordering::Relaxed), 2);
    let stopped = stop_in_background(&h.recorder);
    h.release.send(()).unwrap();
    stopped
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    assert_eq!(h.backend.calls.load(Ordering::Acquire), 1);
}

#[test]
fn stop_during_settle_prevents_second_capture_and_commit() {
    let h = Harness::new(PauseAt::Settle);
    h.start();
    h.recorder.enqueue(Trigger::KeyAction).unwrap();
    h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let stopped = stop_in_background(&h.recorder);
    assert!(matches!(
        stopped.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    h.release.send(()).unwrap();
    assert_eq!(
        stopped
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
            .count,
        0
    );
    assert_eq!(h.backend.calls.load(Ordering::Acquire), 1);
}

#[test]
fn failed_checkpoint_keeps_source_and_empty_pending_session() {
    let h = Harness::new(PauseAt::Never);
    h.backend.fail_checkpoint.store(true, Ordering::Release);
    h.start();
    h.recorder.mark().unwrap();
    let mut paused = false;
    loop {
        let event = h.events.recv_timeout(Duration::from_secs(5)).unwrap();
        if matches!(
            event,
            StepsEvent::State {
                recording: false,
                count: 0
            }
        ) {
            paused = true;
        }
        if let StepsEvent::Skip(reason) = event {
            assert!(reason.contains("checkpoint failed"));
            assert!(reason.contains("source retained"));
            assert!(paused, "stopped state precedes the actionable failure");
            break;
        }
    }
    let source = h.source();
    assert!(
        !h.recorder.status().recording,
        "storage failure pauses capture"
    );
    assert_eq!(h.recorder.stop().unwrap().count, 0);
    assert!(h.recorder.has_pending());
    assert!(h.recorder.save(&h.inbox, meta()).is_err());
    assert!(h.recorder.start(&h.inbox, meta()).is_err());
    assert!(source.read_dir().unwrap().any(|entry| entry
        .unwrap()
        .path()
        .extension()
        .is_some_and(|ext| ext == "webp")));
    drop_recorder(h.recorder);
    let recovery = StepsRecorder::with_backend(Arc::new(|_| {}), Arc::new(NativeCapture), false);
    assert_eq!(
        recovery.recover_pending(&source).unwrap().count,
        0,
        "uncheckpointed frame is not an invented step"
    );
    recovery.discard().unwrap();
    assert!(!recovery.has_pending());
    assert!(source.is_dir());
}

#[test]
fn pending_start_and_failed_save_preserve_retry_and_source() {
    let h = Harness::new(PauseAt::Never);
    h.start();
    assert!(h
        .recorder
        .save(&h.inbox, meta())
        .unwrap_err()
        .contains("stop recording"));
    h.recorder.mark().unwrap();
    h.await_count(1);
    h.recorder.stop().unwrap();
    let source = h.source();
    let original = h.recorder.inner.lock().unwrap().as_ref().unwrap().steps[0].clone();
    let bytes = std::fs::read(source.join(&original.file)).unwrap();
    let invalid = CaptureMeta {
        name: "bad name".into(),
        ..meta()
    };
    assert!(h.recorder.start(&h.inbox, invalid.clone()).is_err());
    assert!(h.recorder.save(&h.inbox, invalid).is_err());
    assert!(h.recorder.has_pending());
    std::fs::create_dir(h.inbox.join("my-guide_v1")).unwrap();
    let saved = h.recorder.save_with_path(&h.inbox, meta()).unwrap();
    assert_eq!(saved.name, "my-guide_v1-2");
    assert_eq!(saved.dir, h.inbox.join(&saved.name));
    assert_eq!(std::fs::read(source.join(&original.file)).unwrap(), bytes);
    assert_eq!(std::fs::read(saved.dir.join(original.file)).unwrap(), bytes);
    assert!(!h.recorder.has_pending());
    assert!(h.recorder.save(&h.inbox, meta()).is_err());
}

#[test]
fn status_reentry_from_sink_never_needs_lifecycle_or_capture_lock() {
    let h = Harness::new(PauseAt::Never);
    let slot = Arc::new(Mutex::new(Weak::<StepsRecorder>::new()));
    let read_slot = slot.clone();
    let observed = Arc::new(AtomicUsize::new(0));
    let callback_observed = observed.clone();
    let recorder = StepsRecorder::with_backend(
        Arc::new(move |_| {
            if let Some(recorder) = read_slot.lock().unwrap().upgrade() {
                let _ = recorder.status();
                callback_observed.fetch_add(1, Ordering::Relaxed);
            }
        }),
        Arc::new(NativeCapture),
        false,
    );
    *slot.lock().unwrap() = Arc::downgrade(&recorder);
    recorder.start(&h.inbox, meta()).unwrap();
    recorder.stop().unwrap();
    assert_eq!(observed.load(Ordering::Relaxed), 2);
}

#[test]
fn stop_waits_for_durable_checkpoint_before_acknowledging_its_count() {
    let h = Harness::new(PauseAt::Checkpoint);
    h.start();
    h.recorder.mark().unwrap();
    h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(h.recorder.status().count, 0);
    let source = h.source();
    let stopped = stop_in_background(&h.recorder);
    assert!(matches!(
        stopped.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    h.release.send(()).unwrap();
    assert_eq!(
        stopped
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
            .count,
        1
    );
    assert!(!h.events.try_iter().any(|event| matches!(
        event,
        StepsEvent::State {
            recording: true,
            count: 1
        }
    )));
    drop_recorder(h.recorder);
    let recovery = StepsRecorder::with_backend(Arc::new(|_| {}), Arc::new(NativeCapture), false);
    assert_eq!(recovery.recover_pending(&source).unwrap().count, 1);
}

#[test]
fn live_source_lease_excludes_a_second_recorder_until_owner_drops() {
    let h = Harness::new(PauseAt::Never);
    h.start();
    h.recorder.mark().unwrap();
    h.await_count(1);
    h.recorder.stop().unwrap();
    let source = h.source();
    let second = StepsRecorder::with_backend(Arc::new(|_| {}), Arc::new(NativeCapture), false);
    assert!(second.recover_pending(&source).is_err());
    assert!(!second.has_pending());
    drop_recorder(h.recorder);
    assert_eq!(second.recover_pending(&source).unwrap().count, 1);
    assert!(!second.status().recording);
}

#[test]
fn initial_recording_event_precedes_first_capture_when_sink_is_delayed() {
    let h = Harness::new(PauseAt::Never);
    let (entered_tx, entered) = channel();
    let (release_tx, release) = channel();
    let release = Mutex::new(release);
    let (event_tx, events) = channel();
    let recorder = StepsRecorder::with_backend(
        Arc::new(move |event| {
            if matches!(
                event,
                StepsEvent::State {
                    recording: true,
                    count: 0
                }
            ) {
                entered_tx.send(()).unwrap();
                release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
            event_tx.send(event).unwrap();
        }),
        h.backend.clone(),
        false,
    );
    let worker = recorder.clone();
    let inbox = h.inbox.clone();
    let start = std::thread::spawn(move || worker.start(&inbox, meta()));
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    recorder.mark().unwrap();
    assert!(
        recorder.capture.try_lock().is_err(),
        "initial event holds the capture barrier"
    );
    assert_eq!(h.backend.calls.load(Ordering::Acquire), 0);
    release_tx.send(()).unwrap();
    start.join().unwrap().unwrap();
    assert!(matches!(
        events.recv_timeout(Duration::from_secs(5)).unwrap(),
        StepsEvent::State {
            recording: true,
            count: 0
        }
    ));
    assert!(matches!(
        events.recv_timeout(Duration::from_secs(5)).unwrap(),
        StepsEvent::State {
            recording: true,
            count: 1
        }
    ));
    recorder.stop().unwrap();
}

#[test]
fn existing_result_target_pauses_without_overwriting_it() {
    let h = Harness::new(PauseAt::Settle);
    h.start();
    h.recorder.enqueue(Trigger::KeyAction).unwrap();
    h.entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let source = h.source();
    let frame = source
        .read_dir()
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|ext| ext == "webp"))
        .unwrap();
    let result = source.join(format!(
        "{}-done.webp",
        frame.file_stem().unwrap().to_string_lossy()
    ));
    std::fs::write(&result, b"existing result must survive").unwrap();
    h.release.send(()).unwrap();
    loop {
        if let StepsEvent::Skip(reason) = h.events.recv_timeout(Duration::from_secs(5)).unwrap() {
            assert!(reason.contains("paused"));
            break;
        }
    }
    assert!(!h.recorder.status().recording);
    assert_eq!(h.recorder.status().count, 0);
    assert_eq!(h.backend.calls.load(Ordering::Acquire), 1);
    assert_eq!(
        std::fs::read(&result).unwrap(),
        b"existing result must survive"
    );
    h.recorder.stop().unwrap();
}

#[test]
fn idle_worker_does_not_own_recorder_forever() {
    let recorder = StepsRecorder::with_backend(Arc::new(|_| {}), Arc::new(NativeCapture), false);
    let weak = Arc::downgrade(&recorder);
    drop(recorder);
    assert!(weak.upgrade().is_none());
}

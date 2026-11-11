//! Steps lifecycle acknowledgements and interrupted-recording recovery.
use super::*;
use std::sync::mpsc::{self, TryRecvError};

impl Viziwiz {
    pub(super) fn begin_steps_stop(&mut self) {
        if self.steps_stop.is_some() || self.steps_save.is_some() || self.pending.is_some() {
            return;
        }
        let recorder = Arc::clone(&self.steps);
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("steps-stop".into())
            .spawn(move || {
                let _ = tx.send(recorder.stop());
            }) {
            Ok(_) => {
                self.steps_stop = Some(rx);
                self.status = "Stopping steps — waiting for the current capture".into();
                self.tab = Tab::Steps;
                self.set_open(true);
            }
            Err(error) => self.status = format!("Could not stop steps: {error}"),
        }
    }

    pub(super) fn poll_steps_stop(&mut self) {
        let Some(rx) = self.steps_stop.as_ref() else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err("Stop worker ended without acknowledgement".into())
            }
        };
        self.steps_stop = None;
        match result {
            Ok(stop) => self.open_steps_landing(stop, None),
            Err(error) => self.status = format!("steps: {error}"),
        }
    }

    pub(super) fn begin_steps_save(&mut self, meta: CaptureMeta) {
        if self.steps_save.is_some() || self.steps_stop.is_some() {
            return;
        }
        let inbox = match Self::inbox() {
            Ok(inbox) => inbox,
            Err(error) => {
                self.status = format!("guide: {error}");
                return;
            }
        };
        let generation = dpl_roots::generation();
        let recorder = Arc::clone(&self.steps);
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("steps-save".into())
            .spawn(move || {
                let result = if generation != dpl_roots::generation() {
                    Err(
                        "Library changed before Save started; retry in the recording's library"
                            .into(),
                    )
                } else {
                    recorder
                        .save_with_path(&inbox, meta.clone())
                        .map(|saved| (saved, meta, generation))
                };
                let _ = tx.send(result);
            }) {
            Ok(_) => {
                self.steps_save = Some(rx);
                self.status = "Saving guide — keeping the source recording".into();
            }
            Err(error) => self.status = format!("Could not start Save: {error}"),
        }
    }

    pub(super) fn poll_steps_save(&mut self) {
        let Some(rx) = self.steps_save.as_ref() else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err("Save worker ended without acknowledgement; source retained".into())
            }
        };
        self.steps_save = None;
        match result {
            Ok((saved, meta, generation)) => {
                self.steps_prepared_meta = None;
                let count = self.pending.take().map(|p| p.count).unwrap_or(0);
                let target = if generation != dpl_roots::generation() {
                    Err("library changed during Save; rescan the original library to index this guide".into())
                } else {
                    dpl_roots::writable(&saved.dir).and_then(|target| {
                        if generation != dpl_roots::generation()
                            || !target.starts_with(dpl_roots::content_root())
                            || !target.join("guide.json").is_file()
                        {
                            Err("saved guide is not available in the current library".into())
                        } else {
                            Ok(target)
                        }
                    })
                };
                let warn = match target {
                    Ok(target) => Self::index_guide(&self.fabric, &target, &meta),
                    Err(error) => {
                        self.fabric
                            .publish_capture(dpl_fabric::CaptureEvent::Failed {
                                error: error.clone(),
                            });
                        format!(" · index deferred: {error}")
                    }
                };
                self.status = format!("guide: {count} steps → {}{warn}", saved.name);
            }
            Err(error) => {
                self.retain_prepared_steps();
                self.status = format!("guide: {error}");
            }
        }
    }

    pub(super) fn open_steps_landing(&mut self, stop: StopResult, prepared: Option<CaptureMeta>) {
        let meta = prepared.clone().unwrap_or_else(|| CaptureMeta {
            library: true,
            show: String::new(),
            episode: String::new(),
            name: seed_name(&stop.seed_title),
            tags: self
                .landing
                .for_kind(CaptureKind::Guide)
                .pattern
                .tags
                .clone(),
        });
        self.pending = Some(PendingLanding {
            kind: CaptureKind::Guide,
            path: PathBuf::new(),
            count: stop.count,
            name: meta.name,
            tags: meta.tags.join(" "),
            library: meta.library,
            show: meta.show,
            episode: meta.episode,
        });
        self.steps_prepared_meta = prepared;
        self.tab = Tab::Steps;
        self.set_open(true);
        self.status = if stop.count == 0 {
            "No completed steps. Archive this recording to start again; source files are retained."
                .into()
        } else if self.steps_prepared_meta.is_some() {
            "Recovered a prepared guide. Save will finish the existing copy.".into()
        } else {
            format!("{} steps pending — name your guide to file it", stop.count)
        };
    }

    pub(super) fn retain_prepared_steps(&mut self) {
        if let Ok(Some(meta)) = self.steps.pending_save_meta() {
            if let Some(pending) = self.pending.as_mut() {
                pending.name = meta.name.clone();
                pending.tags = meta.tags.join(" ");
                pending.library = meta.library;
                pending.show = meta.show.clone();
                pending.episode = meta.episode.clone();
            }
            self.steps_prepared_meta = Some(meta);
        }
    }

    pub(super) fn refresh_steps_recovery(&mut self) {
        if self.capture_blocked() || self.steps.status().recording {
            return;
        }
        match Self::inbox().and_then(|inbox| StepsRecorder::discover_pending(&inbox)) {
            Ok(scan) => {
                self.status = if let Some(issue) = scan.issues.first() {
                    format!(
                        "{} recordings available; {} need attention: {issue}",
                        scan.candidates.len(),
                        scan.issues.len()
                    )
                } else if scan.candidates.is_empty() {
                    "No interrupted recordings in this library".into()
                } else {
                    format!("{} interrupted recordings available", scan.candidates.len())
                };
                self.recoverable_steps = scan.candidates;
            }
            Err(error) => {
                self.recoverable_steps.clear();
                self.status = format!("Recovery: {error}");
            }
        }
    }

    pub(super) fn recover_steps(&mut self, index: usize) {
        if self.capture_blocked() || self.steps.status().recording {
            return;
        }
        let Some(candidate) = self.recoverable_steps.get(index) else {
            return;
        };
        let prepared = candidate.prepared_meta.clone();
        match self.steps.recover_pending(&candidate.dir) {
            Ok(stop) => {
                self.recoverable_steps.clear();
                self.open_steps_landing(stop, prepared);
                // Read again after recovery, so a receipt created since Refresh
                // is authoritative over the cached candidate.
                self.retain_prepared_steps();
            }
            Err(error) => self.status = format!("Recovery: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_waits_for_ack_and_repeated_toggle_cannot_restart() {
        let mut ui = Viziwiz::new(&Fabric::new());
        let (tx, rx) = mpsc::channel();
        ui.steps_stop = Some(rx);
        ui.poll_steps_stop();
        ui.toggle_steps();
        assert!(ui.steps_recording());
        assert!(ui.pending.is_none());
        assert!(ui.capture_blocked());
        tx.send(Ok(StopResult {
            count: 2,
            seed_title: "Downloads - Files".into(),
        }))
        .unwrap();
        ui.poll_steps_stop();
        assert!(!ui.steps_recording());
        let pending = ui.pending.as_ref().unwrap();
        assert_eq!(pending.count, 2);
        assert_eq!(pending.name, "downloads");
        ui.pending.as_mut().unwrap().name = "my-custom-name".into();
        ui.toggle_steps();
        ui.begin_steps_stop();
        assert!(ui.steps_stop.is_none());
        assert_eq!(ui.pending.as_ref().unwrap().name, "my-custom-name");
    }

    #[test]
    fn failed_stop_does_not_invent_a_saved_or_pending_guide() {
        let mut ui = Viziwiz::new(&Fabric::new());
        let (tx, rx) = mpsc::channel::<Result<StopResult, String>>();
        ui.steps_stop = Some(rx);
        drop(tx);
        ui.poll_steps_stop();
        assert!(ui.pending.is_none());
        assert!(ui.steps_stop.is_none());
        assert!(ui.status.contains("without acknowledgement"));
    }

    #[test]
    fn failed_save_keeps_recovered_prepared_metadata_and_draft() {
        let mut ui = Viziwiz::new(&Fabric::new());
        let meta = CaptureMeta {
            library: false,
            show: "course".into(),
            episode: "lesson".into(),
            name: "original_v1".into(),
            tags: vec!["original-tag".into()],
        };
        ui.open_steps_landing(
            StopResult {
                count: 1,
                seed_title: "Different title".into(),
            },
            Some(meta),
        );
        let (tx, rx) = mpsc::channel::<GuideSaveResult>();
        ui.steps_save = Some(rx);
        ui.poll_steps_save();
        assert!(ui.steps_saving());
        ui.discard_landing();
        assert!(ui.pending.is_some(), "Archive cannot detach a running Save");
        tx.send(Err("synthetic copy failure".into())).unwrap();
        ui.poll_steps_save();
        assert!(!ui.steps_saving());
        let pending = ui.pending.as_ref().unwrap();
        assert_eq!(pending.name, "original_v1");
        assert_eq!(pending.tags, "original-tag");
        assert!(!pending.library);
        assert_eq!(pending.episode, "lesson");
        assert!(ui.steps_prepared_meta.is_some());
    }

    #[test]
    fn published_guide_clears_draft_when_library_change_defers_indexing() {
        let mut ui = Viziwiz::new(&Fabric::new());
        ui.open_steps_landing(
            StopResult {
                count: 1,
                seed_title: "Saved guide".into(),
            },
            None,
        );
        let (tx, rx) = mpsc::channel::<GuideSaveResult>();
        ui.steps_save = Some(rx);
        tx.send(Ok((
            dpl_engine_steps::SavedGuide {
                name: "saved_v1".into(),
                dir: PathBuf::from("must-not-be-indexed"),
            },
            CaptureMeta {
                name: "saved_v1".into(),
                tags: vec!["test".into()],
                ..Viziwiz::blank_meta()
            },
            dpl_roots::generation().wrapping_add(1),
        )))
        .unwrap();
        ui.poll_steps_save();
        assert!(ui.pending.is_none());
        assert!(!ui.steps_saving());
        assert!(ui.status.starts_with("guide: 1 steps"));
        assert!(ui.status.contains("index deferred"));
        assert!(ui.status.contains("library changed during Save"));
        assert!(!Path::new("must-not-be-indexed").exists());
    }
}

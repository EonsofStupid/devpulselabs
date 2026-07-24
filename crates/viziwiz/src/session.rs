//! The SESSION recorder (S1, s-adr-sessions.md — RATIFIED native,
//! video-first): the existing video engine pointed at the app's world,
//! plus a typed event track. A session is a FOLDER artifact:
//!
//! ```text
//! inbox/session-<stamp>/
//!   session.mp4        record_ctl_to, primary monitor
//!   events.jsonl       one JSON line per OBSERVED fabric event
//!   session.jpg        poster
//!   artifact.meta.json the X1 sidecar, kind "session"
//! ```
//!
//! LAW: events are observed fabric traffic ONLY — the log's single
//! source is a `subscribe_shell` receiver taken at session start (a
//! broadcast receiver only sees traffic after subscription, which is
//! exactly the wanted semantics). Nothing here ever fabricates a line.
//! Each line is flushed as written — a killed app leaves a valid
//! prefix, and a partial session still indexes (files are the truth).

use dpl_engine_capture as cap;
use dpl_fabric::{ShellEvent, ShellReceiver};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// The append-only event track. One JSON line per event, flushed per
/// line (crash-safe by construction).
pub struct EventLog {
    file: std::fs::File,
    started: Instant,
    count: usize,
}

impl EventLog {
    pub fn create(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::create(path).map_err(|e| format!("events.jsonl: {e}"))?;
        Ok(Self {
            file,
            started: Instant::now(),
            count: 0,
        })
    }

    /// Serialize one OBSERVED event. Typed enum → hand-mapped JSON —
    /// the wire shape is authored here, never screen-scraped.
    pub fn log(&mut self, ev: &ShellEvent) {
        let ms = self.started.elapsed().as_millis() as u64;
        let line = match ev {
            ShellEvent::RouteChanged { route } => serde_json::json!({
                "ms": ms, "type": "route-changed",
                "hub": route.hub.0, "sub": route.sub.0,
            }),
            ShellEvent::ViziwizSummon => serde_json::json!({
                "ms": ms, "type": "viziwiz-summon",
            }),
            ShellEvent::FullBleedRequest(on) => serde_json::json!({
                "ms": ms, "type": "full-bleed", "on": on,
            }),
            ShellEvent::NavigateRequest { route } => serde_json::json!({
                "ms": ms, "type": "navigate-request",
                "hub": route.hub.0, "sub": route.sub.0,
            }),
        };
        // Best-effort per line: a failed write must never take the
        // recording down with it.
        if writeln!(self.file, "{line}").is_ok() {
            let _ = self.file.flush();
            self.count += 1;
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

/// A running session: the Video tab's thread shape + the event track.
pub struct SessionRec {
    pub dir: PathBuf,
    pub stop: Arc<AtomicBool>,
    pub started: Instant,
    pub done: Arc<Mutex<Option<Result<u32, String>>>>,
    pub rx: ShellReceiver,
    pub events: EventLog,
}

impl SessionRec {
    /// Drain everything the fabric published since the last frame into
    /// the track.
    pub fn drain(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            self.events.log(&ev);
        }
    }
}

/// The session folder: `session-<stamp>`, never-clobber suffixed.
pub fn session_dir(inbox: &Path) -> PathBuf {
    let stamp = dpl_engine_steps::compact_timestamp();
    let first = inbox.join(format!("session-{stamp}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| inbox.join(format!("session-{stamp}-{n}")))
        .find(|p| !p.exists())
        .expect("unbounded suffix search")
}

/// Spawn the video leg: `record_ctl_to` → `<dir>/session.mp4`, poster
/// on success — the Video tab's exact options (1080p mp4, silent).
pub fn spawn_video(
    dir: &Path,
    stop: Arc<AtomicBool>,
    done: Arc<Mutex<Option<Result<u32, String>>>>,
) -> Result<(), String> {
    let primary = cap::list_targets()
        .0
        .into_iter()
        .find(|m| m.is_primary)
        .ok_or("no primary monitor")?;
    let out = dir.join("session.mp4");
    std::thread::Builder::new()
        .name("viziwiz-session".into())
        .spawn(move || {
            let opts = cap::VideoOpts {
                height: 1080,
                format: "mp4".into(),
                audio: "none".into(),
                audio_device: None,
                max_seconds: None,
            };
            let r = cap::record_ctl_to(
                &out.to_string_lossy(),
                cap::RecTarget::Monitor(primary.id),
                &opts,
                &stop,
            );
            if r.is_ok() {
                let poster = out.with_file_name("session.jpg");
                cap::poster_for(&out.to_string_lossy(), &poster.to_string_lossy());
            }
            *done.lock().expect("session done lock") = Some(r);
        })
        .map_err(|e| format!("spawn: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_hub::{HubId, Route, SubId};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-session-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn event_lines_are_valid_json_ordered_and_flushed_as_written() {
        let dir = scratch("log");
        let path = dir.join("events.jsonl");
        let mut log = EventLog::create(&path).unwrap();
        log.log(&ShellEvent::RouteChanged {
            route: Route {
                hub: HubId("present"),
                sub: SubId("shows"),
            },
        });
        log.log(&ShellEvent::FullBleedRequest(true));
        log.log(&ShellEvent::ViziwizSummon);
        assert_eq!(log.count(), 3);

        // Read back WHILE the log is still open — flushed-per-line is
        // the crash-safety contract (a killed app leaves this prefix).
        let raw = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<serde_json::Value> = raw
            .lines()
            .map(|l| serde_json::from_str(l).expect("every line parses"))
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0]["type"], "route-changed");
        assert_eq!(lines[0]["hub"], "present");
        assert_eq!(lines[1]["type"], "full-bleed");
        assert_eq!(lines[1]["on"], true);
        assert_eq!(lines[2]["type"], "viziwiz-summon");
        let ms: Vec<u64> = lines.iter().map(|l| l["ms"].as_u64().unwrap()).collect();
        assert!(ms.windows(2).all(|w| w[0] <= w[1]), "monotonic ms: {ms:?}");
    }

    #[test]
    fn the_log_only_carries_observed_fabric_traffic() {
        // The receiver is the ONLY source: what was published is what
        // lands, in order, and nothing else.
        let dir = scratch("observed");
        let fabric = dpl_fabric::Fabric::new();
        let mut rx = fabric.subscribe_shell();
        fabric.publish_shell(ShellEvent::FullBleedRequest(true));
        fabric.publish_shell(ShellEvent::FullBleedRequest(false));
        let mut log = EventLog::create(&dir.join("events.jsonl")).unwrap();
        while let Ok(ev) = rx.try_recv() {
            log.log(&ev);
        }
        assert_eq!(log.count(), 2);
        let raw = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert_eq!(raw.lines().count(), 2);
        assert!(raw.lines().all(|l| l.contains("full-bleed")));
    }

    #[test]
    fn session_dirs_never_clobber() {
        let inbox = scratch("dirs");
        let first = session_dir(&inbox);
        std::fs::create_dir_all(&first).unwrap();
        let second = session_dir(&inbox);
        assert_ne!(first, second, "an existing folder is never reused");
        assert!(second
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("session-"));
    }
}

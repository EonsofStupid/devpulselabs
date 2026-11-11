//! Real filesystem recovery/publication drill with synthetic frame bytes.
//! Does not install input hooks or capture a window.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use dpl_engine_steps::{CaptureMeta, StepRec, StepsRecorder};
use serde_json::json;

fn write(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::current_dir()?
        .join("target")
        .join(format!("recorder-recovery-{token}"));
    let inbox = root.join("Inbox");
    let source_id = format!(".steps-recording-drill-{token}");
    let source = inbox.join(&source_id);
    fs::create_dir_all(&source)?;
    write(&source.join(".steps-lock"), b"")?;
    let frame_name = "capture-00000000000000000001.webp";
    let frame = b"synthetic fixture: not a WebP image";
    write(&source.join(frame_name), frame)?;
    write(&source.join(".attempt-00000000000000000001"), b"")?;
    write(
        &source.join(".steps-source.json"),
        &serde_json::to_vec(&json!({"version":1,"source_id":source_id}))?,
    )?;
    let step = StepRec {
        order: 1,
        file: frame_name.into(),
        result_file: None,
        action: "marked".into(),
        click: None,
        app: "synthetic fixture".into(),
        title: "Recovery drill".into(),
        width: 32,
        height: 32,
        bytes: frame.len(),
    };
    let checkpoint = serde_json::to_vec(
        &json!({"version":1,"source_id":source_id,"next_attempt":2,"steps":[step]}),
    )?;
    write(&source.join("checkpoint-0000000001.json"), &checkpoint)?;

    let scan = StepsRecorder::discover_pending(&inbox)?;
    assert!(scan.issues.is_empty());
    let candidates = scan.candidates;
    assert_eq!(candidates.len(), 1);
    let recorder = StepsRecorder::new(Arc::new(|_| {}));
    let recovered = recorder.recover_pending(&candidates[0].dir)?;
    assert_eq!(recovered.count, 1);
    assert!(!recorder.status().recording);
    let name = recorder.save(
        &inbox,
        CaptureMeta {
            library: true,
            show: String::new(),
            episode: String::new(),
            name: "recovery-drill".into(),
            tags: vec!["fixture".into()],
        },
    )?;
    let target = inbox.join(&name);
    assert!(target.join("guide.json").is_file());
    assert!(target.join(".guide-revision-complete").is_file());
    assert!(!target.join(".guide-revision-pending").exists());
    let guide = dpl_guide::load_guide(&target)?;
    assert_eq!(guide.step_count, 1);
    assert_eq!(guide.steps.len(), 1);
    assert_eq!(fs::read(source.join(frame_name))?, frame);
    assert_eq!(
        fs::read(source.join("checkpoint-0000000001.json"))?,
        checkpoint
    );
    assert_eq!(fs::read(target.join(frame_name))?, frame);
    let after_save = StepsRecorder::discover_pending(&inbox)?;
    assert!(after_save.candidates.is_empty() && after_save.issues.is_empty());
    let after_restart = StepsRecorder::new(Arc::new(|_| {}));
    assert!(after_restart.recover_pending(&source).is_err());
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "fixture": "synthetic bytes; no OS hook or capture", "source": source, "published": target,
            "recovered_count": recovered.count, "recovered_stopped": true, "source_bytes_preserved": true,
            "destination_bytes_match": true, "publication_complete": true, "restart_duplicate_refused": true,
            "native_guide_reader_accepted": true,
            "pending_discovery_after_save": 0
        }))?
    );
    Ok(())
}

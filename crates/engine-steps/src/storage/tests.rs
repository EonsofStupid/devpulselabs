use super::*;
use std::collections::BTreeMap;

fn fixture() -> (PathBuf, Session) {
    let inbox = std::env::temp_dir().join(format!("dpl-recorder-storage-{}", token()));
    let mut session = create(&inbox).expect("create owned recording");
    add_step(&mut session);
    (inbox, session)
}

fn add_step(session: &mut Session) {
    let (frame, _) = reserve_attempt(session).expect("reserve frame");
    write_new(&frame, b"synthetic frame bytes").expect("fixture frame");
    session.steps.push(StepRec {
        order: session.steps.len() as u32 + 1,
        file: frame.file_name().unwrap().to_str().unwrap().into(),
        result_file: None,
        action: "marked".into(),
        click: None,
        app: "fixture".into(),
        title: "Example guide".into(),
        width: 16,
        height: 16,
        bytes: 21,
    });
    checkpoint(session).expect("durable step");
}

fn meta() -> CaptureMeta {
    CaptureMeta {
        library: true,
        show: String::new(),
        episode: String::new(),
        name: "example_v1".into(),
        tags: vec!["guide".into()],
    }
}

fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    entries(dir)
        .unwrap()
        .into_iter()
        .filter(|(name, _)| name != LEASE)
        .map(|(name, path)| (name, fs::read(path).unwrap()))
        .collect()
}

fn assert_preserved(dir: &Path, before: &BTreeMap<String, Vec<u8>>) {
    for (name, bytes) in before {
        assert_eq!(
            &fs::read(dir.join(name)).unwrap(),
            bytes,
            "source changed: {name}"
        );
    }
}

#[test]
fn recovery_uses_committed_steps_and_skips_reserved_attempts() {
    let (inbox, mut session) = fixture();
    let (uncommitted, _) = reserve_attempt(&mut session).unwrap();
    write_new(&uncommitted, b"interrupted capture").unwrap();
    let dir = session.dir.clone();
    drop(session);
    let scan = discover(&inbox).unwrap();
    assert!(scan.issues.is_empty());
    let candidates = scan.candidates;
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].count, 1);
    assert!(candidates[0].prepared_meta.is_none());
    let mut recovered = recover(&dir).unwrap();
    assert_eq!(recovered.steps.len(), 1);
    let (fresh, _) = reserve_attempt(&mut recovered).unwrap();
    assert_ne!(fresh, uncommitted);
    assert_eq!(fs::read(uncommitted).unwrap(), b"interrupted capture");
}

#[test]
fn save_copies_source_once_and_preserves_colliding_destination() {
    let (inbox, session) = fixture();
    let original = snapshot(&session.dir);
    let collision = inbox.join(meta().name);
    fs::create_dir(&collision).unwrap();
    write_new(&collision.join("keep.txt"), b"original").unwrap();
    let saved = save(&session, &inbox, &meta()).unwrap();
    assert_eq!(saved, "example_v1-2");
    assert_preserved(&session.dir, &original);
    assert_eq!(fs::read(collision.join("keep.txt")).unwrap(), b"original");
    assert!(inbox.join(&saved).join(COMPLETE).is_file());
    assert!(!inbox.join(&saved).join(PENDING).exists());
    let guide = dpl_guide::load_guide(&inbox.join(&saved))
        .expect("native guide reader accepts recorder output");
    assert_eq!(guide.step_count, 1);
    assert_eq!(guide.steps.len(), 1);
    assert_eq!(guide.name, saved);
    assert_eq!(save(&session, &inbox, &meta()).unwrap(), saved);
    let dir = session.dir.clone();
    drop(session);
    assert!(discover(&inbox).unwrap().candidates.is_empty());
    assert!(recover(&dir).unwrap_err().contains("already saved"));
}

#[test]
fn copy_manifest_and_publication_failures_keep_source_retryable() {
    for fail_at in [
        SaveStage::Copying,
        SaveStage::Copied,
        SaveStage::Manifest,
        SaveStage::Prepared,
    ] {
        let (inbox, session) = fixture();
        let original = snapshot(&session.dir);
        let failure = save_impl(&session, &inbox, &meta(), |at| {
            if at == fail_at {
                Err("injected I/O failure".into())
            } else {
                Ok(())
            }
        });
        assert!(failure.unwrap_err().contains("injected"));
        assert_preserved(&session.dir, &original);
        let expected = session.steps.clone();
        let dir = session.dir.clone();
        drop(session);
        let session = recover(&dir).unwrap();
        assert_eq!(session.steps, expected);
        assert!(inbox.join("example_v1").join(PENDING).is_file());
        let result = save(&session, &inbox, &meta()).unwrap();
        assert_eq!(
            result,
            if fail_at == SaveStage::Prepared {
                "example_v1"
            } else {
                "example_v1-2"
            }
        );
        assert_preserved(&session.dir, &original);
        let published = entries(&inbox)
            .unwrap()
            .into_iter()
            .filter(|(_, path)| path.join(COMPLETE).is_file())
            .count();
        assert_eq!(published, 1);
    }
}

#[test]
fn restart_restores_prepared_metadata_and_finishes_same_target() {
    let (inbox, session) = fixture();
    save_impl(&session, &inbox, &meta(), |at| {
        if at == SaveStage::Prepared {
            Err("interrupted".into())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    drop(session);
    let candidates = discover(&inbox).unwrap().candidates;
    assert_eq!(candidates[0].prepared_meta, Some(meta()));
    let recovered = recover(&candidates[0].dir).unwrap();
    assert_eq!(prepared_meta(&recovered).unwrap(), Some(meta()));
    let mut changed = meta();
    changed.tags.push("changed".into());
    assert!(save(&recovered, &inbox, &changed)
        .unwrap_err()
        .contains("original tags"));
    assert_eq!(save(&recovered, &inbox, &meta()).unwrap(), "example_v1");
    drop(recovered);
    assert!(discover(&inbox).unwrap().candidates.is_empty());
}

#[test]
fn archived_source_is_preserved_and_not_offered_again() {
    let (inbox, session) = fixture();
    let original = snapshot(&session.dir);
    archive(&session).unwrap();
    archive(&session).unwrap();
    assert_preserved(&session.dir, &original);
    assert!(save(&session, &inbox, &meta())
        .unwrap_err()
        .contains("archived"));
    let dir = session.dir.clone();
    drop(session);
    assert!(discover(&inbox).unwrap().candidates.is_empty());
    assert!(recover(&dir).unwrap_err().contains("archived"));
}

#[test]
fn corrupt_latest_complete_checkpoint_refuses_instead_of_rolling_back() {
    let (_, session) = fixture();
    write_new(&session.dir.join(checkpoint_name(2)), b"{broken").unwrap();
    let dir = session.dir.clone();
    drop(session);
    assert!(recover(&dir).is_err());
    assert!(dir.join(checkpoint_name(1)).is_file());
}

#[test]
fn interrupted_staging_is_ignored_without_removal() {
    let (_, session) = fixture();
    let stage = session.dir.join(".steps-stage-interrupted.pending");
    write_new(&stage, b"{incomplete").unwrap();
    let dir = session.dir.clone();
    drop(session);
    assert_eq!(recover(&dir).unwrap().steps.len(), 1);
    assert_eq!(fs::read(stage).unwrap(), b"{incomplete");
}

#[test]
fn unsafe_references_duplicate_fields_and_frame_size_mismatch_are_rejected() {
    for name in ["../outside.webp", "C:secret", "CON", ".steps-source.json"] {
        let (_, mut session) = fixture();
        session.steps[0].file = name.into();
        assert!(checkpoint(&session).is_err());
    }
    let (_, mut session) = fixture();
    session.steps[0].bytes += 1;
    assert!(checkpoint(&session).unwrap_err().contains("size"));
    let (_, session) = fixture();
    let source_id = owner(&session.dir).unwrap().source_id.0;
    let malformed = format!("{{\"version\":1,\"version\":1,\"source_id\":\"{source_id}\",\"next_attempt\":2,\"steps\":[]}}");
    write_new(&session.dir.join(checkpoint_name(2)), malformed.as_bytes()).unwrap();
    let dir = session.dir.clone();
    drop(session);
    assert!(recover(&dir).unwrap_err().contains("duplicate field"));
}

#[test]
fn non_files_and_existing_attempts_are_never_overwritten() {
    let (_, mut session) = fixture();
    let next = session.next_attempt;
    let (name, _) = frame_names(next);
    write_new(&session.dir.join(&name), b"keep existing frame").unwrap();
    let (reserved, _) = reserve_attempt(&mut session).unwrap();
    assert_ne!(reserved.file_name().unwrap().to_str().unwrap(), name);
    assert_eq!(
        fs::read(session.dir.join(name)).unwrap(),
        b"keep existing frame"
    );
    fs::create_dir(session.dir.join("nested")).unwrap();
    let dir = session.dir.clone();
    drop(session);
    assert!(recover(&dir).unwrap_err().contains("regular files"));
}

#[test]
fn existing_checkpoint_is_immutable_even_when_candidate_is_valid() {
    let (_, mut session) = fixture();
    let original = fs::read(session.dir.join(checkpoint_name(1))).unwrap();
    session.steps[0].title = "changed".into();
    assert!(checkpoint(&session).unwrap_err().contains("immutable"));
    assert_eq!(
        fs::read(session.dir.join(checkpoint_name(1))).unwrap(),
        original
    );
}

#[test]
fn source_copy_to_another_inbox_is_refused_without_a_second_publication() {
    let (_, session) = fixture();
    let other = std::env::temp_dir().join(format!("dpl-recorder-other-{}", token()));
    assert!(save(&session, &other, &meta())
        .unwrap_err()
        .contains("original inbox"));
    assert!(!present(&session.dir.join(RECEIPT)).unwrap());
}

#[test]
fn prepared_destination_corruption_is_not_accepted_as_a_completed_save() {
    let (inbox, session) = fixture();
    save_impl(&session, &inbox, &meta(), |at| {
        if at == SaveStage::Prepared {
            Err("interrupt".into())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    // Simulate external corruption only inside this synthetic test fixture.
    fs::write(
        inbox.join("example_v1").join(&session.steps[0].file),
        b"corrupted",
    )
    .unwrap();
    assert!(save(&session, &inbox, &meta())
        .unwrap_err()
        .contains("differs"));
    assert!(inbox.join("example_v1").join(PENDING).is_file());
    assert_eq!(
        fs::read(session.dir.join(&session.steps[0].file)).unwrap(),
        b"synthetic frame bytes"
    );
}

#[test]
fn source_symlinks_are_refused_when_platform_can_create_them() {
    let (_, session) = fixture();
    let link = session.dir.join("untrusted-link");
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(&session.steps[0].file, &link);
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_file(&session.steps[0].file, &link);
    let dir = session.dir.clone();
    drop(session);
    #[cfg(any(unix, windows))]
    match result {
        Ok(()) => assert!(recover(&dir).unwrap_err().contains("regular files")),
        Err(e) => eprintln!("symlink fixture unavailable on this host: {e}"),
    }
}

#[test]
fn live_session_and_its_clones_hold_an_os_lease_until_the_last_drop() {
    let (inbox, session) = fixture();
    let dir = session.dir.clone();
    let clone = session.clone();
    assert!(recover(&dir).unwrap_err().contains("in use"));
    assert!(discover(&inbox).unwrap().candidates.is_empty());
    drop(session);
    assert!(recover(&dir).unwrap_err().contains("in use"));
    drop(clone);
    let recovered = recover(&dir).unwrap();
    assert_eq!(recovered.steps.len(), 1);
    assert!(recover(&dir).unwrap_err().contains("in use"));
}

#[test]
fn empty_checkpoint_can_be_recovered_and_archived_after_owner_exits() {
    let inbox = std::env::temp_dir().join(format!("dpl-recorder-empty-{}", token()));
    let mut session = create(&inbox).unwrap();
    let (frame, _) = reserve_attempt(&mut session).unwrap();
    write_new(&frame, b"cancelled first capture").unwrap();
    assert!(discover(&inbox).unwrap().candidates.is_empty());
    let dir = session.dir.clone();
    drop(session);
    let scan = discover(&inbox).unwrap();
    assert!(scan.issues.is_empty());
    assert_eq!(scan.candidates[0].count, 0);
    let recovered = recover(&dir).unwrap();
    assert!(save(&recovered, &inbox, &meta())
        .unwrap_err()
        .contains("no steps"));
    archive(&recovered).unwrap();
    drop(recovered);
    assert!(discover(&inbox).unwrap().candidates.is_empty());
    assert_eq!(fs::read(frame).unwrap(), b"cancelled first capture");
}

#[test]
fn malformed_source_does_not_hide_healthy_recovery() {
    let (inbox, broken) = fixture();
    write_new(&broken.dir.join(checkpoint_name(2)), b"{corrupt").unwrap();
    let mut healthy = create(&inbox).unwrap();
    add_step(&mut healthy);
    let healthy_dir = healthy.dir.clone();
    drop(broken);
    drop(healthy);
    let scan = discover(&inbox).unwrap();
    assert_eq!(scan.candidates.len(), 1);
    assert_eq!(scan.candidates[0].dir, healthy_dir);
    assert_eq!(scan.issues.len(), 1);
    assert!(scan.issues[0].contains("Source files are retained"));
}

#[test]
fn missing_prepared_target_is_reported_without_creating_a_second_copy() {
    let (inbox, session) = fixture();
    publish_json(
        &session.dir.join(RECEIPT),
        &Receipt {
            version: 1,
            source_id: owner(&session.dir).unwrap().source_id,
            target: "missing_v1".into(),
            count: 1,
            meta: meta(),
        },
    )
    .unwrap();
    assert!(save(&session, &inbox, &meta()).is_err());
    let dir = session.dir.clone();
    drop(session);
    let scan = discover(&inbox).unwrap();
    assert!(scan.candidates.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert!(recover(&dir).is_err());
    assert!(!inbox.join("example_v1").exists());
    assert!(!inbox.join("missing_v1").exists());
}

#[test]
fn discovery_of_missing_inbox_does_not_create_it() {
    let inbox = std::env::temp_dir().join(format!("dpl-recorder-absent-{}", token()));
    let scan = discover(&inbox).unwrap();
    assert!(scan.candidates.is_empty() && scan.issues.is_empty());
    assert!(!inbox.exists());
}

#[test]
fn capture_target_refuses_existing_or_unowned_paths_without_writes() {
    let (_, mut session) = fixture();
    let (frame, _) = reserve_attempt(&mut session).unwrap();
    capture_target(&frame).unwrap();
    write_new(&frame, b"first frame").unwrap();
    assert!(capture_target(&frame)
        .unwrap_err()
        .contains("already exists"));
    assert_eq!(fs::read(frame).unwrap(), b"first frame");
    assert!(capture_target(&session.dir.join("unreserved.webp")).is_err());
}

#[test]
fn completed_guides_remain_filed_after_their_published_content_is_edited() {
    let (inbox, session) = fixture();
    let name = save(&session, &inbox, &meta()).unwrap();
    let source = session.dir.clone();
    // Simulates a later edit of this synthetic guide, keeping its source marker.
    fs::write(
        inbox.join(&name).join(&session.steps[0].file),
        b"legitimately edited image",
    )
    .unwrap();
    fs::write(
        inbox.join(&name).join("guide.json"),
        b"edited manifest fixture",
    )
    .unwrap();
    drop(session);
    let scan = discover(&inbox).unwrap();
    assert!(scan.candidates.is_empty() && scan.issues.is_empty());
    assert!(recover(&source).unwrap_err().contains("already saved"));
    assert!(!inbox.join("example_v1-2").exists());
}

#[test]
fn recorder_publication_loads_and_supports_repeated_preserving_guide_edits() {
    let (inbox, session) = fixture();
    let saved = save_with_path(&session, &inbox, &meta()).unwrap();
    assert_eq!(saved.dir.file_name().unwrap().to_str().unwrap(), saved.name);
    assert_eq!(dpl_guide::load_guide(&saved.dir).unwrap().step_count, 1);
    let original = snapshot(&saved.dir);
    let mut step: crate::edit::GuideStepEdit =
        serde_json::from_value(serde_json::to_value(&session.steps[0]).unwrap()).unwrap();
    step.caption = Some("First review".into());
    let revision = crate::edit::update_guide(&saved.dir, vec![step]).unwrap();
    assert_eq!(dpl_guide::load_guide(&revision.dir).unwrap().step_count, 1);
    assert_preserved(&saved.dir, &original);
    let revision_before = snapshot(&revision.dir);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(revision.dir.join("guide.json")).unwrap()).unwrap();
    let mut step: crate::edit::GuideStepEdit =
        serde_json::from_value(manifest["steps"][0].clone()).unwrap();
    step.caption = Some("Second review".into());
    let second = crate::edit::update_guide(&revision.dir, vec![step]).unwrap();
    assert_eq!(dpl_guide::load_guide(&second.dir).unwrap().step_count, 1);
    assert_preserved(&saved.dir, &original);
    assert_preserved(&revision.dir, &revision_before);
}

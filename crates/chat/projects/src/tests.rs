use super::*;
use serde_json::json;

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir()
        .join("dpl-chat-projects-tests")
        .join(ProjectId::mint().unwrap().as_str());
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn workspace(base: &Path, name: &str) -> PathBuf {
    let dir = base.join(name);
    fs::create_dir(&dir).unwrap();
    dir
}

#[test]
fn existing_registry_open_never_creates_or_adopts_a_missing_registry() {
    let base = scratch();
    let missing = base.join("missing");
    assert!(Registry::open_existing(&missing).is_err());
    assert!(!missing.exists());
    let unmarked = workspace(&base, "unmarked");
    assert!(Registry::open_existing(&unmarked).is_err());
    assert_eq!(fs::read_dir(&unmarked).unwrap().count(), 0);
    let registry = Registry::open(&missing).unwrap();
    let project = Project::create_new(&workspace(&base, "project"), "Existing").unwrap();
    registry.activate(&project).unwrap();
    let before = fs::read(missing.join("registry.json")).unwrap();
    let opened = Registry::open_existing(&missing).unwrap();
    assert_eq!(opened.open_active().unwrap().unwrap().id(), project.id());
    assert_eq!(fs::read(missing.join("registry.json")).unwrap(), before);
}

#[test]
fn project_and_registry_round_trip_isolate_a_b_a_and_restart() {
    let base = scratch();
    let a_dir = workspace(&base, "a");
    let b_dir = workspace(&base, "b");
    let a = Project::create(&a_dir, "Alpha").unwrap();
    let b = Project::create(&b_dir, "Beta").unwrap();
    assert_ne!(a.id(), b.id());
    let registry = Registry::open(&base.join("machine")).unwrap();
    a.save_state(&json!({"draft": "unfinished alpha", "events": ["alpha answer"]}))
        .unwrap();
    registry.activate(&a).unwrap();
    b.save_state(&json!({"draft": "unfinished beta"})).unwrap();
    registry.activate(&b).unwrap();
    // The old handle remains bound to Alpha after activating Beta.
    a.save_state(&json!({"draft": "alpha still writing"}))
        .unwrap();
    assert_eq!(registry.open_active().unwrap().unwrap().id(), b.id());
    registry.activate(&a).unwrap();

    let restarted = Registry::open(&base.join("machine")).unwrap();
    let active = restarted.open_active().unwrap().unwrap();
    assert_eq!(active.id(), a.id());
    assert_eq!(
        active.load_state().unwrap(),
        Some(json!({"draft": "alpha still writing"}))
    );
    assert_eq!(
        Project::open(&b_dir).unwrap().load_state().unwrap(),
        Some(json!({"draft": "unfinished beta"}))
    );
    assert_eq!(restarted.list().unwrap().len(), 2);
}

#[test]
fn interrupted_revision_and_commit_keep_previous_save_and_are_retained() {
    let base = scratch();
    let project = Project::create(&base, "Recovery").unwrap();
    project.save_state(&json!({"draft": "saved"})).unwrap();
    let state = project.archive_dir().join("state");
    let incomplete = state.join("00000000000000000002.json");
    let partial_commit = state.join("00000000000000000002.commit");
    fs::write(&incomplete, b"{\"v\":1,\"data\":").unwrap();
    fs::write(&partial_commit, b"{\"v\":1").unwrap();
    assert_eq!(
        Project::open(&base).unwrap().load_state().unwrap(),
        Some(json!({"draft": "saved"}))
    );
    project.save_state(&json!({"draft": "retry"})).unwrap();
    assert_eq!(fs::read(&incomplete).unwrap(), b"{\"v\":1,\"data\":");
    assert_eq!(fs::read(&partial_commit).unwrap(), b"{\"v\":1");
    assert!(state.join("00000000000000000003.commit").is_file());
    assert_eq!(
        project.load_state().unwrap(),
        Some(json!({"draft": "retry"}))
    );
}

#[test]
fn corrupt_committed_state_is_not_silently_replaced() {
    let base = scratch();
    let project = Project::create(&base, "Preserved").unwrap();
    project.save_state(&json!({"draft": "saved"})).unwrap();
    let file = project
        .archive_dir()
        .join("state/00000000000000000001.json");
    fs::write(&file, b"damaged").unwrap();
    assert!(project.load_state().is_err());
    assert!(Project::open(&base).is_err());
    assert!(project
        .save_state(&json!({"draft": "must not replace"}))
        .is_err());
    assert_eq!(fs::read(file).unwrap(), b"damaged");
    assert!(!project
        .archive_dir()
        .join("state/00000000000000000002.json")
        .exists());
}

#[test]
fn missing_or_replaced_marker_refuses_save_and_registry_switch() {
    let base = scratch();
    let a = Project::create(&workspace(&base, "a"), "A").unwrap();
    let b = Project::create(&workspace(&base, "b"), "B").unwrap();
    let registry = Registry::open(&base.join("machine")).unwrap();
    registry.activate(&a).unwrap();
    let original = fs::read(b.archive_dir().join(MARKER)).unwrap();
    // Synthetic fixture replacement represents another disk at this path.
    fs::write(
        b.archive_dir().join(MARKER),
        fs::read(a.archive_dir().join(MARKER)).unwrap(),
    )
    .unwrap();
    assert!(b.save_state(&json!("wrong project")).is_err());
    assert!(registry.activate(&b).is_err());
    assert_eq!(registry.active().unwrap().unwrap().id, *a.id());
    assert!(!b.archive_dir().join("state").exists());

    fs::write(b.archive_dir().join(MARKER), original).unwrap();
    let parked = base.join("b-parked");
    fs::rename(b.workspace(), &parked).unwrap();
    assert!(b.save_state(&json!("offline")).is_err());
    assert!(
        !b.workspace().exists(),
        "save must not create a fallback workspace"
    );
}

#[test]
fn setup_never_adopts_or_overwrites_unmarked_data() {
    let base = scratch();
    let chat = base.join(".devpulse/chat");
    fs::create_dir_all(&chat).unwrap();
    fs::write(chat.join("notes.txt"), "precious").unwrap();
    assert!(Project::create(&base, "No overwrite").is_err());
    assert_eq!(
        fs::read_to_string(chat.join("notes.txt")).unwrap(),
        "precious"
    );
    assert!(!chat.join(MARKER).exists());

    let machine = base.join("machine");
    fs::create_dir(&machine).unwrap();
    fs::write(machine.join("existing.txt"), "keep").unwrap();
    assert!(Registry::open(&machine).is_err());
    assert!(!machine.join("registry.json").exists());
}

#[test]
fn create_new_refuses_existing_marked_archive_and_preserves_its_state() {
    let base = scratch();
    let project = Project::create_new(&base, "Original").unwrap();
    project
        .save_state(&json!({"draft": "original work"}))
        .unwrap();
    let marker = fs::read(project.archive_dir().join(MARKER)).unwrap();
    assert!(Project::create_new(&base, "Recovery copy").is_err());
    assert_eq!(
        fs::read(project.archive_dir().join(MARKER)).unwrap(),
        marker
    );
    assert_eq!(
        Project::open(&base).unwrap().load_state().unwrap(),
        Some(json!({"draft": "original work"}))
    );
    // Ordinary create keeps its documented open-existing behavior.
    assert_eq!(
        Project::create(&base, "Ignored new label").unwrap().id(),
        project.id()
    );
}

#[test]
fn create_new_refuses_unmarked_content_and_only_one_concurrent_creator_wins() {
    let base = scratch();
    let populated = workspace(&base, "populated");
    let chat = populated.join(".devpulse/chat");
    fs::create_dir_all(&chat).unwrap();
    fs::write(chat.join("keep.txt"), "existing content").unwrap();
    assert!(Project::create_new(&populated, "Recovery copy").is_err());
    assert!(!chat.join(MARKER).exists());
    assert_eq!(
        fs::read_to_string(chat.join("keep.txt")).unwrap(),
        "existing content"
    );

    let destination = workspace(&base, "race");
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let joiners: Vec<_> = (0..2)
        .map(|_| {
            let destination = destination.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                Project::create_new(&destination, "Exclusive copy")
            })
        })
        .collect();
    let results: Vec<_> = joiners
        .into_iter()
        .map(|joiner| joiner.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(Project::open(&destination).unwrap().id(), winner.id());
}

#[test]
fn future_format_and_invalid_identity_are_rejected_without_writes() {
    let base = scratch();
    let project = Project::create(&base, "Future").unwrap();
    let marker = project.archive_dir().join(MARKER);
    let mut data: Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
    data["v"] = json!(999);
    let bytes = serde_json::to_vec(&data).unwrap();
    fs::write(&marker, &bytes).unwrap();
    assert!(Project::create(&base, "Cannot replace").is_err());
    assert_eq!(fs::read(&marker).unwrap(), bytes);
    for invalid in ["", "../x", "a/b", "a\\b", "CON", "lpt9", "x.y"] {
        assert!(ProjectId::parse(invalid).is_err(), "{invalid}");
    }
    assert!(Project::create(Path::new("relative"), "No").is_err());
    assert!(Project::create(&base.join("../elsewhere"), "No").is_err());
}

#[test]
fn unchanged_snapshots_are_deduplicated_and_busy_writer_refuses() {
    let base = scratch();
    let project = Project::create(&base, "Writer").unwrap();
    project.save_state(&json!({"draft": "same"})).unwrap();
    project.save_state(&json!({"draft": "same"})).unwrap();
    assert_eq!(
        fs::read_dir(project.archive_dir().join("state"))
            .unwrap()
            .count(),
        2
    );
    let lock = disk::lock(project.archive_dir()).unwrap();
    assert!(project.save_state(&json!({"draft": "busy"})).is_err());
    drop(lock);
    project.save_state(&json!({"draft": "now saved"})).unwrap();
    assert_eq!(
        project.load_state().unwrap(),
        Some(json!({"draft": "now saved"}))
    );
}

#[test]
fn registry_rejects_replacement_identity_and_preserves_saved_bindings() {
    let base = scratch();
    let project = Project::create(&workspace(&base, "workspace"), "Project").unwrap();
    let machine = base.join("machine");
    let registry = Registry::open(&machine).unwrap();
    registry.activate(&project).unwrap();
    let saved = fs::read(machine.join("bindings/00000000000000000001.json")).unwrap();
    let other_path = base.join("other-machine");
    Registry::open(&other_path).unwrap();
    fs::write(
        machine.join("registry.json"),
        fs::read(other_path.join("registry.json")).unwrap(),
    )
    .unwrap();
    assert!(registry.activate(&project).is_err());
    assert_eq!(
        fs::read(machine.join("bindings/00000000000000000001.json")).unwrap(),
        saved
    );
}

#[test]
fn binding_refuses_same_path_with_a_different_project() {
    let base = scratch();
    let project = Project::create(&workspace(&base, "workspace"), "Original").unwrap();
    let registry = Registry::open(&base.join("machine")).unwrap();
    registry.activate(&project).unwrap();
    let replacement = Project::create(&workspace(&base, "replacement"), "Replacement").unwrap();
    fs::write(
        project.archive_dir().join(MARKER),
        fs::read(replacement.archive_dir().join(MARKER)).unwrap(),
    )
    .unwrap();
    assert!(registry.open_active().is_err());
    assert_eq!(registry.active().unwrap().unwrap().id, *project.id());
}

#[test]
fn autosave_deltas_store_changed_text_and_array_entries_only() {
    let base = scratch();
    let project = Project::create(&base, "Deltas").unwrap();
    let long = "historic message ".repeat(10_000);
    let first =
        json!({"events": [{"text": long}, {"text": "Hello 🦀"}], "draft": "abc", "removed": true});
    project.save_state(&first).unwrap();
    let second = json!({"events": [{"text": long}, {"text": "Hello 🦋 there"}, {"text": "new"}], "draft": "abcd"});
    project.save_state(&second).unwrap();
    let delta = fs::read(
        project
            .archive_dir()
            .join("state/00000000000000000002.json"),
    )
    .unwrap();
    assert!(
        delta.len() < 1024,
        "a small edit must not duplicate history"
    );
    assert_eq!(
        Project::open(&base).unwrap().load_state().unwrap(),
        Some(second)
    );
    let third = json!({"events": [{"text": long}], "draft": ""});
    project.save_state(&third).unwrap();
    assert_eq!(project.load_state().unwrap(), Some(third));
}

#[test]
fn registry_keeps_offline_binding_and_relocates_only_explicitly() {
    let base = scratch();
    let original = workspace(&base, "original");
    let project = Project::create(&original, "Portable").unwrap();
    project.save_state(&json!({"draft": "carry me"})).unwrap();
    let registry = Registry::open(&base.join("machine")).unwrap();
    registry.activate(&project).unwrap();
    let relocated = base.join("relocated");
    fs::rename(&original, &relocated).unwrap();
    assert!(registry.open_active().is_err());
    assert_eq!(registry.list().unwrap().len(), 1);
    let same = Project::open(&relocated).unwrap();
    assert_eq!(same.id(), project.id());
    registry.activate(&same).unwrap();
    assert_eq!(
        registry
            .open_active()
            .unwrap()
            .unwrap()
            .load_state()
            .unwrap(),
        Some(json!({"draft": "carry me"}))
    );
    let marker: Value =
        serde_json::from_slice(&fs::read(same.archive_dir().join(MARKER)).unwrap()).unwrap();
    assert!(
        marker.get("workspace").is_none(),
        "portable metadata must not pin a host path"
    );
}

#[test]
fn cloned_live_project_identity_does_not_replace_registry_binding() {
    let base = scratch();
    let original = Project::create(&workspace(&base, "original"), "Original").unwrap();
    let cloned_dir = workspace(&base, "cloned");
    fs::create_dir_all(cloned_dir.join(".devpulse/chat")).unwrap();
    fs::copy(
        original.archive_dir().join(MARKER),
        cloned_dir.join(".devpulse/chat/project.json"),
    )
    .unwrap();
    let clone = Project::open(&cloned_dir).unwrap();
    let registry = Registry::open(&base.join("machine")).unwrap();
    registry.activate(&original).unwrap();
    assert!(registry.activate(&clone).is_err());
    assert_eq!(
        registry.active().unwrap().unwrap().workspace,
        original.workspace()
    );
}

#[test]
fn missing_marker_and_future_commit_preserve_existing_state() {
    let base = scratch();
    let project = Project::create(&base, "Identity").unwrap();
    project.save_state(&json!({"draft": "saved"})).unwrap();
    let marker = project.archive_dir().join(MARKER);
    let parked = project.archive_dir().join("project.parked.json");
    fs::rename(&marker, &parked).unwrap();
    assert!(project.save_state(&json!({"draft": "blocked"})).is_err());
    assert!(Project::create(&base, "Cannot reinitialize").is_err());
    assert!(!marker.exists());
    fs::rename(&parked, &marker).unwrap();
    let commit = project
        .archive_dir()
        .join("state/00000000000000000001.commit");
    let mut value: Value = serde_json::from_slice(&fs::read(&commit).unwrap()).unwrap();
    value["v"] = json!(999);
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    fs::write(&commit, &bytes).unwrap();
    assert!(project.load_state().is_err());
    assert!(project.save_state(&json!({"draft": "blocked"})).is_err());
    assert_eq!(fs::read(commit).unwrap(), bytes);
}

#[test]
fn corrupt_registry_state_does_not_reset_selection_or_allow_activation() {
    let base = scratch();
    let project = Project::create(&workspace(&base, "project"), "Project").unwrap();
    let machine = base.join("machine");
    let registry = Registry::open(&machine).unwrap();
    registry.activate(&project).unwrap();
    let state = machine.join("bindings/00000000000000000001.json");
    fs::write(&state, b"corrupt").unwrap();
    assert!(Registry::open(&machine).is_err());
    assert!(registry.activate(&project).is_err());
    assert_eq!(fs::read(state).unwrap(), b"corrupt");
}

#[test]
fn independent_project_handles_refuse_stale_saves_but_clones_share_revision() {
    let base = scratch();
    let first = Project::create(&base, "Concurrent").unwrap();
    let stale = Project::open(&base).unwrap();
    first.save_state(&json!({"draft": "first"})).unwrap();
    let edits = json!({"draft": "second instance unsaved edits"});
    let error = stale.save_state(&edits).unwrap_err();
    assert!(error.contains("another app instance"));
    assert_eq!(edits["draft"], "second instance unsaved edits");
    assert_eq!(first.load_state().unwrap(), Some(json!({"draft": "first"})));
    first
        .clone()
        .save_state(&json!({"draft": "clone"}))
        .unwrap();
    first
        .save_state(&json!({"draft": "shared revision"}))
        .unwrap();
    assert_eq!(
        stale.load_state().unwrap(),
        Some(json!({"draft": "shared revision"}))
    );
    stale.save_state(&edits).unwrap();
    assert!(first.save_state(&json!({"draft": "now stale"})).is_err());
}

#[test]
fn checkpoint_bounds_long_history_and_preserves_superseded_revisions() {
    let base = scratch();
    let project = Project::create(&base, "Long history").unwrap();
    let history = "earlier conversation ".repeat(13_000);
    let start = std::time::Instant::now();
    for revision in 1..=130 {
        project
            .save_state(&json!({"history": history, "draft": format!("revision {revision}")}))
            .unwrap();
    }
    let save_duration = start.elapsed();
    let state = project.archive_dir().join("state");
    for revision in [1, 65, 129] {
        let commit: Value = serde_json::from_slice(
            &fs::read(state.join(format!("{revision:020}.commit"))).unwrap(),
        )
        .unwrap();
        assert_eq!(commit["checkpoint"], true);
    }
    let disk_bytes: u64 = fs::read_dir(&state)
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    assert!(
        disk_bytes < 1_000_000,
        "checkpoints plus small deltas must not copy history on every edit"
    );
    assert_eq!(fs::read_dir(&state).unwrap().count(), 260);
    let start = std::time::Instant::now();
    assert_eq!(
        Project::open(&base).unwrap().load_state().unwrap().unwrap()["draft"],
        "revision 130"
    );
    eprintln!("130 saves of 273KiB history: {save_duration:?}; reopen+load: {:?}; disk bytes: {disk_bytes}", start.elapsed());
    let superseded = state.join("00000000000000000001.json");
    fs::write(&superseded, "old corruption").unwrap();
    assert_eq!(
        project.load_state().unwrap().unwrap()["draft"],
        "revision 130"
    );
    assert_eq!(fs::read_to_string(&superseded).unwrap(), "old corruption");
    // A complete newer checkpoint is authoritative: corruption cannot fall
    // back to checkpoint65 and silently discard the latest saved work.
    fs::write(state.join("00000000000000000129.json"), "new corruption").unwrap();
    assert!(project.load_state().is_err());
    assert!(project
        .save_state(&json!({"draft": "do not replace"}))
        .is_err());
}

#[test]
fn interrupted_checkpoint_retries_without_clobbering_partial_files() {
    let base = scratch();
    let project = Project::create(&base, "Checkpoint recovery").unwrap();
    for revision in 1..=64 {
        project.save_state(&json!({"draft": revision})).unwrap();
    }
    let state = project.archive_dir().join("state");
    let partial = state.join("00000000000000000065.json");
    fs::write(&partial, "partial checkpoint").unwrap();
    fs::write(state.join("00000000000000000065.commit"), "partial commit").unwrap();
    assert_eq!(project.load_state().unwrap(), Some(json!({"draft": 64})));
    project.save_state(&json!({"draft": 66})).unwrap();
    let commit: Value =
        serde_json::from_slice(&fs::read(state.join("00000000000000000066.commit")).unwrap())
            .unwrap();
    assert_eq!(commit["checkpoint"], true);
    assert_eq!(fs::read_to_string(partial).unwrap(), "partial checkpoint");
    assert_eq!(
        Project::open(&base).unwrap().load_state().unwrap(),
        Some(json!({"draft": 66}))
    );
}

#[cfg(unix)]
#[test]
fn symlinked_project_or_state_is_not_followed() {
    let base = scratch();
    let real = workspace(&base, "real");
    let linked = base.join("linked");
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    assert!(Project::create(&linked, "No link").is_err());
    assert!(!real.join(".devpulse").exists());
    let project = Project::create(&real, "Real").unwrap();
    let outside = workspace(&base, "outside");
    std::os::unix::fs::symlink(&outside, project.archive_dir().join("state")).unwrap();
    assert!(project.save_state(&json!("no escape")).is_err());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

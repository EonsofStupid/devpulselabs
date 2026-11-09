use super::*;

fn fixture() -> (PathBuf, Registry, Project) {
    let base = std::env::temp_dir()
        .join("dpl-profile-candidates")
        .join(ProjectId::mint().unwrap().as_str());
    let workspace = base.join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let registry = Registry::open(&base.join("machine")).unwrap();
    let project = Project::create_new(&workspace, "Candidate project").unwrap();
    (base, registry, project)
}

#[test]
fn shared_leases_block_exclusive_setup_and_drop_releases_without_pid_recovery() {
    let (base, _, _) = fixture();
    let pool = base.join("lease-fixture");
    fs::create_dir(&pool).unwrap();
    disk::create(&pool.join(LEASE), b"").unwrap();
    let first = lock_clients(&pool, true).unwrap();
    let second = lock_clients(&pool, true).unwrap();
    assert!(lock_clients(&pool, false).is_err());
    drop(first);
    assert!(lock_clients(&pool, false).is_err());
    drop(second);
    let exclusive = lock_clients(&pool, false).unwrap();
    assert!(lock_clients(&pool, true).is_err());
    drop(exclusive);
    assert!(lock_clients(&pool, true).is_ok());
    assert!(
        pool.join(LEASE).is_file(),
        "lock files are retained, never deleted as stale PIDs"
    );
}

#[test]
fn unowned_profile_and_native_storage_are_never_adopted() {
    let (_, registry, project) = fixture();
    let profiles = registry.base().join("profiles");
    fs::create_dir(&profiles).unwrap();
    fs::write(profiles.join("keep.txt"), "existing").unwrap();
    assert!(CodexProfileCandidate::create_new(&registry, &project).is_err());
    assert_eq!(
        fs::read_to_string(profiles.join("keep.txt")).unwrap(),
        "existing"
    );
    assert!(!profiles.join(OWNER).exists());

    let (_, registry, project) = fixture();
    let native = native_root(&project);
    fs::create_dir_all(native.join("sessions")).unwrap();
    fs::write(
        native.join("sessions/rollout.jsonl"),
        "existing native content",
    )
    .unwrap();
    assert!(CodexProfileCandidate::create_new(&registry, &project).is_err());
    assert!(!native.join(OWNER).exists());
    assert_eq!(
        fs::read_to_string(native.join("sessions/rollout.jsonl")).unwrap(),
        "existing native content"
    );
}

#[test]
fn a_machine_profile_cannot_be_created_inside_the_portable_workspace() {
    let (_, _, project) = fixture();
    let registry = Registry::open(&project.workspace().join("machine")).unwrap();
    assert!(CodexProfileCandidate::create_new(&registry, &project)
        .unwrap_err()
        .contains("outside the portable workspace"));
    assert!(!registry.base().join("profiles").exists());
}

#[test]
fn link_creation_refuses_existing_directories_and_files() {
    let (base, _, _) = fixture();
    let target = base.join("target");
    fs::create_dir(&target).unwrap();
    let occupied = base.join("occupied");
    fs::create_dir(&occupied).unwrap();
    assert!(create_link(&target, &occupied).is_err());
    assert!(occupied.is_dir());
    let occupied_file = base.join("occupied-file");
    fs::write(&occupied_file, "keep").unwrap();
    assert!(create_link(&target, &occupied_file).is_err());
    assert_eq!(fs::read_to_string(occupied_file).unwrap(), "keep");
}

/// A missing Windows symlink privilege is itself a tested, fail-closed result;
/// it must not be confused with native provider compatibility qualification.
fn candidate_or_link_refusal(
    registry: &Registry,
    project: &Project,
) -> Option<CodexProfileCandidate> {
    match CodexProfileCandidate::create_new(registry, project) {
        Ok(candidate) => Some(candidate),
        Err(error) if cfg!(windows) && error.contains("check platform link privileges") => {
            let pool = registry.base().join("profiles").join(project.id().as_str());
            let pending = fs::read_dir(&pool)
                .unwrap()
                .filter_map(Result::ok)
                .find(|entry| entry.path().join(PENDING).is_file())
                .unwrap()
                .path();
            assert!(!pending.join(PROFILE).exists());
            assert!(pending.join(PENDING).is_file());
            assert!(native_root(project).join("sessions").is_dir());
            eprintln!(
                "Windows candidate links unavailable; verified retained incomplete setup: {error}"
            );
            None
        }
        Err(error) => panic!("candidate setup failed: {error}"),
    }
}

#[test]
fn candidate_generations_share_project_native_files_but_never_retarget_old_links() {
    let (_, registry, project) = fixture();
    let Some(first) = candidate_or_link_refusal(&registry, &project) else {
        return;
    };
    let lease = first.lease().unwrap();
    let another_client = first.lease().unwrap();
    assert!(CodexProfileCandidate::create_new(&registry, &project).is_err());
    fs::write(
        first.home().join("sessions/synthetic.jsonl"),
        "fixture conversation",
    )
    .unwrap();
    drop(lease);
    drop(another_client);
    let second = CodexProfileCandidate::create_new(&registry, &project).unwrap();
    assert_ne!(first.generation(), second.generation());
    assert_ne!(first.home(), second.home());
    assert_eq!(
        fs::read_to_string(second.home().join("sessions/synthetic.jsonl")).unwrap(),
        "fixture conversation"
    );
    assert!(first.home().is_dir());
    assert_eq!(
        fs::read_link(first.home().join("sessions")).unwrap(),
        native_root(&project).join("sessions")
    );
    let marker: serde_json::Value =
        disk::json(&first.directory.join(PROFILE), MAX_MARKER_BYTES).unwrap();
    assert!(marker.get("qualified").is_none());
    assert!(marker.get("supported").is_none());
    assert!(
        CodexProfileCandidate::open(&registry, &project, first.generation())
            .unwrap()
            .lease()
            .is_ok()
    );
}

#[test]
fn replacement_marker_wrong_target_and_offline_project_refuse_launch_lease() {
    let (base, registry, project) = fixture();
    let Some(candidate) = candidate_or_link_refusal(&registry, &project) else {
        return;
    };
    let original = fs::read(candidate.directory.join(PROFILE)).unwrap();
    fs::write(candidate.directory.join(PROFILE), b"{}").unwrap();
    assert!(candidate.lease().is_err());
    fs::write(candidate.directory.join(PROFILE), original).unwrap();
    let link = candidate.home().join("sessions");
    fs::rename(&link, candidate.home().join("sessions-parked")).unwrap();
    let wrong = base.join("wrong");
    fs::create_dir(&wrong).unwrap();
    create_link(&wrong, &link).unwrap();
    assert!(candidate.lease().is_err());
    assert_eq!(fs::read_dir(&wrong).unwrap().count(), 0);
    fs::rename(project.workspace(), base.join("parked-workspace")).unwrap();
    assert!(candidate.verify().is_err());
}

#[test]
fn relocation_waits_for_all_generations_and_preserves_previous_home() {
    let (base, registry, project) = fixture();
    let Some(old) = candidate_or_link_refusal(&registry, &project) else {
        return;
    };
    let lease = old.lease().unwrap();
    let relocated = base.join("relocated");
    fs::rename(project.workspace(), &relocated).unwrap();
    let moved = Project::open(&relocated).unwrap();
    assert!(lease.verify().is_err());
    assert!(CodexProfileCandidate::create_new(&registry, &moved).is_err());
    drop(lease);
    let new = CodexProfileCandidate::create_new(&registry, &moved).unwrap();
    assert!(new.verify().is_ok());
    assert!(old.home().is_dir());
    assert!(old.verify().is_err());
    assert_eq!(
        fs::read_link(old.home().join("sessions")).unwrap(),
        native_root(&project).join("sessions")
    );
}

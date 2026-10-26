use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn scratch() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "dpl-roots-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn program(base: &Path, name: &str) -> PathBuf {
    let program = base.join(name);
    fs::create_dir_all(&program).unwrap();
    program
}

fn imported(program: &Path) -> Layout {
    setup_at(
        program,
        &program.join("library"),
        Mode::Imported,
        "My library",
    )
    .unwrap()
}

#[test]
fn no_config_means_choice_and_creates_nothing() {
    let root = scratch();
    // Parent and current-directory content are deliberately present decoys.
    fs::create_dir(root.join("content")).unwrap();
    let exe = program(&root, "program");
    let roots = Roots::open(&exe, None);
    assert_eq!(roots.state(), &State::Unconfigured);
    assert!(roots.layout().is_err());
    assert_eq!(fs::read_dir(&exe).unwrap().count(), 0);
}

#[test]
fn imported_round_trip_has_the_final_layout() {
    let root = scratch();
    let layout = imported(&root);
    let marker = read_marker(&layout.library).unwrap();
    assert_eq!(marker.v, 1);
    assert_eq!(marker.label, "My library");
    assert!(marker.id.as_str().starts_with("library-"));
    let config = disk::read_config(&root).unwrap();
    assert_eq!(config.id, marker.id);
    assert_eq!(config.mode, Mode::Imported);
    assert_eq!(config.last_path, None);
    assert_eq!(layout.content, root.join("library/accounts/me/content"));
    assert_eq!(layout.temp, root.join("library/common/temp"));
    assert_eq!(layout.aidata, root.join("library/accounts/me/.aidata"));
    let mut roots = Roots::open(&root, None);
    assert_eq!(roots.layout().unwrap(), layout);
    assert_eq!(roots.ensure_rings().unwrap(), layout);
    assert_eq!(roots.ensure_rings().unwrap(), layout);
    assert!(fs::read_to_string(layout.userdata.join("README.txt"))
        .unwrap()
        .contains("safe to delete"));
}

#[test]
fn imported_config_moves_with_its_program() {
    let root = scratch();
    let first = program(&root, "first");
    let layout = imported(&first);
    let second = program(&root, "second");
    imported(&second);
    fs::copy(first.join(CONFIG_FILE), second.join(CONFIG_FILE)).unwrap();
    fs::copy(
        layout.library.join(MARKER_FILE),
        second.join("library").join(MARKER_FILE),
    )
    .unwrap();
    assert_eq!(
        Roots::open(&second, None).layout().unwrap().library,
        second.join("library")
    );
}

#[test]
fn staging_is_read_only_for_library_and_never_clobbers_config() {
    let base = scratch();
    let source = program(&base, "source");
    let layout = imported(&source);
    let target = program(&base, "target");
    let before = fs::read(layout.library.join(MARKER_FILE)).unwrap();
    stage_roots(&target, &layout.library).unwrap();
    let config = fs::read(target.join(CONFIG_FILE)).unwrap();
    assert_eq!(Roots::open(&target, None).layout().unwrap(), layout);
    assert_eq!(disk::read_config(&target).unwrap().mode, Mode::Linked);
    assert!(stage_roots(&target, &layout.library).is_err());
    assert_eq!(fs::read(target.join(CONFIG_FILE)).unwrap(), config);
    assert_eq!(fs::read(layout.library.join(MARKER_FILE)).unwrap(), before);
}

#[test]
fn legacy_unmarked_content_is_not_adopted_or_modified() {
    let base = scratch();
    let exe = program(&base, "program");
    let legacy = program(&base, "legacy");
    fs::write(legacy.join("my-video.mp4"), b"precious").unwrap();
    assert!(setup_at(&exe, &legacy, Mode::Linked, "Drive").is_err());
    assert!(stage_roots(&exe, &legacy).is_err());
    assert_eq!(fs::read_dir(&legacy).unwrap().count(), 1);
    assert_eq!(fs::read(legacy.join("my-video.mp4")).unwrap(), b"precious");
    assert!(!exe.join(CONFIG_FILE).exists());
}

#[test]
fn unavailable_linked_library_never_creates_a_replacement() {
    let base = scratch();
    let exe = program(&base, "program");
    let source = program(&base, "source");
    let source_layout = imported(&source);
    let marker = read_marker(&source_layout.library).unwrap();
    let missing = base.join("unplugged/library");
    disk::create_config(
        &exe,
        &Config {
            v: 1,
            mode: Mode::Linked,
            id: marker.id,
            label: marker.label,
            account: AccountId::default(),
            last_path: Some(missing.clone()),
        },
    )
    .unwrap();
    let mut roots = Roots::open(&exe, None);
    assert!(matches!(roots.state(), State::Disconnected { .. }));
    assert!(roots.ensure_rings().is_err());
    assert!(!missing.exists());
    assert!(!exe.join("library").exists());
    assert!(!exe.join("content").exists());
}

#[test]
fn wrong_identity_at_remembered_path_is_refused() {
    let base = scratch();
    let exe = program(&base, "program");
    let layout = imported(&exe);
    let mut marker = read_marker(&layout.library).unwrap();
    marker.id = LibraryId::parse("another-library").unwrap();
    fs::write(
        layout.library.join(MARKER_FILE),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();
    let roots = Roots::open(&exe, None);
    assert!(roots.layout().unwrap_err().contains("different library"));
}

#[test]
fn marker_loss_transitions_handle_to_disconnected_before_creation() {
    let base = scratch();
    let layout = imported(&base);
    let mut roots = Roots::open(&base, None);
    fs::write(layout.library.join(MARKER_FILE), b"invalid marker").unwrap();
    assert!(roots.ensure_rings().is_err());
    assert!(matches!(roots.state(), State::Disconnected { .. }));
    assert!(roots.layout().is_err());
}

#[test]
fn exact_candidates_find_a_different_drive_and_reject_duplicate_identities() {
    let base = scratch();
    let exe = program(&base, "program");
    let a = program(&base, "drive-a");
    let b = program(&base, "drive-b");
    let c = program(&base, "drive-c");
    let layout_b = imported(&b);
    let layout_c = imported(&c);
    let marker = read_marker(&layout_b.library).unwrap();
    fs::copy(
        layout_b.library.join(MARKER_FILE),
        layout_c.library.join(MARKER_FILE),
    )
    .unwrap();
    disk::create_config(
        &exe,
        &Config {
            v: 1,
            mode: Mode::Linked,
            id: marker.id,
            label: marker.label,
            account: AccountId::default(),
            last_path: Some(a.join("library")),
        },
    )
    .unwrap();
    let mut roots = Roots::open(&exe, None);
    let paths = candidates_on_roots(&a.join("library"), &a, std::slice::from_ref(&b));
    assert_eq!(
        roots.refresh_with_candidates(&paths).unwrap().library,
        layout_b.library
    );
    let paths = candidates_on_roots(&a.join("library"), &a, &[b, c]);
    assert!(roots
        .refresh_with_candidates(&paths)
        .unwrap_err()
        .contains("More than one"));
    assert!(matches!(roots.state(), State::Disconnected { .. }));
    assert!(!a.join("library").exists());
}

#[test]
fn duplicate_candidate_paths_do_not_create_false_ambiguity() {
    let base = scratch();
    let exe = program(&base, "program");
    let source = program(&base, "source");
    let layout = imported(&source);
    stage_roots(&exe, &layout.library).unwrap();
    let mut roots = Roots::open(&exe, None);
    assert!(roots
        .refresh_with_candidates(&[layout.library.clone(), layout.library.clone()])
        .is_ok());
}

#[test]
fn corrupt_and_future_config_never_create_data() {
    for text in [
        "not toml = [",
        "v = 99\nmode = 'imported'\nid = 'library-test'\nlabel = 'Future'\n",
    ] {
        let base = scratch();
        fs::write(base.join(CONFIG_FILE), text).unwrap();
        let roots = Roots::open(&base, None);
        assert!(matches!(roots.state(), State::Disconnected { .. }));
        assert_eq!(fs::read_dir(&base).unwrap().count(), 1);
    }
}

#[test]
fn future_marker_and_traversal_account_are_rejected() {
    let base = scratch();
    let layout = imported(&base);
    let mut marker = read_marker(&layout.library).unwrap();
    marker.v = 2;
    fs::write(
        layout.library.join(MARKER_FILE),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();
    assert!(Roots::open(&base, None)
        .layout()
        .unwrap_err()
        .contains("schema 2"));
    for id in ["", "../escape", "a/b", "a\\b", "..", "CON", "nul", "x:"] {
        assert!(AccountId::parse(id).is_err(), "{id}");
        assert!(serde_json::from_str::<AccountId>(&format!("{id:?}")).is_err());
    }
}

#[test]
fn cli_overrides_env_and_does_not_touch_config() {
    let base = scratch();
    let args = vec![
        OsString::from("--content-root"),
        OsString::from("scratch/content"),
    ];
    let overrides = parse_overrides(
        &args,
        Some("wrong".into()),
        Some("scratch/work".into()),
        Some("scratch/state".into()),
        &base,
    )
    .unwrap()
    .unwrap();
    let roots = Roots::open(&base, Some(overrides));
    let layout = roots.layout().unwrap();
    assert_eq!(layout.content, base.join("scratch/content"));
    assert_eq!(layout.assets, base.join("scratch/work"));
    assert_eq!(layout.userdata, base.join("scratch/state"));
    assert_eq!(layout.temp, base.join("scratch/common/temp"));
    assert_eq!(fs::read_dir(&base).unwrap().count(), 0);
}

#[test]
fn malformed_cli_never_falls_back_to_environment() {
    let base = scratch();
    for args in [
        vec!["--content-root"],
        vec!["--content-root="],
        vec!["--content-root", "--screenshot"],
        vec!["--content-root=a", "--content-root=b"],
    ] {
        assert!(parse_overrides(
            &args.iter().map(OsString::from).collect::<Vec<_>>(),
            Some("env".into()),
            None,
            None,
            &base
        )
        .is_err());
    }
    assert!(parse_overrides(&[], None, Some("assets".into()), None, &base).is_err());
    assert!(parse_overrides(&[], Some("".into()), None, None, &base).is_err());
}

#[test]
fn existing_configuration_prevents_setup_writes_anywhere() {
    let base = scratch();
    imported(&base);
    let elsewhere = base.join("another-library");
    assert!(setup_at(&base, &elsewhere, Mode::Linked, "Another").is_err());
    assert!(!elsewhere.exists());
}

#[test]
fn asset_resolution_uses_the_selected_library_and_program_only() {
    let base = scratch();
    let exe = program(&base, "program");
    let layout = imported(&exe);
    fs::create_dir(layout.content.join("inbox")).unwrap();
    fs::write(layout.content.join("inbox/clip.mp4"), b"library").unwrap();
    fs::write(layout.assets.join("working.mp4"), b"working").unwrap();
    fs::write(exe.join("shipped.mp4"), b"shipped").unwrap();
    fs::write(base.join("parent-only.mp4"), b"wrong").unwrap();
    let roots = Roots::open(&exe, None);
    assert_eq!(
        roots.locate_asset("inbox/clip.mp4"),
        Some(layout.content.join("inbox/clip.mp4"))
    );
    assert_eq!(
        roots.locate_asset("assets/working.mp4"),
        Some(layout.assets.join("working.mp4"))
    );
    assert_eq!(
        roots.locate_asset("shipped.mp4"),
        Some(exe.join("shipped.mp4"))
    );
    assert_eq!(roots.locate_asset("parent-only.mp4"), None);
    assert_eq!(roots.locate_asset("../parent-only.mp4"), None);
}

#[test]
fn relative_config_path_is_executable_relative() {
    let base = scratch();
    let exe = program(&base, "program");
    let source = program(&base, "source");
    let layout = imported(&source);
    let marker = read_marker(&layout.library).unwrap();
    disk::create_config(
        &exe,
        &Config {
            v: 1,
            mode: Mode::Linked,
            id: marker.id,
            label: marker.label,
            account: AccountId::default(),
            last_path: Some("../source/library".into()),
        },
    )
    .unwrap();
    assert_eq!(
        Roots::open(&exe, None).layout().unwrap().library,
        layout.library
    );
}

#[test]
fn a_missing_account_does_not_become_an_empty_library() {
    let base = scratch();
    let layout = imported(&base);
    let mut config = disk::read_config(&base).unwrap();
    config.account = AccountId::parse("missing").unwrap();
    fs::write(base.join(CONFIG_FILE), toml::to_string(&config).unwrap()).unwrap();
    let mut roots = Roots::open(&base, None);
    assert!(roots.ensure_rings().is_err());
    assert!(!layout.library.join("accounts/missing").exists());
}

#[cfg(unix)]
#[test]
fn symlinked_account_state_is_refused() {
    let base = scratch();
    let layout = imported(&base);
    let outside = program(&base, "outside");
    let alias = layout.library.join("accounts/alias");
    std::os::unix::fs::symlink(&outside, &alias).unwrap();
    let mut config = disk::read_config(&base).unwrap();
    config.account = AccountId::parse("alias").unwrap();
    fs::write(base.join(CONFIG_FILE), toml::to_string(&config).unwrap()).unwrap();
    assert!(Roots::open(&base, None).layout().is_err());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

#[test]
fn setup_cannot_turn_the_configuration_path_into_a_library_folder() {
    let base = scratch();
    for selected in [
        base.join(CONFIG_FILE),
        base.join(CONFIG_FILE).join("library"),
    ] {
        assert!(setup_at(&base, &selected, Mode::Linked, "Wrong place").is_err());
        assert_eq!(fs::read_dir(&base).unwrap().count(), 0);
    }
    #[cfg(windows)]
    {
        let selected = base.join("ROOTS.TOML/library");
        assert!(setup_at(&base, &selected, Mode::Linked, "Wrong place").is_err());
        assert_eq!(fs::read_dir(&base).unwrap().count(), 0);
    }
}

#[test]
fn existing_files_in_ring_paths_prevent_publishing_configuration() {
    for bad_ring in [
        "accounts/me/content",
        "accounts/me/.userdata/profiles",
        "common/temp",
        "common",
    ] {
        let base = scratch();
        let source = program(&base, "source");
        let source_layout = imported(&source);
        let library = program(&base, "library");
        fs::create_dir_all(library.join("accounts/me")).unwrap();
        fs::copy(
            source_layout.library.join(MARKER_FILE),
            library.join(MARKER_FILE),
        )
        .unwrap();
        fs::copy(
            source_layout.library.join("accounts/me/account.json"),
            library.join("accounts/me/account.json"),
        )
        .unwrap();
        let invalid = library.join(bad_ring);
        fs::create_dir_all(invalid.parent().unwrap()).unwrap();
        fs::write(&invalid, b"existing material").unwrap();
        let staged = program(&base, "staged");
        assert!(stage_roots(&staged, &library).is_err(), "{bad_ring}");
        assert!(!staged.join(CONFIG_FILE).exists());
        let selected = program(&base, "selected");
        assert!(
            setup_at(&selected, &library, Mode::Linked, "Existing library").is_err(),
            "{bad_ring}"
        );
        assert!(!selected.join(CONFIG_FILE).exists());
        assert_eq!(fs::read(&invalid).unwrap(), b"existing material");
    }
}

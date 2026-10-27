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

/// Every file under `dir`, as (relative path, bytes): a tree's full content.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(base: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(base, &path, out);
            } else {
                let rel = path.strip_prefix(base).unwrap().to_path_buf();
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    visit(dir, dir, &mut out);
    out.sort();
    out
}

/// Today's three rings with a little of everything, including an empty folder.
fn legacy(base: &Path, name: &str) -> PathBuf {
    let source = program(base, name);
    fs::create_dir_all(source.join("content/library/slides")).unwrap();
    fs::write(source.join("content/library/slides/one.json"), b"{\"v\":3}").unwrap();
    fs::write(
        source.join("content/composer-state.json"),
        vec![7u8; 200_000],
    )
    .unwrap();
    fs::create_dir_all(source.join("content/inbox")).unwrap();
    fs::create_dir_all(source.join("assets/captures")).unwrap();
    fs::write(source.join("assets/captures/shot.png"), b"png").unwrap();
    fs::create_dir_all(source.join(".userdata/profiles")).unwrap();
    fs::write(source.join(".userdata/index.db"), b"index").unwrap();
    source
}

#[test]
fn a_build_attunes_to_its_project_and_nothing_else_does() {
    let base = scratch();
    let project = program(&base, "project");
    fs::write(project.join("Cargo.toml"), b"[workspace]\n").unwrap();
    let release = program(&project, "target/release");
    assert_eq!(project_of_build(&release), Some(project.clone()));
    assert_eq!(
        project_of_build(&program(&project, "target/debug")),
        Some(project.clone())
    );
    // Test binaries, other folder names and a missing manifest stay put.
    assert_eq!(
        project_of_build(&program(&project, "target/release/deps")),
        None
    );
    assert_eq!(project_of_build(&program(&project, "build/release")), None);
    let bare = program(&base, "bare");
    assert_eq!(project_of_build(&program(&bare, "target/release")), None);
    assert_eq!(project_of_build(&program(&base, "portable")), None);
}

#[test]
fn the_plan_counts_each_ring_and_writes_nothing() {
    let base = scratch();
    let source = legacy(&base, "source");
    let before = snapshot(&source);
    let library = base.join("library");
    let plan = plan_move(&source, &library).unwrap();
    let rings: Vec<_> = plan
        .items
        .iter()
        .map(|item| (item.ring, item.files))
        .collect();
    assert_eq!(rings, [("content", 2), ("assets", 1), (".userdata", 1)]);
    assert_eq!(plan.files(), 4);
    assert_eq!(plan.bytes(), 7 + 200_000 + 3 + 5);
    assert_eq!(plan.items[0].to, library.join("accounts/me/content"));
    assert_eq!(plan.items[2].to, library.join("accounts/me/.userdata"));
    assert!(!library.exists());
    assert_eq!(snapshot(&source), before);
}

#[test]
fn the_plan_refuses_a_busy_or_nested_library_and_an_empty_source() {
    let base = scratch();
    let source = legacy(&base, "source");
    let busy = program(&base, "busy");
    fs::write(busy.join("notes.txt"), b"mine").unwrap();
    assert!(plan_move(&source, &busy)
        .unwrap_err()
        .contains("already holds files"));
    assert!(plan_move(&source, &source.join("content/library"))
        .unwrap_err()
        .contains("can't go inside"));
    assert!(plan_move(&source, &source)
        .unwrap_err()
        .contains("inside the folder you chose"));
    assert!(plan_move(&source, Path::new("relative")).is_err());
    let empty = program(&base, "empty");
    assert!(plan_move(&empty, &base.join("lib"))
        .unwrap_err()
        .contains("no content, assets or .userdata"));
    assert_eq!(fs::read(busy.join("notes.txt")).unwrap(), b"mine");
}

#[test]
fn bringing_in_copies_exactly_publishes_last_and_leaves_the_source() {
    let base = scratch();
    let source = legacy(&base, "source");
    let before = snapshot(&source);
    let exe = program(&base, "program");
    let library = base.join("library");
    let progress = MoveProgress::default();
    let layout = migrate::move_in(
        &exe,
        &source,
        &library,
        "My library",
        &progress,
        Publish::New,
    )
    .unwrap();
    assert_eq!(snapshot(&source), before, "the source is only read");
    assert_eq!(snapshot(&layout.content), snapshot(&source.join("content")));
    assert_eq!(snapshot(&layout.assets), snapshot(&source.join("assets")));
    assert_eq!(
        snapshot(&layout.userdata),
        snapshot(&source.join(".userdata"))
    );
    assert!(
        layout.content.join("inbox").is_dir(),
        "empty folders come too"
    );
    assert_eq!(progress.copied_files.load(Ordering::Relaxed), 4);
    assert_eq!(progress.checked_files.load(Ordering::Relaxed), 4);
    let marker = read_marker(&library).unwrap();
    let config = disk::read_config(&exe).unwrap();
    assert_eq!((config.mode, config.id), (Mode::Linked, marker.id));
    let roots = Roots::open(&exe, None);
    assert_eq!(
        roots.layout().unwrap().content,
        library.join("accounts/me/content")
    );
}

#[test]
fn bringing_in_beside_the_program_is_imported() {
    let base = scratch();
    let exe = legacy(&base, "program");
    let layout = migrate::move_in(
        &exe,
        &exe,
        &exe.join("library"),
        "My library",
        &MoveProgress::default(),
        Publish::New,
    )
    .unwrap();
    assert_eq!(disk::read_config(&exe).unwrap().mode, Mode::Imported);
    assert_eq!(snapshot(&layout.content), snapshot(&exe.join("content")));
}

#[test]
fn bringing_in_never_replaces_a_configuration() {
    let base = scratch();
    let source = legacy(&base, "source");
    let exe = program(&base, "program");
    fs::write(exe.join(CONFIG_FILE), b"mine").unwrap();
    let library = base.join("library");
    let error = migrate::move_in(
        &exe,
        &source,
        &library,
        "L",
        &MoveProgress::default(),
        Publish::New,
    );
    assert!(error.is_err());
    assert!(!library.exists());
    assert_eq!(fs::read(exe.join(CONFIG_FILE)).unwrap(), b"mine");
}

/// A file another program holds open without sharing fails the copy; the
/// move then removes what it made and the source is as it was.
#[cfg(windows)]
#[test]
fn a_failed_move_removes_what_it_made() {
    use std::os::windows::fs::OpenOptionsExt;
    for pre_made in [false, true] {
        let base = scratch();
        let source = legacy(&base, "source");
        let before = snapshot(&source);
        let exe = program(&base, "program");
        let library = base.join("library");
        if pre_made {
            fs::create_dir(&library).unwrap();
        }
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(source.join("assets/captures/shot.png"))
            .unwrap();
        let error = migrate::move_in(
            &exe,
            &source,
            &library,
            "L",
            &MoveProgress::default(),
            Publish::New,
        )
        .unwrap_err();
        drop(held);
        assert!(error.contains("Nothing was changed"), "{error}");
        assert_eq!(library.exists(), pre_made);
        if pre_made {
            assert_eq!(fs::read_dir(&library).unwrap().count(), 0);
        }
        assert!(!exe.join(CONFIG_FILE).exists());
        assert_eq!(snapshot(&source), before);
    }
}

/// `exe` linked to the library at `last`, as a moved drive leaves it.
fn linked_to(exe: &Path, library: &Path, last: &Path) {
    let marker = read_marker(library).unwrap();
    disk::create_config(
        exe,
        &Config {
            v: 1,
            mode: Mode::Linked,
            id: marker.id,
            label: marker.label,
            account: AccountId::default(),
            last_path: Some(last.to_path_buf()),
        },
    )
    .unwrap();
}

#[test]
fn a_linked_library_found_on_another_drive_is_remembered() {
    let base = scratch();
    let exe = program(&base, "program");
    let moved = imported(&program(&base, "drive-b")).library;
    let gone = base.join("drive-a/library");
    linked_to(&exe, &moved, &gone);
    let mut roots = Roots::open(&exe, None);
    assert!(matches!(roots.state(), State::Disconnected { .. }));
    assert_eq!(
        roots.remember(),
        Ok(false),
        "nothing found, nothing written"
    );
    assert_eq!(disk::read_config(&exe).unwrap().last_path, Some(gone));
    roots
        .refresh_with_candidates(std::slice::from_ref(&moved))
        .unwrap();
    assert_eq!(roots.remember(), Ok(true));
    assert_eq!(roots.remember(), Ok(false), "written once");
    assert_eq!(
        disk::read_config(&exe).unwrap().last_path,
        Some(moved.clone())
    );
    assert!(!exe.join("roots.toml.new").exists());
    assert_eq!(Roots::open(&exe, None).layout().unwrap().library, moved);
}

#[test]
fn an_imported_library_and_a_seam_never_rewrite_roots_toml() {
    let base = scratch();
    let layout = imported(&base);
    let before = fs::read(base.join(CONFIG_FILE)).unwrap();
    assert_eq!(Roots::open(&base, None).remember(), Ok(false));
    let seam = DrillRoots {
        content: layout.content.clone(),
        assets: None,
        userdata: None,
    };
    assert_eq!(Roots::open(&base, Some(seam)).remember(), Ok(false));
    assert_eq!(fs::read(base.join(CONFIG_FILE)).unwrap(), before);
}

#[test]
fn replacing_roots_toml_finishes_an_interrupted_replace() {
    let base = scratch();
    let exe = program(&base, "program");
    let library = imported(&program(&base, "drive")).library;
    linked_to(&exe, &library, &base.join("elsewhere/library"));
    // An earlier replace that stopped before its rename.
    fs::write(exe.join("roots.toml.new"), b"half").unwrap();
    setup::stage(&exe, &library, Publish::Replace).unwrap();
    assert_eq!(disk::read_config(&exe).unwrap().last_path, Some(library));
    assert!(!exe.join("roots.toml.new").exists());
}

/// A `roots.toml` held open without sharing can't be renamed over; the
/// replace then fails and the old file is exactly as it was.
#[cfg(windows)]
#[test]
fn a_replace_that_cannot_finish_leaves_the_old_file() {
    use std::os::windows::fs::OpenOptionsExt;
    let base = scratch();
    let exe = program(&base, "program");
    let library = imported(&program(&base, "drive")).library;
    linked_to(&exe, &library, &base.join("elsewhere/library"));
    let before = fs::read(exe.join(CONFIG_FILE)).unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(exe.join(CONFIG_FILE))
        .unwrap();
    assert!(setup::stage(&exe, &library, Publish::Replace).is_err());
    drop(held);
    assert_eq!(fs::read(exe.join(CONFIG_FILE)).unwrap(), before);
    assert!(!exe.join("roots.toml.new").exists());
}

#[test]
fn locate_names_this_library_or_another_and_writes_nothing() {
    let base = scratch();
    let exe = program(&base, "program");
    let ours = imported(&program(&base, "drive-b")).library;
    linked_to(&exe, &ours, &base.join("drive-a/library"));
    let theirs = imported(&program(&base, "drive-c")).library;
    let plain = program(&base, "plain");
    let config = fs::read(exe.join(CONFIG_FILE)).unwrap();
    let (ours_before, theirs_before) = (snapshot(&ours), snapshot(&theirs));
    assert_eq!(locate(&exe, &ours), Ok(Located::Same(ours.clone())));
    assert_eq!(
        locate(&exe, &theirs),
        Ok(Located::Other {
            library: theirs.clone(),
            label: "My library".into()
        })
    );
    assert!(locate(&exe, &plain)
        .unwrap_err()
        .contains("isn't a DevPULSE library"));
    assert!(locate(&exe, Path::new("relative")).is_err());
    assert_eq!(fs::read(exe.join(CONFIG_FILE)).unwrap(), config);
    assert_eq!(snapshot(&ours), ours_before);
    assert_eq!(snapshot(&theirs), theirs_before);
    assert_eq!(fs::read_dir(&plain).unwrap().count(), 0);
}

#[test]
fn another_library_replaces_roots_toml_only_when_asked() {
    let base = scratch();
    let exe = program(&base, "program");
    let drive = imported(&program(&base, "drive")).library;
    linked_to(&exe, &drive, &base.join("unplugged/library"));
    let here = exe.join("library");
    assert!(
        setup::setup_publishing(&exe, &here, Mode::Imported, "Here", Publish::New)
            .unwrap_err()
            .contains("already exists")
    );
    assert!(!here.exists());
    let layout =
        setup::setup_publishing(&exe, &here, Mode::Imported, "Here", Publish::Replace).unwrap();
    let config = disk::read_config(&exe).unwrap();
    assert_eq!(
        (config.mode, config.label.as_str()),
        (Mode::Imported, "Here")
    );
    assert_eq!(Roots::open(&exe, None).layout().unwrap(), layout);
    assert!(
        read_marker(&drive).is_ok(),
        "the drive's library is untouched"
    );
}

/// The only test that touches the process-wide handle. A probe that
/// connects is installed; one that raced a `roots.toml` write is not.
#[test]
fn a_reconnect_installs_only_a_probe_no_choice_has_overtaken() {
    let base = scratch();
    let exe = program(&base, "program");
    let drive = imported(&program(&base, "drive")).library;
    linked_to(&exe, &drive, &drive);
    let away = base.join("away");
    fs::rename(&drive, &away).unwrap();
    *global().write().unwrap() = Some(Roots::open(&exe, None));
    assert!(!needs_setup() && try_layout().is_err(), "unplugged");
    // Plugged back in, but the screen wrote roots.toml while it probed.
    fs::rename(&away, &drive).unwrap();
    let raced = reconnect_with(|| {
        disk::CONFIG_WRITES.fetch_add(1, Ordering::AcqRel);
        Roots::open(&exe, None)
    });
    assert_eq!(raced, None);
    assert!(try_layout().is_err(), "the raced probe was not installed");
    // Tests running alongside write configurations of their own, which the
    // guard counts too; a quiet moment comes within a few tries.
    let layout = (0..100)
        .find_map(|_| reconnect_with(|| Roots::open(&exe, None)))
        .unwrap();
    assert_eq!(layout.library, drive);
    assert_eq!(try_layout().unwrap(), layout);
    *global().write().unwrap() = None;
}

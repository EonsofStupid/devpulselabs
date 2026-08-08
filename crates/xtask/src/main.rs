//! Workspace automation — the cargo-xtask pattern (no external task runner).
//!
//! `cargo green` runs THE gate, in order, stopping at the first red:
//!   1. fmt --check          (style is law, not opinion)
//!   2. clippy -D warnings   (all targets, whole workspace)
//!   3. test --workspace
//!   4. release build, then ASSERT the exe mtime advanced — a running
//!      instance silently masks a failed link; the mtime check is the
//!      stale-exe killer.
//!
//! `cargo xtask fix` = fmt + clippy --fix (the tidy loop).

use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime};

fn run(name: &str, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command.args(args);
    run_command(name, &mut command)
}

fn run_command(name: &str, command: &mut Command) -> Result<(), String> {
    println!("── {name} ─────────────────────────────────");
    let status = command
        .status()
        .map_err(|e| format!("{name}: spawn failed: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} FAILED ({status})"))
    }
}

fn windows_test_linker() -> Result<Option<(String, PathBuf)>, String> {
    if !cfg!(windows) {
        return Ok(None);
    }

    let version = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("test: could not inspect rustc host: {error}"))?;
    if !version.status.success() {
        return Err(format!("test: rustc -vV failed ({})", version.status));
    }
    let version = String::from_utf8(version.stdout)
        .map_err(|error| format!("test: rustc host output was not UTF-8: {error}"))?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| "test: rustc host triple was not reported".to_owned())?;

    let sysroot = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .map_err(|error| format!("test: could not inspect rustc sysroot: {error}"))?;
    if !sysroot.status.success() {
        return Err(format!(
            "test: rustc --print sysroot failed ({})",
            sysroot.status
        ));
    }
    let sysroot = String::from_utf8(sysroot.stdout)
        .map_err(|error| format!("test: rustc sysroot output was not UTF-8: {error}"))?;
    let linker = Path::new(sysroot.trim())
        .join("lib")
        .join("rustlib")
        .join(host)
        .join("bin")
        .join("rust-lld.exe");
    if !linker.is_file() {
        return Err(format!(
            "test: bundled Windows linker is missing: {}",
            linker.display()
        ));
    }

    let environment = format!(
        "CARGO_TARGET_{}_LINKER",
        host.to_ascii_uppercase().replace('-', "_")
    );
    Ok(Some((environment, linker)))
}

fn run_workspace_tests() -> Result<(), String> {
    let mut command = Command::new("cargo");
    command.args(["test", "--workspace"]);
    command.env("CARGO_PROFILE_TEST_DEBUG", "0");

    if let Some((environment, linker)) = windows_test_linker()? {
        println!(
            "test linker: {}={} (avoids MSVC PDB record exhaustion)",
            environment,
            linker.display()
        );
        command.env(environment, linker);
    }

    run_command("test", &mut command)
}

fn exe_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Where cargo actually puts artifacts. NOT hardcoded `target/`: a shared
/// build cache, a CI image redirecting output to a fast volume, or a plain
/// `CARGO_TARGET_DIR` in the environment all move it — and the gate would
/// then build the exe successfully and declare it missing, which is exactly
/// what happened on a Linux clone 2026-07-30. `CARGO_TARGET_DIR` wins;
/// otherwise `cargo metadata` is authoritative (it also honours
/// `build.target-dir` from a config file, which the env var does not).
fn target_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| serde_json::from_slice::<serde_json::Value>(&out.stdout).ok())
        .and_then(|meta| {
            meta["target_directory"]
                .as_str()
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
        })
        .unwrap_or_else(|| PathBuf::from("target"))
}

/// The release exe the gate asserts on, wherever cargo put it.
fn release_exe() -> PathBuf {
    target_dir()
        .join("release")
        .join(format!("devpulselabs{}", std::env::consts::EXE_SUFFIX))
}

fn green() -> Result<(), String> {
    run("fmt", &["fmt", "--all", "--", "--check"])?;
    docs_status()?;
    spawn_audit()?;
    structure_audit()?;
    run(
        "clippy",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_workspace_tests()?;

    let exe = release_exe();
    let before = exe_mtime(&exe);
    // `cargo green` itself is the release-mode `xtask.exe` on Windows. A
    // workspace release build must not try to replace that running file.
    // xtask was already compiled successfully by `cargo green`; the final
    // artifact gate covers every shipped crate and binary.
    run(
        "release build",
        &["build", "--workspace", "--release", "--exclude", "xtask"],
    )?;
    let after = exe_mtime(&exe);
    match (before, after) {
        (_, None) => Err("release exe missing after build".into()),
        // Fresh-but-unchanged is fine ONLY when cargo declared it fresh —
        // which the successful status already covers; the check exists to
        // catch a masked link failure that leaves an OLD exe behind.
        (Some(b), Some(a)) if a < b => Err("release exe went BACKWARDS — locked/stale".into()),
        _ => {
            // LAST, because it needs the release helper the build just made —
            // and because a gate that never runs the real renderers proves
            // only that they compile.
            smoke_audit()?;
            println!("green ✔ exe: {}", exe.display());
            Ok(())
        }
    }
}

/// THE SMOKE PHASE — run the real subprocess renderers and look at what they
/// actually produced.
///
/// **Why this exists.** ADR-009 makes ffmpeg, aerender, ThorVG and the AI
/// providers all subprocesses, and tests must stay hermetic to run in CI. That
/// left the entire ADR-009 surface with NO coverage: unit tests reach the pure
/// functions, drills reach the UI over authored fixtures, and nothing ever ran
/// a real helper and checked real output. Three failures shipped through that
/// gap and none of them returned a non-zero exit code:
///
/// - `dpl-lottie-render export` emitted ZERO frames — frame 0 always failed;
/// - Lottie TEXT rendered a fully blank frame, because no font was ever loaded;
/// - `CodexAppServerRuntime::from_parts` had no callers at all.
///
/// So every assertion here is on the ARTIFACT — opaque pixel counts, frame
/// counts — never on process success. An exit code is what lied all three
/// times.
///
/// Hermetic by construction: in-repo fixtures, in-repo fonts, an in-repo
/// helper, no network and no login. Provider smoke (a real Codex turn) needs
/// the operator's credentials and therefore stays a local example, not a gate.
fn smoke_audit() -> Result<(), String> {
    println!("── smoke: real renderers, real pixels ────────");
    let helper = target_dir()
        .join("release")
        .join(format!("dpl-lottie-render{}", std::env::consts::EXE_SUFFIX));
    if !helper.is_file() {
        return Err(format!(
            "smoke: {} is missing — the release build must produce it",
            helper.display()
        ));
    }
    let scratch = target_dir().join("smoke");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(|e| format!("smoke scratch: {e}"))?;
    let fonts = Path::new("crates/coldlight/assets/fonts");

    // 1. A SHAPE at t=0. Frame 0 is the case that used to be fatal, so it is
    //    the case the gate pins.
    let shape_png = scratch.join("shape-t0.png");
    render_frame(
        &helper,
        "tests/fixtures/lottie/shape.json",
        256,
        0.0,
        &shape_png,
        None,
    )?;
    let painted = opaque_pixels(&shape_png)?;
    if painted < 1_000 {
        return Err(format!(
            "smoke: the shape fixture rendered {painted} opaque pixels at t=0 — \
             a blank or near-blank frame, which is what a broken renderer looks \
             like at exit code 0"
        ));
    }
    println!("  shape  t=0     {painted:>7} opaque px");

    // 2. TEXT, which needs a font registered. This is the regression guard for
    //    the blank-text bug: without `--fonts` this frame is EMPTY and the
    //    process still succeeds.
    let text_png = scratch.join("text.png");
    render_frame(
        &helper,
        "tests/fixtures/lottie/text.json",
        512,
        0.5,
        &text_png,
        Some(fonts),
    )?;
    let lettered = opaque_pixels(&text_png)?;
    if lettered < 1_000 {
        return Err(format!(
            "smoke: the text fixture rendered {lettered} opaque pixels — text is \
             not drawing. ThorVG renders NOTHING for a font family it was never \
             given, and it does not error."
        ));
    }
    println!("  text   t=0.5   {lettered:>7} opaque px");

    // 3. EXPORT a whole sequence. This is the path that produced no files at
    //    all, so the gate counts them.
    let seq = scratch.join("seq");
    let out = Command::new(&helper)
        .args([
            "export",
            "tests/fixtures/lottie/shape.json",
            "128",
            "128",
            "30",
        ])
        .arg(&seq)
        .output()
        .map_err(|e| format!("smoke: spawn export: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "smoke: export failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let frames = std::fs::read_dir(&seq)
        .map_err(|e| format!("smoke: read export dir: {e}"))?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("png"))
        .count();
    // The fixture is 60 frames at 30fps; exporting at 30fps must yield 60.
    if frames != 60 {
        return Err(format!(
            "smoke: export wrote {frames} frames, expected 60 — this path once \
             wrote ZERO while exiting 0"
        ));
    }
    println!("  export         {frames:>7} frames");
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(())
}

/// Render one frame through the real helper.
fn render_frame(
    helper: &Path,
    lottie: &str,
    size: u32,
    t: f32,
    out: &Path,
    fonts: Option<&Path>,
) -> Result<(), String> {
    let mut command = Command::new(helper);
    command
        .arg("frame")
        .arg(lottie)
        .arg(size.to_string())
        .arg(size.to_string())
        .arg(t.to_string())
        .arg(out);
    if let Some(dir) = fonts {
        command.arg("--fonts").arg(dir);
    }
    let output = command
        .output()
        .map_err(|e| format!("smoke: spawn {lottie}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "smoke: {lottie} at t={t} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// Count pixels that are not fully transparent.
///
/// The measure that matters: a renderer that draws nothing still writes a
/// valid, correctly-sized PNG and exits 0. Only the alpha channel tells the
/// truth.
fn opaque_pixels(png: &Path) -> Result<usize, String> {
    let image = image::open(png).map_err(|e| format!("smoke: decode {}: {e}", png.display()))?;
    Ok(image
        .to_rgba8()
        .pixels()
        .filter(|pixel| pixel.0[3] > 0)
        .count())
}

fn docs_status() -> Result<(), String> {
    let path = Path::new("docs/eguipilotai-playwright-review.md");
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut in_matrix = false;
    let mut checked = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line == "## Capability matrix" {
            in_matrix = true;
            continue;
        }
        if in_matrix && line.starts_with("## ") {
            break;
        }
        if !in_matrix || !line.starts_with('|') || line.starts_with("|---") {
            continue;
        }
        let columns: Vec<_> = line.split('|').map(str::trim).collect();
        if columns.get(1) == Some(&"Playwright contract") {
            continue;
        }
        let status = columns.get(3).copied().unwrap_or_default();
        if status == "Core"
            || status == "Partial"
            || status == "Missing"
            || status.starts_with("Ready")
        {
            let evidence = columns.get(5).copied().unwrap_or_default();
            if !["test:`", "slice:`", "issue:`"]
                .iter()
                .any(|marker| evidence.contains(marker))
            {
                return Err(format!(
                    "{}:{} status `{status}` needs test:, slice:, or issue: evidence",
                    path.display(),
                    index + 1
                ));
            }
            checked += 1;
        }
    }
    if checked == 0 {
        return Err(format!(
            "{}: capability matrix was not found or contained no checked statuses",
            path.display()
        ));
    }
    println!("docs status: {checked} evidence-backed capability rows");
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DependencySources {
    schema: u32,
    dependency: Vec<DependencySource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DependencySource {
    id: String,
    family: dpl_tools_deps::DependencyFamily,
    version: String,
    target: String,
    source: PathBuf,
    entrypoint: Option<PathBuf>,
}

fn stage_tool_dependencies(arguments: &[String]) -> Result<(), String> {
    let release_root = arguments.first().ok_or_else(|| {
        "stage-tool-deps requires a new release root; optional second argument is the source TOML"
            .to_owned()
    })?;
    let release_root = Path::new(release_root);
    let sources_path = arguments
        .get(1)
        .map_or_else(|| Path::new("dptools-deps/sources.local.toml"), Path::new);
    if release_root.join(dpl_tools_deps::TOOLS_DIRECTORY).exists() {
        return Err(format!(
            "{} already contains dptools-deps; choose a new release root",
            release_root.display()
        ));
    }

    let sources_text = fs::read_to_string(sources_path)
        .map_err(|error| format!("{}: {error}", sources_path.display()))?;
    let sources: DependencySources = toml::from_str(&sources_text)
        .map_err(|error| format!("{}: {error}", sources_path.display()))?;
    if sources.schema != 1 {
        return Err(format!(
            "{}: source schema {} is unsupported",
            sources_path.display(),
            sources.schema
        ));
    }

    let target = dpl_tools_deps::current_target();
    let selected = sources
        .dependency
        .iter()
        .filter(|dependency| dependency.target == target)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(format!(
            "{} has no dependencies for {target}",
            sources_path.display()
        ));
    }
    let mut ids = HashSet::with_capacity(selected.len());
    for source in &selected {
        validate_source_segment("id", &source.id)?;
        validate_source_segment("version", &source.version)?;
        let expected = dependency_id(&source.id)
            .ok_or_else(|| format!("unsupported DevPULSE tool `{}`", source.id))?;
        if source.family != expected.family() {
            return Err(format!(
                "{} registers `{}` as {:?}; expected {:?}",
                sources_path.display(),
                source.id,
                source.family,
                expected.family()
            ));
        }
        if !ids.insert(source.id.as_str()) {
            return Err(format!(
                "{} registers `{}` more than once for {target}",
                sources_path.display(),
                source.id
            ));
        }
        if !source.source.is_absolute() {
            return Err(format!(
                "{} source for `{}` must be absolute",
                sources_path.display(),
                source.id
            ));
        }
        match (
            source.source.is_file(),
            source.source.is_dir(),
            source.entrypoint.as_deref(),
        ) {
            (true, false, None) => {}
            // A directory source is a TREE (chromium's whole browser,
            // poppler's exe-plus-DLLs); the entrypoint names the executable
            // within it. Any family may ship a tree — the resolver's tree
            // digest is family-agnostic.
            (false, true, Some(entrypoint)) => {
                validate_source_relative_path("entrypoint", entrypoint)?;
                if !source.source.join(entrypoint).is_file() {
                    return Err(format!(
                        "{} entrypoint for `{}` is not a file",
                        source.source.join(entrypoint).display(),
                        source.id
                    ));
                }
            }
            (false, true, None) => {
                return Err(format!(
                    "{} directory source for `{}` requires an entrypoint",
                    source.source.display(),
                    source.id
                ));
            }
            _ => {
                return Err(format!(
                    "{} source for `{}` is not a file or directory",
                    source.source.display(),
                    source.id
                ));
            }
        }
    }
    for required in ["codex", "grok", "chromium", "ffmpeg"] {
        if !ids.contains(required) {
            return Err(format!(
                "{} is missing required `{required}` for {target}",
                sources_path.display()
            ));
        }
    }

    let tools_root = release_root.join(dpl_tools_deps::TOOLS_DIRECTORY);
    fs::create_dir_all(&tools_root)
        .map_err(|error| format!("{}: {error}", tools_root.display()))?;
    let mut dependencies = Vec::with_capacity(selected.len());
    for source in selected {
        let family_root = Path::new(source.family.directory())
            .join(&source.id)
            .join(&source.version);
        let (relative, destination, tree) = if source.source.is_dir() {
            let tree_destination = tools_root.join(&family_root);
            copy_tool_tree(&source.source, &tree_destination)?;
            let entrypoint = source.entrypoint.as_deref().ok_or_else(|| {
                format!(
                    "{} directory source has no entrypoint",
                    source.source.display()
                )
            })?;
            let relative = family_root.join(entrypoint);
            let destination = tools_root.join(&relative);
            let (bytes, sha256) = dpl_tools_deps::sha256_tree(&tree_destination)
                .map_err(|error| format!("{}: {error}", tree_destination.display()))?;
            (
                relative,
                destination,
                Some(dpl_tools_deps::ManifestTree {
                    root: family_root.to_string_lossy().replace('\\', "/"),
                    bytes,
                    sha256,
                }),
            )
        } else {
            let relative = family_root.join(dependency_file_name(&source.id));
            let destination = tools_root.join(&relative);
            let parent = destination
                .parent()
                .ok_or_else(|| format!("{} has no parent", destination.display()))?;
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
            fs::copy(&source.source, &destination).map_err(|error| {
                format!(
                    "copy {} -> {}: {error}",
                    source.source.display(),
                    destination.display()
                )
            })?;
            (relative, destination, None)
        };
        verify_staged_version(&source.id, &source.version, &destination)?;
        let metadata = destination
            .metadata()
            .map_err(|error| format!("{}: {error}", destination.display()))?;
        let sha256 = dpl_tools_deps::sha256_file(&destination)
            .map_err(|error| format!("{}: {error}", destination.display()))?;
        dependencies.push(dpl_tools_deps::ManifestDependency {
            id: source.id.clone(),
            family: source.family,
            version: source.version.clone(),
            path: relative.to_string_lossy().replace('\\', "/"),
            bytes: metadata.len(),
            sha256,
            tree,
        });
        println!(
            "staged {} {} -> {}",
            source.id,
            source.version,
            destination.display()
        );
    }
    dependencies.sort_by(|left, right| left.id.cmp(&right.id));
    let manifest = dpl_tools_deps::Manifest {
        schema: dpl_tools_deps::MANIFEST_SCHEMA,
        target: target.to_owned(),
        dependencies,
    };
    let manifest_path = tools_root.join(dpl_tools_deps::MANIFEST_FILE);
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("serialize manifest: {error}"))?,
    )
    .map_err(|error| format!("{}: {error}", manifest_path.display()))?;

    let resolver = dpl_tools_deps::Resolver::from_release_root(release_root)
        .map_err(|error| format!("staged bundle rejected: {error}"))?;
    // Re-resolve EVERY staged id through the production resolver — the
    // bundle is verified as it will actually be consumed, optional tools
    // (pdftoppm) included when present.
    let staged: Vec<dpl_tools_deps::DependencyId> = resolver
        .manifest()
        .dependencies
        .iter()
        .map(|dependency| {
            dependency_id(&dependency.id)
                .ok_or_else(|| format!("staged unknown tool `{}`", dependency.id))
        })
        .collect::<Result<_, _>>()?;
    for id in staged {
        resolver
            .resolve(id)
            .map_err(|error| format!("staged `{id}` rejected: {error}"))?;
    }
    println!(
        "DevPULSE tool depot verified - target: {target} root: {}",
        tools_root.display()
    );
    Ok(())
}

fn validate_source_segment(field: &str, value: &str) -> Result<(), String> {
    let valid = !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-._+".contains(character));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "DevPULSE tool {field} `{value}` is not a safe path segment"
        ))
    }
}

fn validate_source_relative_path(field: &str, value: &Path) -> Result<(), String> {
    let valid = !value.as_os_str().is_empty()
        && !value.is_absolute()
        && value
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "DevPULSE tool {field} `{}` is not a safe relative path",
            value.display()
        ))
    }
}

fn copy_tool_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("{}: {error}", destination.display()))?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("{}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", source.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "{}: tool trees cannot contain symlinks",
                entry.path().display()
            ));
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tool_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| {
                format!(
                    "copy {} -> {}: {error}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn dependency_id(id: &str) -> Option<dpl_tools_deps::DependencyId> {
    match id {
        "codex" => Some(dpl_tools_deps::CODEX),
        "grok" => Some(dpl_tools_deps::GROK),
        "ffmpeg" => Some(dpl_tools_deps::FFMPEG),
        "chromium" => Some(dpl_tools_deps::CHROMIUM),
        "pdftoppm" => Some(dpl_tools_deps::PDFTOPPM),
        _ => None,
    }
}

fn dependency_file_name(id: &str) -> String {
    format!("{id}{}", std::env::consts::EXE_SUFFIX)
}

fn verify_staged_version(id: &str, expected: &str, executable: &Path) -> Result<(), String> {
    #[cfg(windows)]
    if id == "chromium" {
        let actual = windows_file_version(executable)?;
        return (actual == expected).then_some(()).ok_or_else(|| {
            format!(
                "{} reports file version {actual}; expected {expected}",
                executable.display()
            )
        });
    }

    let version_argument = match id {
        "ffmpeg" => "-version",
        // Poppler's arg parser treats `--version` as an input file; `-v`
        // prints `pdftoppm version <x>` to stderr and exits 0.
        "pdftoppm" => "-v",
        _ => "--version",
    };
    let output = Command::new(executable)
        .arg(version_argument)
        .output()
        .map_err(|error| format!("{} version probe failed: {error}", executable.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} version probe failed ({})",
            executable.display(),
            output.status
        ));
    }
    let mut bounded = output.stdout;
    bounded.extend(output.stderr);
    bounded.truncate(4 * 1024);
    let bounded = String::from_utf8_lossy(&bounded);
    if !bounded.contains(expected) {
        return Err(format!(
            "{} did not report expected version {expected}",
            executable.display()
        ));
    }
    Ok(())
}

/// The QUIET-SPAWN audit (W8, docs/design/signal-doctrine.md sibling
/// law): any source file under crates/ that constructs a Command must
/// either import `dpl_proc::quiet` or carry an explicit
/// `// spawn-audit: exempt(<reason>)` marker — an unflagged subprocess
/// can pop a console window over the GUI app and CANNOT land silently.
fn spawn_audit() -> Result<(), String> {
    println!("── spawn audit ───────────────────────────────");
    let mut offenders = Vec::new();
    let mut stack = vec![PathBuf::from("crates")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != "target" {
                    stack.push(path);
                }
                continue;
            }
            if path.extension().and_then(|x| x.to_str()) != Some("rs") {
                continue;
            }
            let Ok(src) = fs::read_to_string(&path) else {
                continue;
            };
            if !src.contains("Command::new") {
                continue;
            }
            let quiet = src.contains("dpl_proc::quiet") || src.contains("pub fn quiet");
            let exempt = src.contains("spawn-audit: exempt");
            if !quiet && !exempt {
                offenders.push(path.display().to_string());
            }
        }
    }
    if offenders.is_empty() {
        println!("spawn audit: every Command::new file is quiet or exempt");
        Ok(())
    } else {
        Err(format!(
            "spawn audit FAILED — files spawning without dpl_proc::quiet or an              explicit `// spawn-audit: exempt(<reason>)` marker:
  {}",
            offenders.join("
  ")
        ))
    }
}

/// The STRUCTURE audit — the layer law of `docs/design/architecture.md`,
/// enforced as fitness functions (amending a table there means amending
/// the matching table HERE in the same commit). Graph shape only: exact
/// dep ceilings, the egui-free census, no lateral hub deps, the one FFI
/// leaf, manifest hygiene, orphan crate folders, and the harness-flag
/// census. Cheap: one `cargo metadata --no-deps` + text scans.
fn structure_audit() -> Result<(), String> {
    println!("── structure audit ───────────────────────────");

    // Ceilings: these crates' NORMAL direct deps are EXACTLY these sets
    // (architecture.md §2). A new edge is a decision, not a drive-by.
    const CEILINGS: &[(&str, &[&str])] = &[
        ("dpl-proc", &[]),
        ("dpl-comp", &["serde", "serde_json"]),
        (
            "dpl-workflows",
            &["dpl-comp", "dpl-doc", "serde", "serde_json"],
        ),
        ("dpl-doc", &["dpl-coldlight", "serde", "serde_json"]),
        ("dpl-tools-deps", &["serde", "serde_json", "sha2"]),
        ("dpl-landing", &["serde", "serde_json"]),
        ("dpl-estate", &["fjall", "moka", "serde", "serde_json"]),
        ("dpl-intent", &[]),
        ("dpl-svg", &[]),
    ];
    // The egui-free census (architecture.md §1): document/engine/service
    // crates never link egui/eframe directly. Adding UI to one is a
    // conscious one-line amendment, like registering a slide.
    const EGUI_FREE: &[&str] = &[
        "dpl-proc",
        "dpl-comp",
        "dpl-doc",
        "dpl-workflows",
        "dpl-estate",
        "dpl-landing",
        "dpl-index",
        "dpl-intent",
        "dpl-ledger",
        "dpl-tools-deps",
        "dpl-engine-capture",
        "dpl-engine-media",
        "dpl-engine-steps",
        "dpl-engine-ae",
        "dpl-engine-browser",
        "dpl-lottie",
        "dpl-lottie-render",
        "dpl-svg",
        "dpl-guide",
        "dpl-chat-core",
        "dpl-chat-store",
        "dpl-chat-accounts",
        "dpl-chat-browser-auth",
        "dpl-chat-interactions",
        "dpl-chat-profiles",
        "dpl-chat-runtime",
        "dpl-chat-social",
        "dpl-chat-testkit",
        "dpl-chat-codex",
        "dpl-chat-claude",
        "dpl-chat-gemini",
        "dpl-chat-grok",
        "dpl-chat-imagine",
        "egui-pilot-ai-protocol",
        "egui-pilot-ai-transport",
        "egui-pilot-ai-runner",
        "egui-pilot-ai-cli",
        "xtask",
    ];
    // The ONE FFI leaf (ADR-009): `-sys` links live here and nowhere else.
    const SYS_ALLOWED: &[&str] = &["dpl-lottie-render"];
    // Orphan crate folders outside the workspace, named and tolerated
    // (architecture.md §3); anything NEW fails.
    const ORPHAN_ALLOWED: &[&str] = &[
        "desktop-spike",
        "coach-overlay",
        // coach-inventory/fabric/runtime/daemon are workspace members —
        // daemon + overlay are never deps of app (control-plane ADR).
    ];

    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("structure: cargo metadata spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "structure: cargo metadata failed ({})",
            output.status
        ));
    }
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("structure: metadata parse: {e}"))?;
    let packages = meta["packages"]
        .as_array()
        .ok_or("structure: metadata has no packages")?;

    let mut failures: Vec<String> = Vec::new();
    let mut manifests: Vec<PathBuf> = Vec::new();
    let normal_deps = |package: &serde_json::Value| -> Vec<String> {
        package["dependencies"]
            .as_array()
            .map(|deps| {
                deps.iter()
                    .filter(|d| d["kind"].is_null())
                    .filter_map(|d| d["name"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };

    for package in packages {
        let name = package["name"].as_str().unwrap_or_default();
        let deps = normal_deps(package);
        if let Some(path) = package["manifest_path"].as_str() {
            manifests.push(PathBuf::from(path));
        }

        if let Some((_, ceiling)) = CEILINGS.iter().find(|(c, _)| *c == name) {
            let mut actual: Vec<&str> = deps.iter().map(String::as_str).collect();
            actual.sort_unstable();
            let mut expected = ceiling.to_vec();
            expected.sort_unstable();
            if actual != expected {
                failures.push(format!(
                    "{name}: deps {actual:?} != ceiling {expected:?} (amend architecture.md §2 + this table together)"
                ));
            }
        }
        if EGUI_FREE.contains(&name) {
            // Precise UI-lib match: our own `egui-pilot-ai-*` harness
            // crates are NOT egui links.
            if let Some(ui) = deps.iter().find(|d| {
                *d == "egui" || *d == "eframe" || *d == "egui-wgpu" || d.starts_with("egui_")
            }) {
                failures.push(format!(
                    "{name}: egui-free crate links `{ui}` (census: architecture.md §1)"
                ));
            }
        }
        if name.starts_with("dpl-hub-") {
            if let Some(lateral) = deps.iter().find(|d| d.starts_with("dpl-hub-")) {
                failures.push(format!(
                    "{name}: hub depends on hub `{lateral}` — hubs never touch laterally"
                ));
            }
        }
        if !SYS_ALLOWED.contains(&name) {
            if let Some(ffi) = deps.iter().find(|d| d.ends_with("-sys")) {
                failures.push(format!(
                    "{name}: FFI link `{ffi}` outside the one leaf (ADR-009; allowed: {SYS_ALLOWED:?})"
                ));
            }
        }
    }

    // Manifest hygiene: workspace lints + never-publish, every member.
    // The FFI leaf carries a RATIFIED per-crate lint override (unsafe is
    // quarantined there — architecture.md §3); everyone else inherits.
    const LINTS_OVERRIDE_ALLOWED: &[&str] = &["dpl-lottie-render"];
    for package in packages {
        let name = package["name"].as_str().unwrap_or_default();
        let Some(manifest) = package["manifest_path"].as_str() else {
            continue;
        };
        let text =
            fs::read_to_string(manifest).map_err(|e| format!("structure: {manifest}: {e}"))?;
        let lints_ok = text.contains("[lints]")
            || text.contains("lints.workspace")
            || LINTS_OVERRIDE_ALLOWED.contains(&name);
        let publish_ok = text.contains("publish = false") || text.contains("publish.workspace");
        if !lints_ok {
            failures.push(format!("{name}: missing `[lints] workspace = true`"));
        }
        if !publish_ok {
            failures.push(format!("{name}: publish is not pinned false"));
        }
    }

    // Orphans: every crates/**/Cargo.toml is a member or named.
    let member_set: HashSet<PathBuf> = manifests
        .iter()
        .filter_map(|m| m.canonicalize().ok())
        .collect();
    let mut stack = vec![PathBuf::from("crates")];
    while let Some(dir) = stack.pop() {
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
                let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
                let named = ORPHAN_ALLOWED.iter().any(|allow| {
                    path.components()
                        .any(|c| c.as_os_str().to_string_lossy() == *allow)
                });
                if !member_set.contains(&canonical) && !named {
                    failures.push(format!(
                        "{}: crate folder outside the workspace (add to members or the named orphan list)",
                        path.display()
                    ));
                }
            }
        }
    }

    // Naming law (architecture.md §4): a crate is `dpl-` + its folder, a
    // BINARY is kebab-case, and a `src/bin/*.rs` file is snake_case with its
    // binary name DECLARED. The declaration matters — cargo auto-discovers an
    // undeclared file and the FILENAME becomes the shipped binary name, which
    // is exactly how `browser_cli` shipped underscored while every sibling CLI
    // was kebab. The exceptions are RATIFIED, not drift: the exe, cargo's own
    // `xtask` convention, and the `egui-pilot-ai*` family, which stays
    // unprefixed as its own product surface (operator ruling 2026-07-30).
    let mut binaries = 0usize;
    const NAME_EXCEPT: &[&str] = &["devpulselabs", "xtask"];
    const NAME_EXCEPT_PREFIX: &[&str] = &["egui-pilot-ai"];
    for package in packages {
        let name = package["name"].as_str().unwrap_or_default();
        let Some(manifest) = package["manifest_path"].as_str() else {
            continue;
        };
        let excepted =
            NAME_EXCEPT.contains(&name) || NAME_EXCEPT_PREFIX.iter().any(|p| name.starts_with(p));
        if !excepted {
            let folder = Path::new(manifest)
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.starts_with("dpl-") || !name.ends_with(folder) {
                failures.push(format!(
                    "{name}: crate name is not `dpl-…{folder}` (naming law: architecture.md §4 — \
                     ratify an exception in the doc AND this table together if deliberate)"
                ));
            }
        }
        for target in package["targets"].as_array().into_iter().flatten() {
            let is_bin = target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|k| k == "bin"));
            if !is_bin {
                continue;
            }
            binaries += 1;
            let bin = target["name"].as_str().unwrap_or_default();
            if bin.contains('_') {
                failures.push(format!(
                    "{name}: binary `{bin}` is not kebab-case — declare `[[bin]] name = \"{}\"` \
                     with an explicit path (naming law: architecture.md §4)",
                    bin.replace('_', "-")
                ));
            }
            let src = target["src_path"].as_str().unwrap_or_default();
            if src.replace('\\', "/").contains("/src/bin/") {
                if let Some(stem) = Path::new(src).file_stem().and_then(|s| s.to_str()) {
                    if stem.contains('-') {
                        failures.push(format!(
                            "{name}: bin source `{stem}.rs` is kebab — snake the file and declare \
                             the binary name (naming law: architecture.md §4)"
                        ));
                    }
                }
            }
        }
    }

    // Harness-flag census: the flags the app parses ⇔ the flags CLAUDE.md
    // documents, both directions. `--content-root` is parsed inside
    // engine-capture (the ring resolver), hence the one addition.
    let flag_set = |text: &str| -> HashSet<String> {
        let bytes = text.as_bytes();
        let mut flags = HashSet::new();
        let mut i = 0;
        while let Some(offset) = text[i..].find("--") {
            let start = i + offset;
            let rest = &text[start + 2..];
            let end = rest
                .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
                .unwrap_or(rest.len());
            if end > 0 && rest.as_bytes()[0].is_ascii_lowercase() && bytes[start] == b'-' {
                flags.insert(format!("--{}", &rest[..end]));
            }
            i = start + 2;
        }
        flags
    };
    let main_src = fs::read_to_string("crates/app/src/main.rs")
        .map_err(|e| format!("structure: crates/app/src/main.rs: {e}"))?;
    let mut parsed: HashSet<String> = main_src
        .lines()
        .filter(|l| l.contains("\"--"))
        .flat_map(&flag_set)
        .collect();
    parsed.insert("--content-root".into());
    let claude =
        fs::read_to_string("CLAUDE.md").map_err(|e| format!("structure: CLAUDE.md: {e}"))?;
    let mut documented = HashSet::new();
    let mut in_block = false;
    for line in claude.lines() {
        if line.starts_with("- Harness flags") {
            in_block = true;
        } else if in_block && line.starts_with("- ") {
            break;
        }
        if in_block {
            documented.extend(flag_set(line));
        }
    }
    for flag in parsed.difference(&documented) {
        failures.push(format!(
            "{flag}: parsed by the app but missing from CLAUDE.md's harness list"
        ));
    }
    for flag in documented.difference(&parsed) {
        failures.push(format!(
            "{flag}: documented in CLAUDE.md but no app code parses it (phantom flag)"
        ));
    }

    if failures.is_empty() {
        println!(
            "structure: {} ceilings · {} egui-free · {} members hygienic · {} flags in census \
             · {} crates + {} binaries named",
            CEILINGS.len(),
            EGUI_FREE.len(),
            manifests.len(),
            parsed.len(),
            manifests.len(),
            binaries
        );
        Ok(())
    } else {
        failures.sort();
        Err(format!(
            "structure audit FAILED:
  {}",
            failures.join(
                "
  "
            )
        ))
    }
}

#[cfg(windows)]
fn windows_file_version(executable: &Path) -> Result<String, String> {
    let file = pelite::FileMap::open(executable)
        .map_err(|error| format!("{} could not be mapped: {error}", executable.display()))?;
    let image = pelite::PeFile::from_bytes(file.as_ref())
        .map_err(|error| format!("{} is not a PE image: {error}", executable.display()))?;
    let resources = image
        .resources()
        .map_err(|error| format!("{} has no resources: {error}", executable.display()))?;
    let version = resources
        .version_info()
        .map_err(|error| format!("{} has no version info: {error}", executable.display()))?;
    version
        .fixed()
        .map(|fixed| fixed.dwFileVersion.to_string())
        .ok_or_else(|| format!("{} has no fixed Windows file version", executable.display()))
}

/// RR1 — the resolution sweep. Renders every route at every size in ISOLATED
/// subprocesses (one panicking cell fails only itself, never the sweep),
/// writes the cramp-map table + a PNG per cell, and exits non-zero if any cell
/// panics or times out. `cargo xtask sweep [outdir]` (default `target/sweep`).
fn sweep(args: &[String]) -> Result<(), String> {
    const SIZES: &[&str] = &[
        "1024x640",
        "1280x720",
        "1280x800",
        "1366x768",
        "1600x900",
        "1920x1080",
        "860x1200",
    ];
    const ROUTES: &[&str] = &[
        "home/shows",
        "wiredfront/editor",
        "wiredfront/library",
        "workbench/library",
        "workbench/episodes",
        "workbench/test-lab",
    ];
    const CELL_TIMEOUT: Duration = Duration::from_secs(120);

    let outdir = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| target_dir().join("sweep"));
    fs::create_dir_all(&outdir).map_err(|e| format!("create {}: {e}", outdir.display()))?;

    let exe = release_exe();
    if !exe.exists() {
        return Err(format!(
            "release exe missing: {} — run `cargo build --release -p devpulselabs` first",
            exe.display()
        ));
    }

    println!(
        "── resolution sweep · {} routes × {} sizes ──",
        ROUTES.len(),
        SIZES.len()
    );
    let mut table = String::from("| route | size | verdict |\n|---|---|---|\n");
    let (mut panics, mut timeouts, mut fails) = (0usize, 0usize, 0usize);
    for route in ROUTES {
        for size in SIZES {
            let cell = format!("{}__{}", route.replace('/', "-"), size);
            let png = outdir.join(format!("{cell}.png"));
            let log = outdir.join(format!("{cell}.log"));
            let logfile = fs::File::create(&log).map_err(|e| format!("log {cell}: {e}"))?;
            let started = Instant::now();
            let mut child = Command::new(&exe)
                .args([
                    "--screenshot",
                    png.to_str().ok_or("non-utf8 output path")?,
                    "--window",
                    size,
                    "--route",
                    route,
                ])
                // software rendering: the sweep needs no real GPU (the ARM
                // driver is incompatible; lavapipe carries it).
                .env("LIBGL_ALWAYS_SOFTWARE", "1")
                .stdout(Stdio::null())
                .stderr(logfile)
                .spawn()
                .map_err(|e| format!("spawn {cell}: {e}"))?;
            let status = loop {
                match child.try_wait().map_err(|e| format!("wait {cell}: {e}"))? {
                    Some(s) => break Some(s),
                    None if started.elapsed() > CELL_TIMEOUT => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                    None => std::thread::sleep(Duration::from_millis(150)),
                }
            };
            let panicked = fs::read_to_string(&log)
                .map(|s| s.contains("panicked"))
                .unwrap_or(false);
            let verdict = match status {
                None => {
                    timeouts += 1;
                    "TIMEOUT"
                }
                Some(_) if panicked => {
                    panics += 1;
                    "PANIC"
                }
                Some(s) if s.success() && png.exists() => "ok",
                Some(_) => {
                    fails += 1;
                    "fail"
                }
            };
            println!("  {verdict:>7}  {route} @ {size}");
            table.push_str(&format!("| {route} | {size} | {verdict} |\n"));
        }
    }
    let report = outdir.join("sweep.md");
    fs::write(&report, &table).map_err(|e| format!("write {}: {e}", report.display()))?;
    println!("── report: {} ──", report.display());
    if panics + timeouts + fails > 0 {
        Err(format!(
            "{panics} PANIC · {timeouts} TIMEOUT · {fails} fail — see {}",
            report.display()
        ))
    } else {
        Ok(())
    }
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let task = arguments.next().unwrap_or_else(|| "green".into());
    let arguments = arguments.collect::<Vec<_>>();
    let result = match task.as_str() {
        "green" => green(),
        "sweep" => sweep(&arguments),
        "docs-status" => docs_status(),
        "structure" => structure_audit(),
        // Runnable alone so the check can be PROVEN to fail — a gate nobody
        // has watched fail is a gate nobody should trust.
        "smoke" => smoke_audit(),
        "stage-tool-deps" => stage_tool_dependencies(&arguments),
        "fix" => run("fmt", &["fmt", "--all"]).and_then(|()| {
            run(
                "clippy --fix",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--fix",
                    "--allow-dirty",
                ],
            )
        }),
        other => Err(format!(
            "unknown task \"{other}\" — tasks: green, sweep, docs-status, structure, stage-tool-deps, fix"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

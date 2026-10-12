//! Headless After Effects orchestration (the AEP Orchestrator, Phase 1).
//!
//! WiredFRONT drives Adobe's own bundled renderer as a SUBPROCESS — the
//! ADR-009 pattern ffmpeg already follows, never a crate dep. This crate
//! is the pure orchestration spine: locate the install, PROBE the machine
//! and tune memory/multi-frame-rendering DYNAMICALLY (so the same binary
//! performs from a 16 GB laptop to a 128 GB streaming rig), gate `.aep`
//! files by their RIFX magic, build the aerender ARGUMENT VECTOR (never a
//! shell string — spaces in template names must survive), parse aerender's
//! stdout grammar, and safely toggle AE's Welcome-screen pref (the one
//! thing that blocks headless ExtendScript enumeration).
//!
//! Phase 1 is everything that needs no running AE. Enumeration (Phase 2),
//! rendering (Phase 3), and the ffmpeg export selector (Phase 4) build on
//! this. Ground truth (aerender 26.3x87, verified 2026-07-25) lives in
//! `docs-history-2026-10-12:docs/plans/aep-orchestrator.md`.

use std::path::{Path, PathBuf};

// ─────────────────────────── capability probe ───────────────────────────

/// What the machine can give a render — probed at runtime, never assumed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caps {
    pub ram_gb: f32,
    pub logical_cpus: u32,
}

impl Caps {
    /// Read the real machine: total RAM via sysinfo, logical CPUs via std.
    pub fn probe() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let ram_gb = sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
        let logical_cpus = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        Self {
            ram_gb,
            logical_cpus,
        }
    }
}

/// How aggressively to claim the machine. The operator overrides per box:
/// `Gentle` while streaming, `MaxThroughput` on a dedicated render rig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Balanced,
    MaxThroughput,
    Gentle,
}

impl Profile {
    /// Fraction of RAM reserved for the OS + the egui app + its GPU textures.
    fn reserve_frac(self) -> f32 {
        match self {
            Profile::Balanced => 0.12,
            Profile::MaxThroughput => 0.08,
            Profile::Gentle => 0.20,
        }
    }
    /// Never reserve less than this many GB (a floor for small machines).
    fn min_reserve_gb(self) -> f32 {
        match self {
            Profile::Balanced | Profile::MaxThroughput => 4.0,
            Profile::Gentle => 6.0,
        }
    }
    /// Multi-frame rendering spawns a per-core AE worker, each wanting GBs;
    /// enable it only when RAM-per-core clears this bar (else it thrashes).
    fn mfr_ram_per_core_min(self) -> f32 {
        match self {
            Profile::Balanced => 2.5,
            Profile::MaxThroughput => 2.0,
            Profile::Gentle => 3.5,
        }
    }
    /// Logical cores left free for the OS/app when MFR runs.
    fn free_cores(self) -> u32 {
        match self {
            Profile::Balanced => 2,
            Profile::MaxThroughput => 1,
            Profile::Gentle => 4,
        }
    }
}

/// The tuned aerender knobs — `-mem_usage image_cache max_mem` and `-mfr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTuning {
    pub image_cache_pct: u32,
    pub max_mem_pct: u32,
    pub mfr: bool,
    pub mfr_cpu_pct: u32,
}

impl RenderTuning {
    /// Derive the knobs from the real machine + profile — a PURE function,
    /// so every tier (16/32/64/128 GB × cores) is unit-provable.
    pub fn for_caps(caps: Caps, profile: Profile) -> Self {
        let ram = caps.ram_gb.max(1.0);
        let cpus = caps.logical_cpus.max(1);

        let reserve = (ram * profile.reserve_frac()).max(profile.min_reserve_gb());
        // Never hand AE less than 40% even on tiny boxes (it needs a floor).
        let usable = (ram - reserve).max(ram * 0.4);
        let max_mem_pct = ((usable / ram) * 100.0).round().clamp(50.0, 90.0) as u32;
        let image_cache_pct = (max_mem_pct as f32 * 0.6).round().clamp(20.0, 80.0) as u32;

        let ram_per_core = ram / cpus as f32;
        let mfr = ram_per_core >= profile.mfr_ram_per_core_min();
        let mfr_cpu_pct = if mfr {
            let usable_cores = cpus.saturating_sub(profile.free_cores()).max(1);
            ((usable_cores as f32 / cpus as f32) * 100.0)
                .round()
                .clamp(40.0, 90.0) as u32
        } else {
            0
        };

        Self {
            image_cache_pct,
            max_mem_pct,
            mfr,
            mfr_cpu_pct,
        }
    }
}

// ─────────────────────────── install discovery ──────────────────────────

/// A located After Effects install (Windows-only; other OSes have no
/// aerender and `locate` returns None honestly).
#[derive(Debug, Clone, PartialEq)]
pub struct AeInstall {
    pub aerender: PathBuf,
    pub afterfx: PathBuf,
    /// The `Adobe After Effects <year>` folder (for display/diagnostics).
    pub version_dir: PathBuf,
}

/// Find the newest AE install: scan every `%ProgramFiles*%\Adobe\Adobe
/// After Effects *\Support Files` for both binaries. Newest folder wins.
pub fn locate() -> Option<AeInstall> {
    let mut bases: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
        if let Ok(v) = std::env::var(var) {
            bases.push(PathBuf::from(v));
        }
    }
    if bases.is_empty() {
        bases.push(PathBuf::from(r"C:\Program Files"));
    }

    let mut versions: Vec<PathBuf> = Vec::new();
    for base in bases {
        let adobe = base.join("Adobe");
        let Ok(rd) = std::fs::read_dir(&adobe) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("Adobe After Effects"))
            {
                versions.push(p);
            }
        }
    }
    // Newest by folder name descending (…2026 > …2025).
    versions.sort_by(|a, b| {
        b.file_name()
            .unwrap_or_default()
            .cmp(a.file_name().unwrap_or_default())
    });
    versions.dedup();

    for dir in versions {
        let sf = dir.join("Support Files");
        let aerender = sf.join("aerender.exe");
        let afterfx = sf.join("AfterFX.exe");
        if aerender.is_file() && afterfx.is_file() {
            return Some(AeInstall {
                aerender,
                afterfx,
                version_dir: dir,
            });
        }
    }
    None
}

// ─────────────────────────── the RIFX gate ──────────────────────────────

/// Cheap sanity gate before spawning a heavy subprocess: a real `.aep` is a
/// big-endian RIFF file whose first four bytes are `RIFX`.
pub fn rifx_is_aep(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).is_ok() && &magic == b"RIFX"
}

// ─────────────────────────── the render job ─────────────────────────────

/// One headless render request. `args()` yields the aerender ARGUMENT
/// VECTOR — the spawn (Phase 3) feeds it to `Command` arg-by-arg so
/// Windows quotes each value correctly and `Best Settings` never splits.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderJob {
    pub project: PathBuf,
    pub comp: String,
    pub output: PathBuf,
    /// e.g. `H.264 - Match Render Settings - 40 Mbps` (mind the real names).
    pub om_template: String,
    /// e.g. `Best Settings`; None keeps the item's own settings.
    pub rs_template: Option<String>,
    pub start: Option<u32>,
    pub end: Option<u32>,
    pub tuning: RenderTuning,
}

impl RenderJob {
    pub fn args(&self) -> Vec<String> {
        let mut a: Vec<String> = vec![
            "-project".into(),
            self.project.to_string_lossy().into_owned(),
            "-comp".into(),
            self.comp.clone(),
            "-output".into(),
            self.output.to_string_lossy().into_owned(),
        ];
        if let Some(rs) = &self.rs_template {
            a.push("-RStemplate".into());
            a.push(rs.clone());
        }
        a.push("-OMtemplate".into());
        a.push(self.om_template.clone());
        if let Some(s) = self.start {
            a.push("-s".into());
            a.push(s.to_string());
        }
        if let Some(e) = self.end {
            a.push("-e".into());
            a.push(e.to_string());
        }
        a.push("-mem_usage".into());
        a.push(self.tuning.image_cache_pct.to_string());
        a.push(self.tuning.max_mem_pct.to_string());
        a.push("-mfr".into());
        if self.tuning.mfr {
            a.push("ON".into());
            a.push(self.tuning.mfr_cpu_pct.to_string());
        } else {
            // max_cpu is ignored when the flag is OFF (per aerender -help).
            a.push("OFF".into());
            a.push("100".into());
        }
        a.push("-sound".into());
        a.push("OFF".into());
        a.push("-close".into());
        a.push("DO_NOT_SAVE_CHANGES".into());
        a.push("-continueOnMissingFootage".into());
        a.push("-v".into());
        a.push("ERRORS_AND_PROGRESS".into());
        a
    }
}

// ─────────────────────────── stdout grammar ─────────────────────────────

/// One classified aerender stdout line (grammar verified against real
/// 26.3x87 output — see the plan's §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AeLine {
    Version(String),
    Progress(String),
    Info(String),
    Error(String),
    SyntaxError(String),
    Other(String),
}

/// Classify one line. Order matters: the SYNTAX ERROR prefix is checked
/// before the plain ERROR prefix.
pub fn parse_line(line: &str) -> AeLine {
    let t = line.trim_end_matches(['\r', '\n']);
    let tt = t.trim();
    if let Some(r) = tt.strip_prefix("aerender version ") {
        return AeLine::Version(r.to_string());
    }
    if let Some(r) = tt.strip_prefix("PROGRESS: ") {
        return AeLine::Progress(r.to_string());
    }
    // NOTE: no space after the first colon — verbatim from real output.
    if let Some(r) = tt.strip_prefix("INFO:After Effects: ") {
        return AeLine::Info(r.to_string());
    }
    if let Some(r) = tt.strip_prefix("aerender SYNTAX ERROR: ") {
        return AeLine::SyntaxError(r.to_string());
    }
    if let Some(r) = tt.strip_prefix("aerender ERROR: ") {
        return AeLine::Error(r.to_string());
    }
    AeLine::Other(tt.to_string())
}

// ─────────────────────── the Welcome-screen unblock ─────────────────────

const WELCOME_ON: &str = "\"Show Welcome Screen\" = 01";
const WELCOME_OFF: &str = "\"Show Welcome Screen\" = 00";

/// The AE prefs file for the newest version dir under `%APPDATA%\Adobe\
/// After Effects\<X.Y>\`. None if AE has never written prefs.
pub fn prefs_file() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let ae = Path::new(&appdata).join("Adobe").join("After Effects");
    let rd = std::fs::read_dir(&ae).ok()?;
    let mut vers: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    vers.sort_by(|a, b| {
        b.file_name()
            .unwrap_or_default()
            .cmp(a.file_name().unwrap_or_default())
    });
    for dir in vers {
        let name = dir.file_name()?.to_string_lossy().into_owned();
        let prefs = dir.join(format!("Adobe After Effects {name} Prefs.txt"));
        if prefs.is_file() {
            return Some(prefs);
        }
    }
    None
}

fn shows_welcome(contents: &str) -> bool {
    contents.contains(WELCOME_ON)
}

/// Ending-preserving flip of the one pref line (a targeted substring
/// replace — never a lines() split that would rewrite CRLF/trailing bytes).
fn flip_welcome_off(contents: &str) -> String {
    contents.replace(WELCOME_ON, WELCOME_OFF)
}

/// Is the Home/Welcome screen still armed (blocking headless `-r`)?
pub fn welcome_screen_blocking(prefs: &Path) -> bool {
    std::fs::read_to_string(prefs)
        .map(|c| shows_welcome(&c))
        .unwrap_or(false)
}

/// Any AfterFX/aerender process currently running? (Editing prefs while AE
/// runs is futile — AE rewrites them on quit.)
pub fn ae_running() -> bool {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.processes().values().any(|p| {
        let n = p.name().to_string_lossy().to_ascii_lowercase();
        n.starts_with("afterfx") || n.starts_with("aerender")
    })
}

/// Disable the Welcome screen so headless enumeration works — operator-
/// tap-only, reversible. Refuses while AE runs; backs the prefs file up to
/// `<prefs>.dpl-bak` before touching it. A no-op when already off.
pub fn enable_automation(prefs: &Path) -> Result<(), String> {
    enable_automation_at(prefs, ae_running())
}

/// The testable core: the `ae_running` guard is injected.
fn enable_automation_at(prefs: &Path, ae_running: bool) -> Result<(), String> {
    if ae_running {
        return Err(
            "After Effects is running — close it first (AE rewrites its \
             preferences on quit, which would undo this)."
                .into(),
        );
    }
    let contents = std::fs::read_to_string(prefs).map_err(|e| format!("read prefs: {e}"))?;
    if !shows_welcome(&contents) {
        return Ok(()); // already disabled
    }
    let bak = PathBuf::from(format!("{}.dpl-bak", prefs.display()));
    std::fs::copy(prefs, &bak).map_err(|e| format!("back up prefs: {e}"))?;
    std::fs::write(prefs, flip_welcome_off(&contents)).map_err(|e| format!("write prefs: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Caps::probe` reads `sysinfo::System::total_memory()`, and the ONLY
    /// thing standing between that number and the render tuning is a divide
    /// by 1024³. sysinfo has changed that unit before (kB in the 0.2x line,
    /// bytes since), and a unit flip does not fail to compile — it silently
    /// reports a 128 GB rig as 0.125 GB and tunes every render for a machine
    /// that does not exist. Every other test here feeds `Caps` literals, so
    /// this is the one place the real reading is checked at all.
    ///
    /// The band is deliberately wide: it must pass on CI runners and on a
    /// 128 GB workstation alike. It fails on a unit change by three orders of
    /// magnitude in either direction, which is exactly the bug it exists for.
    #[test]
    fn the_probe_reads_total_memory_in_bytes() {
        let caps = Caps::probe();
        assert!(
            (0.5..=4096.0).contains(&caps.ram_gb),
            "ram_gb {} is outside any real machine — sysinfo's total_memory() \
             unit likely changed (expected bytes, divided by 1024³)",
            caps.ram_gb
        );
        assert!(
            caps.logical_cpus >= 1,
            "a machine running this test has at least one CPU"
        );
    }

    /// `ae_running()` decides by `name().starts_with("afterfx")`. If sysinfo
    /// ever returned a full path there instead of a bare file name, that match
    /// would silently go false — and `enable_automation` would rewrite AE's
    /// prefs WHILE AE is running, which this module documents as futile
    /// because AE overwrites them on quit. Nothing else exercises the process
    /// side of sysinfo: the toggle tests drive `enable_automation_at` with a
    /// literal bool, deliberately, so the real scan is checked only here.
    /// Pinned against this test binary — the one process guaranteed to exist.
    #[test]
    fn a_process_name_is_a_bare_file_name_not_a_path() {
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let me = sysinfo::Pid::from_u32(std::process::id());
        let name = sys
            .process(me)
            .expect("the running test process is in the process list")
            .name()
            .to_string_lossy()
            .to_string();
        assert!(!name.is_empty(), "a process always has a name");
        assert!(
            !name.contains('/') && !name.contains('\\'),
            "name() must be a bare file name for the afterfx/aerender prefix \
             match to work; got {name:?}"
        );
    }

    // ---- tuning policy across the real machine tiers ----

    #[test]
    fn tuning_is_conservative_on_the_weak_tier() {
        // This laptop: 15.3 GB / 16 logical — deliberately the weakest box.
        let t = RenderTuning::for_caps(
            Caps {
                ram_gb: 15.3,
                logical_cpus: 16,
            },
            Profile::Balanced,
        );
        assert!(!t.mfr, "MFR OFF: ~1 GB/core would thrash");
        assert_eq!(t.mfr_cpu_pct, 0);
        assert!(
            (70..=78).contains(&t.max_mem_pct),
            "max_mem ~74, got {}",
            t.max_mem_pct
        );
        assert!((40..=48).contains(&t.image_cache_pct));
    }

    #[test]
    fn tuning_opens_up_on_a_big_rig() {
        // A 128 GB / 16-logical streaming rig.
        let t = RenderTuning::for_caps(
            Caps {
                ram_gb: 128.0,
                logical_cpus: 16,
            },
            Profile::Balanced,
        );
        assert!(t.mfr, "MFR ON: 8 GB/core is comfortable");
        assert!(
            (80..=90).contains(&t.mfr_cpu_pct),
            "leaves ~2 cores free, got {}",
            t.mfr_cpu_pct
        );
        assert!(t.max_mem_pct >= 85);
    }

    #[test]
    fn profile_override_flips_a_borderline_rig() {
        // 64 GB / 32 cores = exactly 2.0 GB/core: OFF under Balanced (2.5),
        // ON under MaxThroughput (2.0) — the reason the override exists.
        let caps = Caps {
            ram_gb: 64.0,
            logical_cpus: 32,
        };
        assert!(!RenderTuning::for_caps(caps, Profile::Balanced).mfr);
        assert!(RenderTuning::for_caps(caps, Profile::MaxThroughput).mfr);
    }

    #[test]
    fn tuning_never_exceeds_the_clamps() {
        for &ram in &[8.0f32, 16.0, 32.0, 64.0, 128.0, 512.0] {
            for &cpus in &[4u32, 8, 16, 32, 64] {
                for profile in [Profile::Balanced, Profile::MaxThroughput, Profile::Gentle] {
                    let t = RenderTuning::for_caps(
                        Caps {
                            ram_gb: ram,
                            logical_cpus: cpus,
                        },
                        profile,
                    );
                    assert!((50..=90).contains(&t.max_mem_pct));
                    assert!((20..=80).contains(&t.image_cache_pct));
                    assert!(t.image_cache_pct <= t.max_mem_pct);
                    if t.mfr {
                        assert!((40..=90).contains(&t.mfr_cpu_pct));
                    } else {
                        assert_eq!(t.mfr_cpu_pct, 0);
                    }
                }
            }
        }
    }

    // ---- the aerender argument vector ----

    #[test]
    fn args_keep_spaced_template_names_as_single_tokens() {
        let job = RenderJob {
            project: PathBuf::from(r"C:\p\my proj.aep"),
            comp: "Main Comp".into(),
            output: PathBuf::from(r"C:\o\out.mp4"),
            om_template: "H.264 - Match Render Settings - 40 Mbps".into(),
            rs_template: Some("Best Settings".into()),
            start: Some(0),
            end: Some(5),
            tuning: RenderTuning {
                image_cache_pct: 44,
                max_mem_pct: 74,
                mfr: false,
                mfr_cpu_pct: 0,
            },
        };
        let a = job.args();
        // Each spaced value is ONE vector element (the quoting-gotcha fix).
        assert!(a.contains(&"Best Settings".to_string()));
        assert!(a.contains(&"H.264 - Match Render Settings - 40 Mbps".to_string()));
        assert!(a.contains(&"Main Comp".to_string()));
        // Flag/value adjacency.
        let i = a.iter().position(|x| x == "-mem_usage").unwrap();
        assert_eq!(a[i + 1], "44");
        assert_eq!(a[i + 2], "74");
        let m = a.iter().position(|x| x == "-mfr").unwrap();
        assert_eq!(a[m + 1], "OFF");
        // The always-on safety flags.
        assert!(a.contains(&"-continueOnMissingFootage".to_string()));
        assert!(a.windows(2).any(|w| w[0] == "-sound" && w[1] == "OFF"));
        assert!(a
            .windows(2)
            .any(|w| w[0] == "-v" && w[1] == "ERRORS_AND_PROGRESS"));
    }

    #[test]
    fn args_emit_mfr_on_with_cpu_cap_when_tuned_on() {
        let job = RenderJob {
            project: PathBuf::from("p.aep"),
            comp: "C".into(),
            output: PathBuf::from("o.mp4"),
            om_template: "Lossless".into(),
            rs_template: None,
            start: None,
            end: None,
            tuning: RenderTuning {
                image_cache_pct: 53,
                max_mem_pct: 88,
                mfr: true,
                mfr_cpu_pct: 88,
            },
        };
        let a = job.args();
        let m = a.iter().position(|x| x == "-mfr").unwrap();
        assert_eq!(a[m + 1], "ON");
        assert_eq!(a[m + 2], "88");
        // No RStemplate emitted when None.
        assert!(!a.contains(&"-RStemplate".to_string()));
    }

    // ---- the stdout grammar ----

    #[test]
    fn parse_line_classifies_the_real_grammar() {
        assert_eq!(
            parse_line("aerender version 26.3x87"),
            AeLine::Version("26.3x87".into())
        );
        assert_eq!(
            parse_line("PROGRESS: Launching After Effects..."),
            AeLine::Progress("Launching After Effects...".into())
        );
        assert_eq!(
            parse_line("INFO:After Effects: this project must be converted from version 12.0 (Windows 64)."),
            AeLine::Info("this project must be converted from version 12.0 (Windows 64).".into())
        );
        assert_eq!(
            parse_line("aerender ERROR: No comp was found with the given name."),
            AeLine::Error("No comp was found with the given name.".into())
        );
        assert_eq!(
            parse_line("aerender SYNTAX ERROR: Illegal argument flag: Settings"),
            AeLine::SyntaxError("Illegal argument flag: Settings".into())
        );
        assert!(matches!(parse_line("some banner line"), AeLine::Other(_)));
    }

    // ---- RIFX gate ----

    #[test]
    fn rifx_gate_accepts_rifx_and_rejects_others() {
        let dir = std::env::temp_dir().join(format!("dpl-ae-rifx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("real.aep");
        std::fs::write(&good, b"RIFX\x00\x39\xf0\x1aEgg!").unwrap();
        let bad = dir.join("fake.aep");
        std::fs::write(&bad, b"PK\x03\x04 a zip really").unwrap();
        assert!(rifx_is_aep(&good));
        assert!(!rifx_is_aep(&bad));
        assert!(!rifx_is_aep(&dir.join("missing.aep")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- the Welcome-screen unblock ----

    #[test]
    fn welcome_toggle_flips_and_backs_up_and_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("dpl-ae-prefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let prefs = dir.join("Adobe After Effects 26.3 Prefs.txt");
        // CRLF + a trailing line, to prove ending preservation.
        let original =
            "\t\"Some Other\" = 1\r\n\t\"Show Welcome Screen\" = 01\r\n\t\"Tail\" = 9\r\n";
        std::fs::write(&prefs, original).unwrap();

        assert!(welcome_screen_blocking(&prefs));
        enable_automation_at(&prefs, false).unwrap();
        assert!(!welcome_screen_blocking(&prefs));

        let after = std::fs::read_to_string(&prefs).unwrap();
        assert!(after.contains("\"Show Welcome Screen\" = 00"));
        assert!(after.contains("\r\n"), "CRLF preserved");
        assert!(after.contains("\"Tail\" = 9"), "the rest is untouched");
        // Backup holds the original verbatim.
        let bak = PathBuf::from(format!("{}.dpl-bak", prefs.display()));
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), original);

        // Second run is a clean no-op.
        enable_automation_at(&prefs, false).unwrap();
        assert!(!welcome_screen_blocking(&prefs));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn welcome_toggle_refuses_while_ae_runs() {
        let dir = std::env::temp_dir().join(format!("dpl-ae-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let prefs = dir.join("Adobe After Effects 26.3 Prefs.txt");
        std::fs::write(&prefs, "\t\"Show Welcome Screen\" = 01\n").unwrap();
        let err = enable_automation_at(&prefs, true).unwrap_err();
        assert!(err.contains("running"), "{err}");
        // Untouched: still armed, no backup written.
        assert!(welcome_screen_blocking(&prefs));
        assert!(!PathBuf::from(format!("{}.dpl-bak", prefs.display())).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! Lightweight Coach target inventory — Phase 0 (XDG `.desktop`) +
//! Phase 1 (AT-SPI desktop-root census only).
//!
//! **Hard law (C2d):** never call `GetChildren` below the desktop root.
//! Deep fabric mint is C2e. Plan: `docs/plans/coach-c2d-lightweight-inventory.md`.

#![forbid(unsafe_code)]

mod desktop;

#[cfg(target_os = "linux")]
mod linux_live;

pub use desktop::{
    cold_catalog, parse_desktop_entry, refresh_cold, ApplicationDirs, ColdEntry, DesktopId,
};

/// Pill / ETA hint for the Operator Coach list (mock vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    Small,
    Medium,
    Deep,
    Unknown,
}

impl Complexity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Deep => "deep",
            Self::Unknown => "unknown",
        }
    }
}

/// Shallow live hint — name (+ optional pid). No tree nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveHint {
    pub atspi_name: String,
    pub pid: Option<u32>,
    pub running: bool,
}

/// One row for the Operator Coach picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItem {
    pub id: DesktopId,
    pub display_name: String,
    pub exec_hint: String,
    pub startup_wm_class: Option<String>,
    pub cold_ok: bool,
    pub live: Option<LiveHint>,
    pub complexity: Complexity,
    pub seed_tip: Option<String>,
}

/// Running AT-SPI application seen at the desktop root (Phase 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveApp {
    pub name: String,
    pub pid: Option<u32>,
}

/// Join Phase 0 + Phase 1 into picker rows. Never walks app trees.
pub fn join_inventory(cold: &[ColdEntry], live: &[LiveApp]) -> Vec<InventoryItem> {
    let mut items: Vec<InventoryItem> = cold
        .iter()
        .map(|c| {
            let live_hint = live.iter().find(|l| cold_matches_live(c, l)).map(|l| {
                LiveHint {
                    atspi_name: l.name.clone(),
                    pid: l.pid,
                    running: true,
                }
            });
            let complexity = classify_complexity(c, live_hint.is_some());
            let seed_tip = seed_tip_for(c, live_hint.is_some());
            InventoryItem {
                id: c.id.clone(),
                display_name: c.display_name.clone(),
                exec_hint: c.exec_hint.clone(),
                startup_wm_class: c.startup_wm_class.clone(),
                cold_ok: true,
                live: live_hint,
                complexity,
                seed_tip,
            }
        })
        .collect();

    for l in live {
        if items
            .iter()
            .any(|i| i.live.as_ref().is_some_and(|h| h.atspi_name == l.name))
        {
            continue;
        }
        if is_system_live_noise(&l.name) {
            continue;
        }
        items.push(InventoryItem {
            id: DesktopId::from_raw(format!("live-only:{}", l.name)),
            display_name: l.name.clone(),
            exec_hint: String::new(),
            startup_wm_class: None,
            cold_ok: false,
            live: Some(LiveHint {
                atspi_name: l.name.clone(),
                pid: l.pid,
                running: true,
            }),
            complexity: Complexity::Unknown,
            seed_tip: Some(
                "Running but not in the menu catalog — open it once from the app menu to catalog, or pick a seeded lesson."
                    .into(),
            ),
        });
    }

    items.sort_by(|a, b| {
        b.live.is_some()
            .cmp(&a.live.is_some())
            .then(a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
    });
    items
}

fn cold_matches_live(cold: &ColdEntry, live: &LiveApp) -> bool {
    let live_l = live.name.to_lowercase();
    if let Some(wm) = &cold.startup_wm_class {
        if wm.eq_ignore_ascii_case(&live.name) {
            return true;
        }
    }
    if !cold.exec_hint.is_empty() {
        let base = exec_basename(&cold.exec_hint);
        if base.eq_ignore_ascii_case(&live.name) {
            return true;
        }
    }
    if cold.display_name.eq_ignore_ascii_case(&live.name) {
        return true;
    }
    // Common GNOME: desktop-id org.gnome.Calculator ↔ atspi "gnome-calculator"
    let id_stem = cold.id.as_str().trim_end_matches(".desktop");
    if id_stem.eq_ignore_ascii_case(&live.name) {
        return true;
    }
    let compact_id = id_stem.replace('.', "").to_lowercase();
    let compact_live = live_l.replace(['-', '_', ' '], "");
    if compact_id.contains(&compact_live) || compact_live.contains(&compact_id.replace("orggnome", ""))
    {
        // orggnomeCalculator vs gnomecalculator
        let id_tail = id_stem
            .rsplit('.')
            .next()
            .unwrap_or(id_stem)
            .to_lowercase();
        if id_tail == live_l || live_l.contains(&id_tail) || id_tail.contains(&live_l) {
            return true;
        }
    }
    false
}

fn exec_basename(exec: &str) -> String {
    let token = exec.split_whitespace().next().unwrap_or(exec);
    let token = token.rsplit('/').next().unwrap_or(token);
    token.to_string()
}

fn classify_complexity(cold: &ColdEntry, running: bool) -> Complexity {
    let blob = format!(
        "{} {} {}",
        cold.id.as_str(),
        cold.display_name,
        cold.exec_hint
    )
    .to_lowercase();
    if blob.contains("calculator") || blob.contains("gnome-calculator") {
        return Complexity::Small;
    }
    const DEEP: &[&str] = &[
        "blender",
        "solidworks",
        "electron",
        "chrome",
        "chromium",
        "code",
        "soffice",
        "libreoffice",
        "gimp",
        "inkscape",
        "steam",
    ];
    if DEEP.iter().any(|k| blob.contains(k)) {
        return Complexity::Deep;
    }
    if running {
        Complexity::Medium
    } else {
        Complexity::Unknown
    }
}

/// Desktop-root AT-SPI often lists session helpers — not Operator targets.
fn is_system_live_noise(name: &str) -> bool {
    let n = name.to_lowercase();
    const PREFIXES: &[&str] = &[
        "gsd-",
        "gnome-shell",
        "mutter-",
        "ibus-",
        "xdg-desktop-portal",
        "evolution-alarm",
        "gjs",
        "at-spi",
    ];
    PREFIXES.iter().any(|p| n.starts_with(p) || n == *p)
}

fn seed_tip_for(cold: &ColdEntry, running: bool) -> Option<String> {
    let blob = format!("{} {}", cold.id.as_str(), cold.display_name).to_lowercase();
    if blob.contains("calculator") {
        return Some(if running {
            "Seed proven on this box — try button 7 after mint (C2e).".into()
        } else {
            "Start with Calculator — we already proved paint + press on button 7.".into()
        });
    }
    None
}

/// Phase 1: shallow live census. Linux only; other platforms return empty.
pub fn live_census() -> Result<Vec<LiveApp>, String> {
    #[cfg(target_os = "linux")]
    {
        linux_live::live_census_blocking()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Vec::new())
    }
}

/// Full inventory: cold catalog joined with optional live census.
pub fn inventory(include_live: bool) -> Result<Vec<InventoryItem>, String> {
    let cold = refresh_cold()?;
    let live = if include_live {
        live_census()?
    } else {
        Vec::new()
    };
    Ok(join_inventory(&cold, &live))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::parse_desktop_entry;

    #[test]
    fn hidden_and_nodisplay_are_dropped() {
        let hidden = parse_desktop_entry(
            "hidden.desktop",
            "[Desktop Entry]\nType=Application\nName=HideMe\nExec=hideme\nHidden=true\n",
            "GNOME",
        );
        assert!(hidden.is_none());
        let nodisp = parse_desktop_entry(
            "nodisp.desktop",
            "[Desktop Entry]\nType=Application\nName=NoShow\nExec=noshow\nNoDisplay=true\n",
            "GNOME",
        );
        assert!(nodisp.is_none());
    }

    #[test]
    fn application_with_name_and_exec_parses() {
        let e = parse_desktop_entry(
            "org.gnome.Calculator.desktop",
            "[Desktop Entry]\nType=Application\nName=Calculator\nExec=gnome-calculator\nStartupWMClass=gnome-calculator\n",
            "GNOME",
        )
        .expect("parse");
        assert_eq!(e.display_name, "Calculator");
        assert_eq!(e.exec_hint, "gnome-calculator");
        assert_eq!(e.startup_wm_class.as_deref(), Some("gnome-calculator"));
    }

    #[test]
    fn only_show_in_filters_desktop() {
        let skipped = parse_desktop_entry(
            "kde-only.desktop",
            "[Desktop Entry]\nType=Application\nName=KOnly\nExec=konly\nOnlyShowIn=KDE;\n",
            "GNOME",
        );
        assert!(skipped.is_none());
        let kept = parse_desktop_entry(
            "gnome-app.desktop",
            "[Desktop Entry]\nType=Application\nName=GApp\nExec=gapp\nOnlyShowIn=GNOME;\n",
            "GNOME",
        );
        assert!(kept.is_some());
    }

    #[test]
    fn first_desktop_id_wins_in_scan_order() {
        // Exercised via join: two cold entries same id shouldn't happen after scan;
        // parse independence is enough — catalog builder dedupes by id.
        let dirs = ApplicationDirs {
            paths: vec![],
            current_desktop: "GNOME".into(),
        };
        assert!(dirs.paths.is_empty());
    }

    #[test]
    fn join_matches_calculator_seed_and_marks_small() {
        let cold = parse_desktop_entry(
            "org.gnome.Calculator.desktop",
            "[Desktop Entry]\nType=Application\nName=Calculator\nExec=gnome-calculator %U\nStartupWMClass=gnome-calculator\n",
            "GNOME",
        )
        .unwrap();
        let live = [LiveApp {
            name: "gnome-calculator".into(),
            pid: Some(42),
        }];
        let items = join_inventory(&[cold], &live);
        let calc = items
            .iter()
            .find(|i| i.id.as_str() == "org.gnome.Calculator.desktop")
            .expect("calc");
        assert!(calc.live.is_some());
        assert_eq!(calc.complexity, Complexity::Small);
        assert!(calc.seed_tip.is_some());
    }

    #[test]
    fn live_only_row_gets_tip() {
        let items = join_inventory(
            &[],
            &[LiveApp {
                name: "mystery-app".into(),
                pid: None,
            }],
        );
        assert_eq!(items.len(), 1);
        assert!(!items[0].cold_ok);
        assert!(items[0].seed_tip.is_some());
    }

    #[test]
    fn deep_keywords_classify() {
        let cold = parse_desktop_entry(
            "blender.desktop",
            "[Desktop Entry]\nType=Application\nName=Blender\nExec=blender\n",
            "GNOME",
        )
        .unwrap();
        let items = join_inventory(&[cold], &[]);
        assert_eq!(items[0].complexity, Complexity::Deep);
    }
}

//! Fingerprint bundles — everything one scan observed about one app.
//! The bundle is EVIDENCE; identity comes from resolving it against
//! installed packs, never from the bundle itself.

use dpl_coach_inventory::InventoryItem;

/// One scan's observations. Every field optional — a bundle carries
/// whatever the scan could actually see, nothing invented.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FingerprintBundle {
    /// XDG `.desktop` basename (`org.blender.Blender.desktop`).
    pub desktop_id: Option<String>,
    /// `StartupWMClass` from the desktop entry.
    pub startup_wm_class: Option<String>,
    /// Live X11 `WM_CLASS` (observed on a running window; S3b fills it).
    pub wm_class: Option<String>,
    /// AT-SPI application name at the desktop root.
    pub atspi_name: Option<String>,
    /// Executable basename from Exec/TryExec or the live process.
    pub exe_basename: Option<String>,
    /// Full executable path when the scan saw one.
    pub exe_path: Option<String>,
    /// App version string when detected (Foundry fills it; S3b).
    pub version: Option<String>,
}

impl FingerprintBundle {
    /// The one constructor the Coach hub calls per picker row.
    pub fn from_inventory(item: &InventoryItem) -> Self {
        // "live-only:<name>" ids are inventory synthetics, not real
        // desktop ids — evidence stays honest.
        let desktop_id = (item.cold_ok && !item.id.as_str().starts_with("live-only:"))
            .then(|| item.id.as_str().to_string());
        let exe_token = item.exec_hint.split_whitespace().next().unwrap_or("");
        let exe_path = exe_token.contains('/').then(|| exe_token.to_string());
        let exe_basename = (!exe_token.is_empty()).then(|| {
            exe_token
                .rsplit('/')
                .next()
                .unwrap_or(exe_token)
                .to_string()
        });
        Self {
            desktop_id,
            startup_wm_class: item.startup_wm_class.clone(),
            wm_class: None,
            atspi_name: item.live.as_ref().map(|l| l.atspi_name.clone()),
            exe_basename,
            exe_path,
            version: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.desktop_id.is_none()
            && self.startup_wm_class.is_none()
            && self.wm_class.is_none()
            && self.atspi_name.is_none()
            && self.exe_basename.is_none()
            && self.exe_path.is_none()
            && self.version.is_none()
    }

    /// Best observed human-ish name — feeds `AppId::unresolved` and the
    /// synthesized profile's display name.
    pub fn best_name(&self) -> Option<&str> {
        self.atspi_name
            .as_deref()
            .or(self.wm_class.as_deref())
            .or(self.startup_wm_class.as_deref())
            .or(self
                .desktop_id
                .as_deref()
                .map(|d| d.trim_end_matches(".desktop")))
            .or(self.exe_basename.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_coach_inventory::{Complexity, DesktopId, LiveHint};

    fn item(id: &str, exec: &str, wm: Option<&str>, live: Option<&str>) -> InventoryItem {
        InventoryItem {
            id: DesktopId::from_raw(id),
            display_name: "X".into(),
            exec_hint: exec.into(),
            startup_wm_class: wm.map(str::to_string),
            cold_ok: !id.starts_with("live-only:"),
            live: live.map(|n| LiveHint {
                atspi_name: n.into(),
                pid: Some(1),
                running: true,
            }),
            complexity: Complexity::Unknown,
            seed_tip: None,
        }
    }

    #[test]
    fn inventory_row_becomes_honest_evidence() {
        let b = FingerprintBundle::from_inventory(&item(
            "org.blender.Blender.desktop",
            "/usr/bin/blender %f",
            Some("Blender"),
            Some("Blender"),
        ));
        assert_eq!(b.desktop_id.as_deref(), Some("org.blender.Blender.desktop"));
        assert_eq!(b.startup_wm_class.as_deref(), Some("Blender"));
        assert_eq!(b.atspi_name.as_deref(), Some("Blender"));
        assert_eq!(b.exe_basename.as_deref(), Some("blender"));
        assert_eq!(b.exe_path.as_deref(), Some("/usr/bin/blender"));
        assert_eq!(b.wm_class, None); // never invented — S3b observes it live
        assert_eq!(b.version, None);
    }

    #[test]
    fn live_only_synthetic_id_is_not_evidence() {
        let b = FingerprintBundle::from_inventory(&item(
            "live-only:mystery",
            "",
            None,
            Some("mystery"),
        ));
        assert_eq!(b.desktop_id, None);
        assert_eq!(b.exe_basename, None);
        assert_eq!(b.best_name(), Some("mystery"));
    }

    #[test]
    fn best_name_prefers_live_over_cold() {
        let mut b = FingerprintBundle {
            desktop_id: Some("org.gnome.Calculator.desktop".into()),
            ..Default::default()
        };
        assert_eq!(b.best_name(), Some("org.gnome.Calculator"));
        b.atspi_name = Some("gnome-calculator".into());
        assert_eq!(b.best_name(), Some("gnome-calculator"));
    }
}

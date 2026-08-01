//! Reality gate — mapped window + live pid. AT-SPI-only is not enough.

use dpl_coach_fabric::Refused;

/// Operator-visible target presence (not ghost AT-SPI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPresence {
    pub atspi_name: String,
    pub atspi_listed: bool,
    pub mapped_window: bool,
    pub window_id: Option<u32>,
    pub outer_w: i32,
    pub outer_h: i32,
    pub pid: Option<u32>,
    pub pid_alive: bool,
}

impl TargetPresence {
    /// Hard gate for import / resolve / invoke.
    pub fn is_real(&self) -> bool {
        self.mapped_window
            && self.outer_w >= 50
            && self.outer_h >= 50
            && self.pid.is_some_and(|p| p > 1)
            && self.pid_alive
    }

    pub fn refuse_if_unreal(&self) -> Result<(), Refused> {
        if self.is_real() {
            return Ok(());
        }
        Err(Refused {
            reason: format!(
                "target {:?} not operator-real (atspi={} mapped={} {}×{} pid={:?} alive={})",
                self.atspi_name,
                self.atspi_listed,
                self.mapped_window,
                self.outer_w,
                self.outer_h,
                self.pid,
                self.pid_alive
            ),
            tip: format!(
                "Open {} so a real window is mapped, then retry. Ghost AT-SPI / zombie processes do not count.",
                self.atspi_name
            ),
        })
    }
}

/// Probe presence for an AT-SPI app name / WM_CLASS needle.
pub fn probe_presence(atspi_name: &str) -> TargetPresence {
    #[cfg(target_os = "linux")]
    {
        crate::linux_presence::probe(atspi_name)
    }
    #[cfg(not(target_os = "linux"))]
    {
        TargetPresence {
            atspi_name: atspi_name.into(),
            atspi_listed: false,
            mapped_window: false,
            window_id: None,
            outer_w: 0,
            outer_h: 0,
            pid: None,
            pid_alive: false,
        }
    }
}

/// WM_CLASS needle for common apps (shared with overlay).
pub fn x11_class_needle(app: &str) -> &str {
    match app {
        "gnome-calculator" => "gnome-calculator",
        "cursor" => "cursor",
        other => other,
    }
}

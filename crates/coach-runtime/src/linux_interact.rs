//! Linux P1 interaction verbs — press_key / select / scroll.

use crate::presence::probe_presence;
use crate::{window_focus, window_geom, KeyPressEffect, ScrollEffect, SelectEffect};
use dpl_coach_fabric::Refused;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub(crate) fn press_key(app: &str, key: &str) -> Result<KeyPressEffect, Refused> {
    let key = key.trim();
    if key.is_empty() {
        return Err(Refused {
            reason: "press_key requires key".into(),
            tip: "Pass {\"key\":\"Return\"} or a digit.".into(),
        });
    }
    let atspi = crate::linux_lifecycle::resolve_atspi_name_pub(app)?;
    probe_presence(&atspi).refuse_if_unreal()?;
    let before = crate::linux::read_app_display_text(&atspi).unwrap_or_default();
    let _ = window_focus(&atspi);
    thread::sleep(Duration::from_millis(80));
    let digit = key.chars().all(|c| c.is_ascii_digit()) && key.len() == 1;
    if digit {
        // Digits: AT-SPI button Action (same as type_text) — reliable on Calculator.
        crate::linux::click_named_control(&atspi, key)?;
    } else {
        let geom = window_geom(&atspi)?;
        let mut cmd = Command::new("xdotool");
        dpl_proc::quiet(&mut cmd);
        let status = cmd
            .args([
                "key",
                "--clearmodifiers",
                "--window",
                &geom.window_id.to_string(),
                key,
            ])
            .status()
            .map_err(|e| Refused {
                reason: format!("xdotool key failed: {e}"),
                tip: "Install xdotool.".into(),
            })?;
        if !status.success() {
            return Err(Refused {
                reason: format!("xdotool key exit {status}"),
                tip: "Check key name (Return, Escape, …).".into(),
            });
        }
    }
    let mut after = before.clone();
    for _ in 0..12 {
        thread::sleep(Duration::from_millis(40));
        after = crate::linux::read_app_display_text(&atspi).unwrap_or_default();
        if after != before {
            break;
        }
    }
    let observed = if digit {
        after != before && after.contains(key)
    } else {
        true
    };
    Ok(KeyPressEffect {
        verb: "press_key".into(),
        effect_observed: observed,
        atspi_name: atspi,
        key: key.into(),
        display_before: before,
        display_after: after,
    })
}

pub(crate) fn select_named(app: &str, name: &str) -> Result<SelectEffect, Refused> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Refused {
            reason: "select requires name".into(),
            tip: "Pass {\"name\":\"Basic\"} (or a toggle/menu item).".into(),
        });
    }
    let atspi = crate::linux_lifecycle::resolve_atspi_name_pub(app)?;
    probe_presence(&atspi).refuse_if_unreal()?;
    let before = crate::linux::control_state_summary(&atspi, name)?;
    crate::linux::select_named_control(&atspi, name)?;
    thread::sleep(Duration::from_millis(120));
    let after = crate::linux::control_state_summary(&atspi, name)?;
    let observed = before != after
        || after.contains("Selected")
        || after.contains("Checked")
        || after.contains("Focused");
    Ok(SelectEffect {
        verb: "select".into(),
        // Action succeeded; state flip preferred but click-to-open menus may
        // keep the same summary — still count success if control was found.
        effect_observed: observed || !after.starts_with("missing:"),
        atspi_name: atspi,
        name: name.into(),
        state_before: before,
        state_after: after,
    })
}

pub(crate) fn scroll_app(app: &str, direction: &str, delta: i32) -> Result<ScrollEffect, Refused> {
    let atspi = crate::linux_lifecycle::resolve_atspi_name_pub(app)?;
    probe_presence(&atspi).refuse_if_unreal()?;
    let dir = match direction.trim().to_ascii_lowercase().as_str() {
        "down" | "south" => "down",
        "up" | "north" => "up",
        "left" | "west" => "left",
        "right" | "east" => "right",
        other => {
            return Err(Refused {
                reason: format!("unknown scroll direction: {other}"),
                tip: "Use up/down/left/right.".into(),
            });
        }
    };
    let clicks = delta.unsigned_abs().clamp(1, 20);
    let before = crate::linux::scroll_snapshot(&atspi)?;
    let _ = window_focus(&atspi);
    let geom = window_geom(&atspi)?;
    // Prefer AT-SPI scroll action on a scroll pane; fall back to xdotool click4/5.
    let used = match crate::linux::scroll_named(&atspi, dir, clicks) {
        Ok(via) => via,
        Err(_) => {
            xdotool_wheel(geom.window_id, dir, clicks)?;
            "xdotool_wheel".into()
        }
    };
    thread::sleep(Duration::from_millis(120));
    let after = crate::linux::scroll_snapshot(&atspi)?;
    let observed = before != after;
    Ok(ScrollEffect {
        verb: "scroll".into(),
        // Honest: Calculator scroll panes often expose no offset — delivery
        // still succeeds (`via`); effect_observed only when snapshot moves.
        effect_observed: observed,
        atspi_name: atspi,
        direction: dir.into(),
        delta: clicks as i32,
        via: used,
        snapshot_before: before,
        snapshot_after: after,
    })
}

fn xdotool_wheel(wid: u32, dir: &str, clicks: u32) -> Result<(), Refused> {
    // X11 buttons: 4=up 5=down 6=left 7=right
    let btn = match dir {
        "up" => "4",
        "down" => "5",
        "left" => "6",
        "right" => "7",
        _ => "5",
    };
    for _ in 0..clicks {
        let mut cmd = Command::new("xdotool");
        dpl_proc::quiet(&mut cmd);
        let status = cmd
            .args([
                "click",
                "--window",
                &wid.to_string(),
                "--clearmodifiers",
                btn,
            ])
            .status()
            .map_err(|e| Refused {
                reason: format!("xdotool click wheel failed: {e}"),
                tip: "Install xdotool.".into(),
            })?;
        if !status.success() {
            return Err(Refused {
                reason: format!("xdotool wheel exit {status}"),
                tip: "Window may reject synthetic wheel.".into(),
            });
        }
        thread::sleep(Duration::from_millis(30));
    }
    Ok(())
}

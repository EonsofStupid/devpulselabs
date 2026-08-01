//! Linux app lifecycle verbs — open / close / wait_until / assert_state / type_text.

use crate::presence::probe_presence;
use crate::{window_focus, LifecycleEffect, TextTypeEffect, WaitAssertEffect};
use dpl_coach_fabric::Refused;
use dpl_coach_inventory::inventory;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, ClientMessageEvent, ConnectionExt as _, EventMask};
use x11rb::rust_connection::RustConnection;
use x11rb::CURRENT_TIME;

pub(crate) fn open_app(app: &str, timeout_ms: u64) -> Result<LifecycleEffect, Refused> {
    let atspi = resolve_atspi_name(app)?;
    let before = probe_presence(&atspi);
    if before.is_real() {
        return Ok(LifecycleEffect {
            verb: "open".into(),
            effect_observed: false,
            atspi_name: atspi,
            real_before: true,
            real_after: true,
            pid: before.pid,
        });
    }
    let exec = resolve_exec(app)?;
    let mut cmd = Command::new(&exec);
    dpl_proc::quiet(&mut cmd);
    cmd.spawn().map_err(|e| Refused {
        reason: format!("spawn {exec} failed: {e}"),
        tip: "Install the app or fix exec_hint in the desktop entry.".into(),
    })?;
    let after = wait_presence(&atspi, true, timeout_ms)?;
    Ok(LifecycleEffect {
        verb: "open".into(),
        effect_observed: after.is_real() && !before.is_real(),
        atspi_name: atspi,
        real_before: before.is_real(),
        real_after: after.is_real(),
        pid: after.pid,
    })
}

pub(crate) fn close_app(app: &str, timeout_ms: u64) -> Result<LifecycleEffect, Refused> {
    let atspi = resolve_atspi_name(app)?;
    let before = probe_presence(&atspi);
    if !before.is_real() {
        return Ok(LifecycleEffect {
            verb: "close".into(),
            effect_observed: false,
            atspi_name: atspi,
            real_before: false,
            real_after: false,
            pid: None,
        });
    }
    if let Some(wid) = before.window_id {
        let _ = wm_delete(wid);
    }
    let mut after = wait_presence(&atspi, false, timeout_ms.min(2500))?;
    if after.is_real() {
        if let Some(pid) = before.pid {
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
            after = wait_presence(&atspi, false, timeout_ms)?;
        }
    }
    Ok(LifecycleEffect {
        verb: "close".into(),
        effect_observed: before.is_real() && !after.is_real(),
        atspi_name: atspi,
        real_before: before.is_real(),
        real_after: after.is_real(),
        pid: after.pid,
    })
}

pub(crate) fn wait_until(
    app: &str,
    predicate: &str,
    timeout_ms: u64,
) -> Result<WaitAssertEffect, Refused> {
    let atspi = resolve_atspi_name(app)?;
    let want_real = match predicate {
        "real" | "open" | "mapped" => true,
        "unreal" | "closed" | "gone" => false,
        other => {
            return Err(Refused {
                reason: format!("unknown wait_until predicate: {other}"),
                tip: "Use real or unreal.".into(),
            });
        }
    };
    let p = wait_presence(&atspi, want_real, timeout_ms)?;
    let matched = p.is_real() == want_real;
    if !matched {
        return Err(Refused {
            reason: format!(
                "wait_until {predicate} timed out for {atspi} (real={})",
                p.is_real()
            ),
            tip: "Increase timeout_ms or open/close the app.".into(),
        });
    }
    Ok(WaitAssertEffect {
        verb: "wait_until".into(),
        ok: true,
        matched: true,
        predicate: predicate.into(),
        atspi_name: atspi,
        real: p.is_real(),
        pid: p.pid,
        detail: None,
    })
}

pub(crate) fn assert_state(
    app: &str,
    expect: &str,
    display_contains: Option<&str>,
) -> Result<WaitAssertEffect, Refused> {
    let atspi = resolve_atspi_name(app)?;
    let p = probe_presence(&atspi);
    let mut detail = String::new();
    let mut ok = match expect {
        "real" | "open" | "mapped" => {
            if !p.is_real() {
                detail = "expected real presence".into();
            }
            p.is_real()
        }
        "unreal" | "closed" | "gone" => {
            if p.is_real() {
                detail = "expected not real".into();
            }
            !p.is_real()
        }
        other => {
            return Err(Refused {
                reason: format!("unknown assert_state expect: {other}"),
                tip: "Use real or unreal (optional display_contains).".into(),
            });
        }
    };
    if let Some(needle) = display_contains {
        let text = crate::linux::read_app_display_text(&atspi).unwrap_or_default();
        if !text.contains(needle) {
            ok = false;
            detail = format!("display {text:?} missing {needle:?}");
        } else if detail.is_empty() {
            detail = format!("display={text:?}");
        }
    }
    Ok(WaitAssertEffect {
        verb: "assert_state".into(),
        ok,
        matched: ok,
        predicate: expect.into(),
        atspi_name: atspi,
        real: p.is_real(),
        pid: p.pid,
        detail: if detail.is_empty() {
            None
        } else {
            Some(detail)
        },
    })
}

pub(crate) fn type_text(app: &str, text: &str) -> Result<TextTypeEffect, Refused> {
    if text.is_empty() {
        return Err(Refused {
            reason: "type_text requires non-empty text".into(),
            tip: "Pass {\"text\":\"3\"}.".into(),
        });
    }
    let atspi = resolve_atspi_name(app)?;
    crate::presence::probe_presence(&atspi).refuse_if_unreal()?;
    let before = crate::linux::read_app_display_text(&atspi).unwrap_or_default();
    let _ = window_focus(&atspi);
    // Calculator (and similar) — type via named AT-SPI button Actions, not
    // keystroke injection (xdotool often misses the display entry).
    for ch in text.chars() {
        let name = ch.to_string();
        crate::linux::click_named_control(&atspi, &name)?;
        thread::sleep(Duration::from_millis(80));
    }
    // Display text lags Action; poll briefly for effect.
    let mut after = String::new();
    for _ in 0..12 {
        thread::sleep(Duration::from_millis(50));
        after = crate::linux::read_app_display_text(&atspi).unwrap_or_default();
        if after != before && (text == "C" || after.contains(text)) {
            break;
        }
    }
    let observed = if text == "C" {
        after != before || after.is_empty()
    } else {
        after != before && after.contains(text)
    };
    Ok(TextTypeEffect {
        verb: "type_text".into(),
        effect_observed: observed,
        atspi_name: atspi,
        text: text.into(),
        display_before: before,
        display_after: after,
    })
}

pub(crate) fn resolve_atspi_name_pub(app_or_name: &str) -> Result<String, Refused> {
    resolve_atspi_name(app_or_name)
}

fn resolve_atspi_name(app_or_name: &str) -> Result<String, Refused> {
    let raw = app_or_name.trim();
    if raw.is_empty() {
        return Err(Refused {
            reason: "app required".into(),
            tip: "Pass app like Calculator or gnome-calculator.".into(),
        });
    }
    // Already an atspi-ish id.
    if raw.contains('-') || raw.eq_ignore_ascii_case("cursor") {
        return Ok(raw.to_lowercase());
    }
    let items = inventory(true).map_err(|e| Refused {
        reason: e,
        tip: "Inventory unavailable.".into(),
    })?;
    let needle = raw.to_lowercase();
    for i in &items {
        if i.display_name.eq_ignore_ascii_case(raw) {
            if let Some(live) = &i.live {
                return Ok(live.atspi_name.clone());
            }
            if !i.exec_hint.is_empty() {
                return Ok(exec_basename(&i.exec_hint));
            }
        }
        if let Some(live) = &i.live {
            if live.atspi_name.eq_ignore_ascii_case(raw) {
                return Ok(live.atspi_name.clone());
            }
        }
        let base = exec_basename(&i.exec_hint);
        if base == needle {
            return Ok(base);
        }
    }
    // Calculator common alias
    if needle.contains("calc") {
        return Ok("gnome-calculator".into());
    }
    Ok(raw.to_lowercase())
}

fn resolve_exec(app_or_name: &str) -> Result<String, Refused> {
    let items = inventory(true).map_err(|e| Refused {
        reason: e,
        tip: "Inventory unavailable.".into(),
    })?;
    let needle = app_or_name.to_lowercase();
    for i in &items {
        let matches = i.display_name.eq_ignore_ascii_case(app_or_name)
            || i.exec_hint.to_lowercase().contains(&needle)
            || i.live
                .as_ref()
                .is_some_and(|l| l.atspi_name.to_lowercase().contains(&needle));
        if matches && !i.exec_hint.is_empty() {
            return Ok(first_exec_token(&i.exec_hint));
        }
    }
    if needle.contains("calc") {
        return Ok("gnome-calculator".into());
    }
    Err(Refused {
        reason: format!("no exec_hint for {app_or_name}"),
        tip: "Use a catalog display name (Calculator) or install the desktop entry.".into(),
    })
}

fn first_exec_token(exec: &str) -> String {
    exec.split_whitespace()
        .next()
        .unwrap_or(exec)
        .trim_matches('"')
        .to_string()
}

fn exec_basename(exec: &str) -> String {
    let tok = first_exec_token(exec);
    std::path::Path::new(&tok)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&tok)
        .to_lowercase()
}

fn wait_presence(
    atspi: &str,
    want_real: bool,
    timeout_ms: u64,
) -> Result<crate::TargetPresence, Refused> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    loop {
        let p = probe_presence(atspi);
        if p.is_real() == want_real {
            return Ok(p);
        }
        if Instant::now() >= deadline {
            return Ok(p);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn wm_delete(wid: u32) -> Result<(), Refused> {
    let (conn, _) = RustConnection::connect(None).map_err(|e| Refused {
        reason: format!("X11 connect failed: {e}"),
        tip: "Check DISPLAY.".into(),
    })?;
    let protocols = intern(&conn, "WM_PROTOCOLS").ok_or_else(|| Refused {
        reason: "intern WM_PROTOCOLS failed".into(),
        tip: "X11 WM required.".into(),
    })?;
    let delete = intern(&conn, "WM_DELETE_WINDOW").ok_or_else(|| Refused {
        reason: "intern WM_DELETE_WINDOW failed".into(),
        tip: "X11 WM required.".into(),
    })?;
    let data = [delete, CURRENT_TIME, 0, 0, 0];
    let event = ClientMessageEvent::new(32, wid, protocols, data);
    conn.send_event(false, wid, EventMask::NO_EVENT, event)
        .map_err(|e| Refused {
            reason: format!("WM_DELETE_WINDOW send failed: {e}"),
            tip: "Retry close or kill.".into(),
        })?
        .check()
        .map_err(|e| Refused {
            reason: format!("WM_DELETE_WINDOW check failed: {e}"),
            tip: "Retry close or kill.".into(),
        })?;
    let _ = conn.flush();
    Ok(())
}

fn intern(conn: &RustConnection, name: &str) -> Option<xproto::Atom> {
    conn.intern_atom(false, name.as_bytes())
        .ok()?
        .reply()
        .ok()
        .map(|r| r.atom)
}

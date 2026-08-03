//! Linux X11 window verbs — move / resize / focus (reality-gated).
//!
//! Geometry is the proven signal on this seat (ConfigureNotify / xwininfo).
//! AT-SPI `window:move` / `window:resize` are unreliable — do not use them alone.

use crate::presence::{x11_class_needle, TargetPresence};
use crate::{OuterGeom, WindowVerbEffect};
use dpl_coach_fabric::Refused;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, ClientMessageEvent, ConnectionExt as _, EventMask};
use x11rb::rust_connection::RustConnection;
use x11rb::CURRENT_TIME;

const MIN_OUTER: i32 = 50;

pub(crate) fn geom(atspi_name: &str) -> Result<OuterGeom, Refused> {
    require_real(atspi_name)?;
    let class = x11_class_needle(atspi_name);
    find_mapped(class).ok_or_else(|| Refused {
        reason: format!("{atspi_name} has no mapped window"),
        tip: "Open the app so presence is real.".into(),
    })
}

pub(crate) fn move_to(atspi_name: &str, x: i32, y: i32) -> Result<WindowVerbEffect, Refused> {
    let before = geom(atspi_name)?;
    configure(before.window_id, Some(x), Some(y), None, None)?;
    // Flush + settle
    std::thread::sleep(std::time::Duration::from_millis(80));
    let after = geom(atspi_name)?;
    let moved = after.x != before.x || after.y != before.y;
    Ok(WindowVerbEffect {
        verb: "move".into(),
        effect_observed: moved,
        before,
        after,
    })
}

pub(crate) fn resize_to(atspi_name: &str, w: i32, h: i32) -> Result<WindowVerbEffect, Refused> {
    if w < MIN_OUTER || h < MIN_OUTER {
        return Err(Refused {
            reason: format!("resize refused: {w}×{h} below {MIN_OUTER}×{MIN_OUTER}"),
            tip: "Pass outer size ≥ 50×50.".into(),
        });
    }
    let before = geom(atspi_name)?;
    configure(before.window_id, None, None, Some(w as u32), Some(h as u32))?;
    std::thread::sleep(std::time::Duration::from_millis(80));
    let after = geom(atspi_name)?;
    let resized = after.w != before.w || after.h != before.h;
    Ok(WindowVerbEffect {
        verb: "resize".into(),
        effect_observed: resized,
        before,
        after,
    })
}

pub(crate) fn focus(atspi_name: &str) -> Result<WindowVerbEffect, Refused> {
    let before = geom(atspi_name)?;
    activate(before.window_id)?;
    std::thread::sleep(std::time::Duration::from_millis(80));
    let after = geom(atspi_name)?;
    let active = active_window_id().is_some_and(|id| id == after.window_id);
    Ok(WindowVerbEffect {
        verb: "focus".into(),
        effect_observed: active,
        before,
        after,
    })
}

fn require_real(atspi_name: &str) -> Result<TargetPresence, Refused> {
    let p = crate::presence::probe_presence(atspi_name);
    p.refuse_if_unreal()?;
    Ok(p)
}

fn find_mapped(class_needle: &str) -> Option<OuterGeom> {
    let (conn, screen_num) = RustConnection::connect(None).ok()?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let atom_client = intern(&conn, "_NET_CLIENT_LIST")?;
    let atom_class = intern(&conn, "WM_CLASS")?;

    let reply = conn
        .get_property(false, root, atom_client, xproto::AtomEnum::WINDOW, 0, 8192)
        .ok()?
        .reply()
        .ok()?;
    let raw = reply.value;
    if !raw.len().is_multiple_of(4) {
        return None;
    }
    let windows: Vec<u32> = raw
        .chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let needle = class_needle.to_lowercase();
    let mut best: Option<(OuterGeom, i32)> = None;
    for &wid in &windows {
        let class = string_prop(&conn, wid, atom_class).unwrap_or_default();
        if !class.to_lowercase().contains(&needle) {
            continue;
        }
        let geom = conn.get_geometry(wid).ok()?.reply().ok()?;
        let w = i32::from(geom.width);
        let h = i32::from(geom.height);
        if w < MIN_OUTER || h < MIN_OUTER {
            continue;
        }
        let (x, y) = absolute_origin(&conn, root, wid, geom.x, geom.y);
        let area = w * h;
        let g = OuterGeom {
            window_id: wid,
            x,
            y,
            w,
            h,
        };
        if best.as_ref().map(|b| b.1).unwrap_or(0) < area {
            best = Some((g, area));
        }
    }
    best.map(|(g, _)| g)
}

fn absolute_origin(
    conn: &RustConnection,
    root: u32,
    wid: u32,
    geom_x: i16,
    geom_y: i16,
) -> (i32, i32) {
    if let Ok(cookie) = conn.translate_coordinates(wid, root, 0, 0) {
        if let Ok(reply) = cookie.reply() {
            return (i32::from(reply.dst_x), i32::from(reply.dst_y));
        }
    }
    (i32::from(geom_x), i32::from(geom_y))
}

fn configure(
    wid: u32,
    x: Option<i32>,
    y: Option<i32>,
    w: Option<u32>,
    h: Option<u32>,
) -> Result<(), Refused> {
    let (conn, _) = RustConnection::connect(None).map_err(|e| Refused {
        reason: format!("X11 connect failed: {e}"),
        tip: "Check DISPLAY.".into(),
    })?;
    let mut value_list = xproto::ConfigureWindowAux::new();
    if let Some(v) = x {
        value_list = value_list.x(v);
    }
    if let Some(v) = y {
        value_list = value_list.y(v);
    }
    if let Some(v) = w {
        value_list = value_list.width(v);
    }
    if let Some(v) = h {
        value_list = value_list.height(v);
    }
    conn.configure_window(wid, &value_list)
        .map_err(|e| Refused {
            reason: format!("configure_window failed: {e}"),
            tip: "WM may reject programmatic move/resize.".into(),
        })?
        .check()
        .map_err(|e| Refused {
            reason: format!("configure_window check failed: {e}"),
            tip: "WM may reject programmatic move/resize.".into(),
        })?;
    conn.flush().map_err(|e| Refused {
        reason: format!("X11 flush failed: {e}"),
        tip: "Retry.".into(),
    })?;
    Ok(())
}

fn activate(wid: u32) -> Result<(), Refused> {
    let (conn, screen_num) = RustConnection::connect(None).map_err(|e| Refused {
        reason: format!("X11 connect failed: {e}"),
        tip: "Check DISPLAY.".into(),
    })?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let atom_active = intern(&conn, "_NET_ACTIVE_WINDOW").ok_or_else(|| Refused {
        reason: "intern _NET_ACTIVE_WINDOW failed".into(),
        tip: "EWMH WM required.".into(),
    })?;

    // Source indication 2 = pager; timestamp CurrentTime (0).
    let data = [2u32, 0, 0, 0, 0];
    let event = ClientMessageEvent::new(32, wid, atom_active, data);
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event,
    )
    .map_err(|e| Refused {
        reason: format!("send_event _NET_ACTIVE_WINDOW failed: {e}"),
        tip: "WM may block focus steals.".into(),
    })?
    .check()
    .map_err(|e| Refused {
        reason: format!("_NET_ACTIVE_WINDOW check failed: {e}"),
        tip: "WM may block focus steals.".into(),
    })?;

    let _ = conn.set_input_focus(xproto::InputFocus::PARENT, wid, CURRENT_TIME);
    conn.flush().map_err(|e| Refused {
        reason: format!("X11 flush failed: {e}"),
        tip: "Retry.".into(),
    })?;
    Ok(())
}

fn active_window_id() -> Option<u32> {
    let (conn, screen_num) = RustConnection::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let atom = intern(&conn, "_NET_ACTIVE_WINDOW")?;
    let reply = conn
        .get_property(false, root, atom, xproto::AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let v = reply.value;
    if v.len() >= 4 {
        Some(u32::from_ne_bytes([v[0], v[1], v[2], v[3]]))
    } else {
        None
    }
}

fn intern(conn: &RustConnection, name: &str) -> Option<xproto::Atom> {
    conn.intern_atom(false, name.as_bytes())
        .ok()?
        .reply()
        .ok()
        .map(|r| r.atom)
}

fn string_prop(conn: &RustConnection, wid: u32, atom: xproto::Atom) -> Option<String> {
    let reply = conn
        .get_property(false, wid, atom, xproto::AtomEnum::ANY, 0, 4096)
        .ok()?
        .reply()
        .ok()?;
    Some(String::from_utf8_lossy(&reply.value).replace('\0', " "))
}

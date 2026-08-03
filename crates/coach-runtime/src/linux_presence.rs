//! Linux X11 mapped-window + pid presence (reality gate).

use crate::presence::{x11_class_needle, TargetPresence};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

pub(crate) fn probe(atspi_name: &str) -> TargetPresence {
    let atspi_listed = atspi_app_listed(atspi_name);
    let class = x11_class_needle(atspi_name);
    let Some((window_id, outer_w, outer_h, pid)) = mapped_window(class) else {
        return TargetPresence {
            atspi_name: atspi_name.into(),
            atspi_listed,
            mapped_window: false,
            window_id: None,
            outer_w: 0,
            outer_h: 0,
            pid: None,
            pid_alive: false,
        };
    };
    let pid_alive = pid.is_some_and(pid_alive);
    TargetPresence {
        atspi_name: atspi_name.into(),
        atspi_listed,
        mapped_window: true,
        window_id: Some(window_id),
        outer_w,
        outer_h,
        pid,
        pid_alive,
    }
}

fn pid_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).is_dir()
}

fn atspi_app_listed(app: &str) -> bool {
    // Cheap: inventory live census names.
    match dpl_coach_inventory::live_census() {
        Ok(live) => {
            let app_l = app.to_lowercase();
            live.iter().any(|l| {
                let n = l.name.to_lowercase();
                n == app_l
                    || n.contains(&app_l)
                    || app_l.contains(&n)
                    || compact(&n) == compact(&app_l)
            })
        }
        Err(_) => false,
    }
}

fn compact(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn mapped_window(class_needle: &str) -> Option<(u32, i32, i32, Option<u32>)> {
    let (conn, screen_num) = RustConnection::connect(None).ok()?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let atom_client = intern(&conn, "_NET_CLIENT_LIST")?;
    let atom_pid = intern(&conn, "_NET_WM_PID")?;
    let atom_class = intern(&conn, "WM_CLASS")?;

    let reply = conn
        .get_property(false, root, atom_client, xproto::AtomEnum::WINDOW, 0, 8192)
        .ok()?
        .reply()
        .ok()?;
    let raw: Vec<u8> = reply.value;
    if !raw.len().is_multiple_of(4) {
        return None;
    }
    let windows: Vec<u32> = raw
        .chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let mut best: Option<(u32, i32, i32, Option<u32>, i32)> = None;
    for &wid in &windows {
        let class = string_prop(&conn, wid, atom_class).unwrap_or_default();
        if !class.to_lowercase().contains(&class_needle.to_lowercase()) {
            continue;
        }
        let geom = conn.get_geometry(wid).ok()?.reply().ok()?;
        let outer_w = i32::from(geom.width);
        let outer_h = i32::from(geom.height);
        // Reality: ignore tray-sized / unmapped-ish leftovers.
        if outer_w < 50 || outer_h < 50 {
            continue;
        }
        let pid = card_prop(&conn, wid, atom_pid);
        let area = outer_w * outer_h;
        if best.as_ref().map(|b| b.4).unwrap_or(0) < area {
            best = Some((wid, outer_w, outer_h, pid, area));
        }
    }
    best.map(|(id, w, h, pid, _)| (id, w, h, pid))
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

fn card_prop(conn: &RustConnection, wid: u32, atom: xproto::Atom) -> Option<u32> {
    let reply = conn
        .get_property(false, wid, atom, xproto::AtomEnum::CARDINAL, 0, 1)
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

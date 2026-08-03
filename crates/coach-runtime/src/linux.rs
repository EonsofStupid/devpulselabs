//! Linux AT-SPI resolve / invoke / extents for FabricSession.

use crate::resolve::{fingerprint_matches, path_indices};
use crate::{AgencyMode, FabricSession, IRect, InvokeEffect, LiveHandle, NodeRef};
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::action::ActionProxy;
use atspi::proxy::component::ComponentProxy;
use atspi::CoordType;
use atspi::ObjectRefOwned;
use dpl_coach_fabric::Refused;
use std::future::Future;
use zbus::proxy;
use zbus::Connection as ZbusConnection;

#[derive(Debug, Clone)]
pub(crate) struct LinuxLive {
    dest: zbus::names::BusName<'static>,
    path: zbus::zvariant::ObjectPath<'static>,
}

pub(crate) fn resolve_live(session: &FabricSession, node: &NodeRef) -> Result<LiveHandle, Refused> {
    block_on(resolve_live_async(session, node))
}

pub(crate) fn invoke_action(
    handle: &LiveHandle,
    action_name: &str,
    agency: AgencyMode,
    require_effect: bool,
) -> Result<InvokeEffect, Refused> {
    block_on(invoke_async(handle, action_name, agency, require_effect))
}

pub(crate) fn live_extents(handle: &LiveHandle) -> Result<IRect, Refused> {
    block_on(extents_async(handle))
}

/// Read Calculator-class display text (empty if none).
pub(crate) fn read_app_display_text(atspi_name: &str) -> Result<String, Refused> {
    block_on(async {
        let conn = a11y_connection().await.map_err(|e| Refused {
            reason: format!("a11y bus unavailable: {e}"),
            tip: "Accessibility must be on for Coach.".into(),
        })?;
        read_display_text_on(&conn, atspi_name)
            .await
            .map_err(|e| Refused {
                reason: e,
                tip: "Open the app and ensure AT-SPI exposes a text node.".into(),
            })
    })
}

/// Click a named control (e.g. Calculator digit button) via AT-SPI Action.
pub(crate) fn click_named_control(atspi_name: &str, name: &str) -> Result<(), Refused> {
    block_on(async {
        let handle = resolve_named_handle(atspi_name, name, Some("button")).await?;
        invoke_async(&handle, "click", AgencyMode::Coach, false).await?;
        Ok(())
    })
}

/// Select / activate a named control (any role) via Select|Click|Press Action.
pub(crate) fn select_named_control(atspi_name: &str, name: &str) -> Result<(), Refused> {
    block_on(async {
        let handle = resolve_named_handle(atspi_name, name, None).await?;
        let role = handle.node.role.clone();
        // Prefer Select, then click/press/activate.
        let mut last_err = String::new();
        for want in ["select", "click", "press", "activate"] {
            match invoke_async(&handle, want, AgencyMode::Coach, false).await {
                Ok(_) => return Ok(()),
                Err(e) => last_err = e.reason,
            }
        }
        // Some GTK4 toggles expose Actions to GI but empty via zbus ActionProxy.
        // Fall back to Component screen extents + synthetic click (still a11y-addressed).
        match click_handle_center(&handle).await {
            Ok(()) => Ok(()),
            Err(e) => Err(Refused {
                reason: format!(
                    "select: no usable Action on {name:?} (role={role}): {last_err}; extents-click: {}",
                    e.reason
                ),
                tip: "Pick a selectable or clickable control name.".into(),
            }),
        }
    })
}

async fn click_handle_center(handle: &LiveHandle) -> Result<(), Refused> {
    let conn = a11y_connection().await.map_err(|e| Refused {
        reason: format!("a11y bus unavailable: {e}"),
        tip: "Accessibility must be on for Coach.".into(),
    })?;
    let component = ComponentProxy::builder(&conn)
        .destination(handle.linux.dest.clone())
        .map_err(|e| Refused {
            reason: e.to_string(),
            tip: "No Component iface.".into(),
        })?
        .path(handle.linux.path.clone())
        .map_err(|e| Refused {
            reason: e.to_string(),
            tip: "No Component iface.".into(),
        })?
        .build()
        .await
        .map_err(|e| Refused {
            reason: format!("Component proxy: {e}"),
            tip: "No Component iface.".into(),
        })?;
    let (x, y, w, h) = component
        .get_extents(CoordType::Screen)
        .await
        .map_err(|e| Refused {
            reason: format!("get_extents(Screen) failed: {e}"),
            tip: "Control has no screen extents.".into(),
        })?;
    if w < 2 || h < 2 {
        return Err(Refused {
            reason: format!("degenerate select extents {w}×{h}"),
            tip: "Control not visible.".into(),
        });
    }
    let cx = x + w / 2;
    let cy = y + h / 2;
    let mut cmd = std::process::Command::new("xdotool");
    dpl_proc::quiet(&mut cmd);
    let status = cmd
        .args([
            "mousemove",
            "--sync",
            &cx.to_string(),
            &cy.to_string(),
            "click",
            "1",
        ])
        .status()
        .map_err(|e| Refused {
            reason: format!("xdotool click failed: {e}"),
            tip: "Install xdotool.".into(),
        })?;
    if !status.success() {
        return Err(Refused {
            reason: format!("xdotool click exit {status}"),
            tip: "WM may block synthetic clicks.".into(),
        });
    }
    Ok(())
}

/// Compact role+state summary for a named control (for select effect).
pub(crate) fn control_state_summary(atspi_name: &str, name: &str) -> Result<String, Refused> {
    block_on(async {
        let conn = a11y_connection().await.map_err(|e| Refused {
            reason: format!("a11y bus unavailable: {e}"),
            tip: "Accessibility must be on for Coach.".into(),
        })?;
        let root = find_app_root(&conn, atspi_name)
            .await
            .map_err(|e| Refused {
                reason: e,
                tip: format!("Open {atspi_name}, then retry."),
            })?;
        let Some(proxy) = find_proxy_by_name(&conn, &root, name, 0, 20).await? else {
            return Ok(format!("missing:{name}"));
        };
        let role = proxy
            .get_role_name()
            .await
            .unwrap_or_else(|_| "unknown".into());
        let states = proxy
            .get_state()
            .await
            .map(|s| format!("{s:?}"))
            .unwrap_or_else(|_| "[]".into());
        Ok(format!("{role}|{states}"))
    })
}

/// Snapshot of scroll panes (name+extents) for scroll effect detection.
pub(crate) fn scroll_snapshot(atspi_name: &str) -> Result<String, Refused> {
    block_on(async {
        let conn = a11y_connection().await.map_err(|e| Refused {
            reason: format!("a11y bus unavailable: {e}"),
            tip: "Accessibility must be on for Coach.".into(),
        })?;
        let root = find_app_root(&conn, atspi_name)
            .await
            .map_err(|e| Refused {
                reason: e,
                tip: format!("Open {atspi_name}, then retry."),
            })?;
        let mut parts = Vec::new();
        collect_scroll_parts(&conn, &root, 0, &mut parts).await?;
        Ok(parts.join(";"))
    })
}

/// Try AT-SPI scroll / click on a scroll pane. Returns via tag.
pub(crate) fn scroll_named(atspi_name: &str, dir: &str, clicks: u32) -> Result<String, Refused> {
    block_on(async {
        let conn = a11y_connection().await.map_err(|e| Refused {
            reason: format!("a11y bus unavailable: {e}"),
            tip: "Accessibility must be on for Coach.".into(),
        })?;
        let root = find_app_root(&conn, atspi_name)
            .await
            .map_err(|e| Refused {
                reason: e,
                tip: format!("Open {atspi_name}, then retry."),
            })?;
        let Some(proxy) = find_scroll_pane(&conn, &root, 0).await? else {
            return Err(Refused {
                reason: "no scroll pane found".into(),
                tip: "App may not expose a scrollable; wheel fallback will try.".into(),
            });
        };
        let expected = NodeRef {
            path: String::new(),
            role: proxy
                .get_role_name()
                .await
                .unwrap_or_else(|_| "scroll pane".into()),
            name: proxy.name().await.unwrap_or_default(),
            actions: Vec::new(),
        };
        let handle = live_from_proxy(&expected, &proxy, atspi_name);
        let action_name = match dir {
            "up" => "scroll up",
            "down" => "scroll down",
            "left" => "scroll left",
            "right" => "scroll right",
            _ => "scroll down",
        };
        for _ in 0..clicks {
            if invoke_async(&handle, action_name, AgencyMode::Coach, false)
                .await
                .is_err()
            {
                // Many panes only expose click — still count as atspi attempt.
                let _ = invoke_async(&handle, "click", AgencyMode::Coach, false).await;
            }
        }
        Ok(format!("atspi:{action_name}"))
    })
}

async fn resolve_named_handle(
    atspi_name: &str,
    name: &str,
    role_hint: Option<&str>,
) -> Result<LiveHandle, Refused> {
    let conn = a11y_connection().await.map_err(|e| Refused {
        reason: format!("a11y bus unavailable: {e}"),
        tip: "Accessibility must be on for Coach.".into(),
    })?;
    let root = find_app_root(&conn, atspi_name)
        .await
        .map_err(|e| Refused {
            reason: e,
            tip: format!("Open {atspi_name}, then retry."),
        })?;
    // Role-preferring walk first (toggle/check before inert push-button aliases).
    let hints: Vec<&str> = match role_hint {
        Some(h) => vec![h],
        None => vec![
            "toggle button",
            "check box",
            "radio button",
            "push button",
            "button",
            "",
        ],
    };
    for hint in hints {
        let expected = NodeRef {
            path: String::new(),
            role: hint.into(),
            name: name.into(),
            actions: Vec::new(),
        };
        if let Some(handle) = find_named_under(&conn, &root, &expected, atspi_name, 0, 20).await? {
            return Ok(handle);
        }
    }
    if let Some(proxy) = find_proxy_by_name(&conn, &root, name, 0, 20).await? {
        let role = proxy
            .get_role_name()
            .await
            .unwrap_or_else(|_| "unknown".into());
        let expected = NodeRef {
            path: String::new(),
            role,
            name: name.into(),
            actions: Vec::new(),
        };
        return Ok(live_from_proxy(&expected, &proxy, atspi_name));
    }
    Err(Refused {
        reason: format!("no control named {name:?} under {atspi_name}"),
        tip: "Re-open the app or use invoke on a fabric seed.".into(),
    })
}

async fn find_proxy_by_name(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    want: &str,
    depth: usize,
    max_depth: usize,
) -> Result<Option<AccessibleProxy<'static>>, Refused> {
    // Prefer a node that exposes Actions (Calculator has a nameless-action
    // "Basic" push button before the real toggle button with click).
    let mut fallback: Option<AccessibleProxy<'static>> = None;
    find_proxy_by_name_inner(conn, node, want, depth, max_depth, &mut fallback).await?;
    if let Some(p) = fallback {
        return Ok(Some(p));
    }
    Ok(None)
}

async fn find_proxy_by_name_inner(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    want: &str,
    depth: usize,
    max_depth: usize,
    fallback: &mut Option<AccessibleProxy<'static>>,
) -> Result<Option<AccessibleProxy<'static>>, Refused> {
    if depth > max_depth {
        return Ok(None);
    }
    let name = node.name().await.unwrap_or_default();
    if name == want {
        let rebuilt = AccessibleProxy::builder(conn)
            .destination(node.inner().destination().to_owned())
            .map_err(|e| Refused {
                reason: e.to_string(),
                tip: "AT-SPI proxy rebuild failed.".into(),
            })?
            .path(node.inner().path().to_owned())
            .map_err(|e| Refused {
                reason: e.to_string(),
                tip: "AT-SPI proxy rebuild failed.".into(),
            })?
            .build()
            .await
            .map_err(|e| Refused {
                reason: e.to_string(),
                tip: "AT-SPI proxy rebuild failed.".into(),
            })?;
        let actionable = match ActionProxy::builder(conn)
            .destination(rebuilt.inner().destination().to_owned())
        {
            Ok(b) => match b.path(rebuilt.inner().path().to_owned()) {
                Ok(b) => match b.build().await {
                    Ok(a) => a
                        .get_actions()
                        .await
                        .map(|v| !v.is_empty())
                        .unwrap_or(false),
                    Err(_) => false,
                },
                Err(_) => false,
            },
            Err(_) => false,
        };
        if actionable {
            return Ok(Some(rebuilt));
        }
        if fallback.is_none() {
            *fallback = Some(rebuilt);
        }
    }
    let children = node.get_children().await.unwrap_or_default();
    for child in children.into_iter().take(64) {
        let Ok(proxy) = proxy_owned(conn, &child).await else {
            continue;
        };
        if let Some(found) = Box::pin(find_proxy_by_name_inner(
            conn,
            &proxy,
            want,
            depth + 1,
            max_depth,
            fallback,
        ))
        .await?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn find_scroll_pane(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    depth: usize,
) -> Result<Option<AccessibleProxy<'static>>, Refused> {
    if depth > 16 {
        return Ok(None);
    }
    let role = node
        .get_role_name()
        .await
        .unwrap_or_else(|_| String::new())
        .to_lowercase();
    if role.contains("scroll") {
        return Ok(Some(
            AccessibleProxy::builder(conn)
                .destination(node.inner().destination().to_owned())
                .map_err(|e| Refused {
                    reason: e.to_string(),
                    tip: "scroll pane proxy failed".into(),
                })?
                .path(node.inner().path().to_owned())
                .map_err(|e| Refused {
                    reason: e.to_string(),
                    tip: "scroll pane proxy failed".into(),
                })?
                .build()
                .await
                .map_err(|e| Refused {
                    reason: e.to_string(),
                    tip: "scroll pane proxy failed".into(),
                })?,
        ));
    }
    let children = node.get_children().await.unwrap_or_default();
    for child in children.into_iter().take(64) {
        let Ok(proxy) = proxy_owned(conn, &child).await else {
            continue;
        };
        if let Some(found) = Box::pin(find_scroll_pane(conn, &proxy, depth + 1)).await? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn collect_scroll_parts(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    depth: usize,
    out: &mut Vec<String>,
) -> Result<(), Refused> {
    if depth > 14 || out.len() >= 8 {
        return Ok(());
    }
    let role = node
        .get_role_name()
        .await
        .unwrap_or_else(|_| String::new())
        .to_lowercase();
    if role.contains("scroll") {
        let name = node.name().await.unwrap_or_default();
        if let Ok(comp) = ComponentProxy::builder(conn)
            .destination(node.inner().destination())
            .map_err(|e| Refused {
                reason: e.to_string(),
                tip: "component".into(),
            })?
            .path(node.inner().path())
            .map_err(|e| Refused {
                reason: e.to_string(),
                tip: "component".into(),
            })?
            .build()
            .await
        {
            if let Ok((x, y, w, h)) = comp.get_extents(CoordType::Window).await {
                out.push(format!("{name}:{x},{y},{w},{h}"));
            } else {
                out.push(format!("{name}:?"));
            }
        }
    }
    let children = node.get_children().await.unwrap_or_default();
    for child in children.into_iter().take(48) {
        let Ok(proxy) = proxy_owned(conn, &child).await else {
            continue;
        };
        Box::pin(collect_scroll_parts(conn, &proxy, depth + 1, out)).await?;
    }
    Ok(())
}

async fn resolve_live_async(
    session: &FabricSession,
    node: &NodeRef,
) -> Result<LiveHandle, Refused> {
    let conn = a11y_connection().await.map_err(|e| Refused {
        reason: format!("a11y bus unavailable: {e}"),
        tip: "Accessibility must be on for Coach — check Settings → Accessibility.".into(),
    })?;
    let root = find_app_root(&conn, &session.fabric.atspi_name)
        .await
        .map_err(|e| Refused {
            reason: e,
            tip: format!("Open {}, then Import again.", session.fabric.atspi_name),
        })?;

    let indices = path_indices(&node.path)?;
    let mut current = root;
    let mut parent_for_search = None;

    for (step, idx) in indices.iter().enumerate() {
        parent_for_search = Some(key_of(&current));
        let count = current.child_count().await.unwrap_or(0);
        // AT-SPI takes i32; refuse before a wrapping cast can bypass bounds.
        let idx_i32 = i32::try_from(*idx).map_err(|_| Refused {
            reason: format!("fabric path index {idx} does not fit AT-SPI child index"),
            tip: "Re-import the app — the signal map path is corrupt.".into(),
        })?;
        if idx_i32 >= count {
            return recover_under_parent(
                &conn,
                parent_for_search.as_ref(),
                node,
                &session.fabric.atspi_name,
                step,
            )
            .await;
        }
        let child = current
            .get_child_at_index(idx_i32)
            .await
            .map_err(|e| Refused {
                reason: format!("get_child_at_index({idx}) failed: {e}"),
                tip: "UI shifted — Re-import to refresh the signal map.".into(),
            })?;
        current = proxy_owned(&conn, &child).await.map_err(|e| Refused {
            reason: e,
            tip: "UI shifted — Re-import to refresh the signal map.".into(),
        })?;
    }

    let name = current.name().await.unwrap_or_default();
    let role = current
        .get_role_name()
        .await
        .unwrap_or_else(|_| "unknown".into());
    if fingerprint_matches(node, &name, &role) {
        return Ok(live_from_proxy(node, &current, &session.fabric.atspi_name));
    }

    // Electron reparent / index drift — bounded search under parent.
    recover_under_parent(
        &conn,
        parent_for_search.as_ref(),
        node,
        &session.fabric.atspi_name,
        indices.len(),
    )
    .await
}

async fn recover_under_parent(
    conn: &ZbusConnection,
    parent: Option<&LinuxLive>,
    node: &NodeRef,
    atspi_name: &str,
    _depth_hint: usize,
) -> Result<LiveHandle, Refused> {
    let Some(parent) = parent else {
        return Err(Refused {
            reason: format!(
                "resolve failed: root fingerprint mismatch for {:?}",
                node.name
            ),
            tip: "Re-import the app, or pick a different seed control.".into(),
        });
    };
    let parent_proxy = proxy_key(conn, parent).await.map_err(|e| Refused {
        reason: e,
        tip: "Re-import the app to refresh the signal map.".into(),
    })?;
    if let Some(found) = find_named_under(conn, &parent_proxy, node, atspi_name, 0, 4).await? {
        return Ok(found);
    }
    Err(Refused {
        reason: format!(
            "resolve refused: no live node matching {:?} ({}) at path {}",
            node.name, node.role, node.path
        ),
        tip: "UI shifted or seed missing — Re-import, or open the control's panel first.".into(),
    })
}

async fn find_named_under(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    expected: &NodeRef,
    atspi_name: &str,
    depth: usize,
    max_depth: usize,
) -> Result<Option<LiveHandle>, Refused> {
    if depth > max_depth {
        return Ok(None);
    }
    let name = node.name().await.unwrap_or_default();
    let role = node
        .get_role_name()
        .await
        .unwrap_or_else(|_| "unknown".into());
    if fingerprint_matches(expected, &name, &role) {
        return Ok(Some(live_from_proxy(expected, node, atspi_name)));
    }
    let children = node.get_children().await.map_err(|e| Refused {
        reason: format!("get_children during resolve recover: {e}"),
        tip: "Accessibility tree busy — retry Import.".into(),
    })?;
    // Cap fan-out to avoid freeze landmines during recover.
    for child in children.into_iter().take(64) {
        let Ok(proxy) = proxy_owned(conn, &child).await else {
            continue;
        };
        if let Some(found) = Box::pin(find_named_under(
            conn,
            &proxy,
            expected,
            atspi_name,
            depth + 1,
            max_depth,
        ))
        .await?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn invoke_async(
    handle: &LiveHandle,
    action_name: &str,
    agency: AgencyMode,
    require_effect: bool,
) -> Result<InvokeEffect, Refused> {
    let conn = a11y_connection().await.map_err(|e| Refused {
        reason: format!("a11y bus unavailable: {e}"),
        tip: "Accessibility must be on for Coach.".into(),
    })?;
    let digit_seed =
        handle.node.name.chars().all(|c| c.is_ascii_digit()) && !handle.node.name.is_empty();
    let before = if require_effect && digit_seed {
        Some(
            read_display_text_on(&conn, &handle.atspi_name)
                .await
                .unwrap_or_default(),
        )
    } else {
        None
    };
    let action = ActionProxy::builder(&conn)
        .destination(handle.linux.dest.clone())
        .map_err(|e| Refused {
            reason: e.to_string(),
            tip: "Could not open Action interface — pick another seed.".into(),
        })?
        .path(handle.linux.path.clone())
        .map_err(|e| Refused {
            reason: e.to_string(),
            tip: "Could not open Action interface — pick another seed.".into(),
        })?
        .build()
        .await
        .map_err(|e| Refused {
            reason: format!("Action proxy build failed: {e}"),
            tip: "This control may not expose Actions — Advanced Import may be required.".into(),
        })?;
    let actions = action.get_actions().await.map_err(|e| Refused {
        reason: format!("get_actions failed: {e}"),
        tip: "This control exposes no Actions — cannot invoke without pixels (refused).".into(),
    })?;
    if actions.is_empty() {
        return Err(Refused {
            reason: format!(
                "invoke refused: {:?} has zero Actions (agency={})",
                handle.node.name,
                agency.as_str()
            ),
            tip: "Electron seed has no click Action — do not fall back to screen click; amend harvest or pick another seed."
                .into(),
        });
    }
    let index = actions
        .iter()
        .position(|a| a.name.eq_ignore_ascii_case(action_name))
        .or_else(|| {
            if action_name.eq_ignore_ascii_case("click") {
                actions
                    .iter()
                    .position(|a| a.name.eq_ignore_ascii_case("press"))
                    .or(Some(0))
            } else {
                None
            }
        })
        .ok_or_else(|| Refused {
            reason: format!(
                "action {action_name:?} not on {:?}; have {:?}",
                handle.node.name,
                actions.iter().map(|a| &a.name).collect::<Vec<_>>()
            ),
            tip: "Use an action listed on the fabric node, or re-import.".into(),
        })?;
    let ok = action.do_action(index as i32).await.map_err(|e| Refused {
        reason: format!("do_action({index}) error: {e}"),
        tip: "Target refused the action — focus the app and retry.".into(),
    })?;
    if !ok {
        return Err(Refused {
            reason: format!(
                "do_action({index}) returned false on {:?}",
                handle.node.name
            ),
            tip: "Target refused the action — focus the app and retry.".into(),
        });
    }
    let after = if require_effect && digit_seed {
        std::thread::sleep(std::time::Duration::from_millis(350));
        Some(
            read_display_text_on(&conn, &handle.atspi_name)
                .await
                .unwrap_or_default(),
        )
    } else {
        None
    };
    let effect_observed = match (&before, &after) {
        (Some(b), Some(a)) => a != b && !a.is_empty(),
        _ => !require_effect,
    };
    if require_effect && digit_seed && !effect_observed {
        return Err(Refused {
            reason: format!(
                "invoke had no observable effect on {:?} (display before={before:?} after={after:?})",
                handle.node.name
            ),
            tip: "Open a visible Calculator window, clear/focus it, then retry. doAction alone is not PASS."
                .into(),
        });
    }
    Ok(InvokeEffect {
        action: actions[index].name.clone(),
        agency,
        display_before: before,
        display_after: after,
        effect_observed,
    })
}

async fn read_display_text_on(conn: &ZbusConnection, app: &str) -> Result<String, String> {
    let root = find_app_root(conn, app).await?;
    if let Some(text) = find_display_text(conn, &root, 0).await? {
        return Ok(text);
    }
    Ok(String::new())
}

async fn find_display_text(
    conn: &ZbusConnection,
    node: &AccessibleProxy<'_>,
    depth: usize,
) -> Result<Option<String>, String> {
    use atspi::Role;
    if depth > 12 {
        return Ok(None);
    }
    let role = node.get_role().await.unwrap_or(Role::Invalid);
    if matches!(role, Role::Text | Role::Entry | Role::PasswordText) {
        if let Ok(text) = atspi::proxy::text::TextProxy::builder(conn)
            .destination(node.inner().destination())
            .map_err(|e| e.to_string())?
            .path(node.inner().path())
            .map_err(|e| e.to_string())?
            .build()
            .await
        {
            if let Ok(body) = text.get_text(0, -1).await {
                let body = body.trim().to_string();
                if !body.is_empty() {
                    return Ok(Some(body));
                }
            }
        }
    }
    let children = node.get_children().await.unwrap_or_default();
    for child in children.into_iter().take(64) {
        let Ok(proxy) = proxy_owned(conn, &child).await else {
            continue;
        };
        if let Some(found) = Box::pin(find_display_text(conn, &proxy, depth + 1)).await? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn extents_async(handle: &LiveHandle) -> Result<IRect, Refused> {
    let conn = a11y_connection().await.map_err(|e| Refused {
        reason: format!("a11y bus unavailable: {e}"),
        tip: "Accessibility must be on for Coach.".into(),
    })?;
    let component = ComponentProxy::builder(&conn)
        .destination(handle.linux.dest.clone())
        .map_err(|e| Refused {
            reason: e.to_string(),
            tip: "No Component iface — cannot Mount paint.".into(),
        })?
        .path(handle.linux.path.clone())
        .map_err(|e| Refused {
            reason: e.to_string(),
            tip: "No Component iface — cannot Mount paint.".into(),
        })?
        .build()
        .await
        .map_err(|e| Refused {
            reason: format!("Component proxy: {e}"),
            tip: "No Component iface — cannot Mount paint.".into(),
        })?;
    let (x, y, w, h) = component
        .get_extents(CoordType::Window)
        .await
        .map_err(|e| Refused {
            reason: format!("get_extents failed: {e}"),
            tip: "Could not read control geometry for Mount.".into(),
        })?;
    if w <= 0 || h <= 0 {
        return Err(Refused {
            reason: format!("degenerate extents {w}×{h}"),
            tip: "Control not visible — open its panel, then retry Mount.".into(),
        });
    }
    Ok(IRect::new(x, y, w, h))
}

fn live_from_proxy(node: &NodeRef, proxy: &AccessibleProxy<'_>, atspi_name: &str) -> LiveHandle {
    LiveHandle {
        node: node.clone(),
        atspi_name: atspi_name.into(),
        linux: LinuxLive {
            dest: proxy.inner().destination().to_owned(),
            path: proxy.inner().path().to_owned(),
        },
    }
}

fn key_of(proxy: &AccessibleProxy<'_>) -> LinuxLive {
    LinuxLive {
        dest: proxy.inner().destination().to_owned(),
        path: proxy.inner().path().to_owned(),
    }
}

async fn proxy_key(
    conn: &ZbusConnection,
    key: &LinuxLive,
) -> Result<AccessibleProxy<'static>, String> {
    AccessibleProxy::builder(conn)
        .destination(key.dest.clone())
        .map_err(|e| e.to_string())?
        .path(key.path.clone())
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())
}

async fn proxy_owned(
    conn: &ZbusConnection,
    obj: &ObjectRefOwned,
) -> Result<AccessibleProxy<'static>, String> {
    let Some(name) = obj.name_as_str() else {
        return Err("null accessible name".into());
    };
    let path = obj.path_as_str();
    if path == "/org/a11y/atspi/null" {
        return Err("null accessible path".into());
    }
    AccessibleProxy::builder(conn)
        .destination(name.to_owned())
        .map_err(|e| e.to_string())?
        .path(path.to_owned())
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())
}

async fn find_app_root(
    conn: &ZbusConnection,
    app: &str,
) -> Result<AccessibleProxy<'static>, String> {
    let desktop = AccessibleProxy::builder(conn)
        .destination("org.a11y.atspi.Registry")
        .map_err(|e| e.to_string())?
        .path("/org/a11y/atspi/accessible/root")
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    let children = desktop.get_children().await.map_err(|e| e.to_string())?;
    let app_l = app.to_lowercase();
    let app_compact = compact_name(&app_l);
    for child in children {
        let Ok(proxy) = proxy_owned(conn, &child).await else {
            continue;
        };
        let name = proxy.name().await.unwrap_or_default();
        let name_l = name.to_lowercase();
        if name == app || name_l == app_l || compact_name(&name_l) == app_compact {
            return Ok(proxy);
        }
    }
    Err(format!("AT-SPI app {app:?} not found"))
}

#[proxy(
    interface = "org.a11y.Bus",
    default_service = "org.a11y.Bus",
    default_path = "/org/a11y/bus"
)]
trait A11yBus {
    #[zbus(name = "GetAddress")]
    fn get_address(&self) -> zbus::Result<String>;
}

async fn a11y_connection() -> zbus::Result<ZbusConnection> {
    let session = ZbusConnection::session().await?;
    let bus = A11yBusProxy::new(&session).await?;
    let address = bus.get_address().await?;
    zbus::connection::Builder::address(address.as_str())?
        .build()
        .await
}

fn compact_name(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn block_on<F: Future>(fut: F) -> F::Output {
    async_io::block_on(fut)
}

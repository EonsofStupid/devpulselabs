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
) -> Result<InvokeEffect, Refused> {
    block_on(invoke_async(handle, action_name, agency))
}

pub(crate) fn live_extents(handle: &LiveHandle) -> Result<IRect, Refused> {
    block_on(extents_async(handle))
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
        if *idx as i32 >= count {
            return recover_under_parent(&conn, parent_for_search.as_ref(), node, step).await;
        }
        let child = current
            .get_child_at_index(*idx as i32)
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
        return Ok(live_from_proxy(node, &current));
    }

    // Electron reparent / index drift — bounded search under parent.
    recover_under_parent(&conn, parent_for_search.as_ref(), node, indices.len()).await
}

async fn recover_under_parent(
    conn: &ZbusConnection,
    parent: Option<&LinuxLive>,
    node: &NodeRef,
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
    if let Some(found) = find_named_under(conn, &parent_proxy, node, 0, 4).await? {
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
        return Ok(Some(live_from_proxy(expected, node)));
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
) -> Result<InvokeEffect, Refused> {
    let conn = a11y_connection().await.map_err(|e| Refused {
        reason: format!("a11y bus unavailable: {e}"),
        tip: "Accessibility must be on for Coach.".into(),
    })?;
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
    Ok(InvokeEffect {
        action: actions[index].name.clone(),
        agency,
    })
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

fn live_from_proxy(node: &NodeRef, proxy: &AccessibleProxy<'_>) -> LiveHandle {
    LiveHandle {
        node: node.clone(),
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

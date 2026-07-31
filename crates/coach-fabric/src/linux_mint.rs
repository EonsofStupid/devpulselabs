//! Linux AT-SPI fabric mint — budgeted BFS for one selected app.
//!
//! Uses `ChildCount` before any child fetch. Never calls `get_children` /
//! `get_child_at_index` when the count exceeds the budget cap.

use crate::walk::{walk_budgeted, NodeSource, WalkParams};
use crate::{MintBudget, MintOutcome, MintProgress, Refused};
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::action::ActionProxy;
use atspi::ObjectRefOwned;
use dpl_coach_inventory::{Complexity, DesktopId};
use std::future::Future;
use std::sync::atomic::AtomicBool;
use zbus::proxy;
use zbus::Connection as ZbusConnection;

struct AtspiTree {
    conn: ZbusConnection,
}

#[derive(Clone)]
struct NodeKey {
    dest: zbus::names::BusName<'static>,
    path: zbus::zvariant::ObjectPath<'static>,
}

impl NodeSource for AtspiTree {
    type Id = NodeKey;

    fn describe(&self, id: &Self::Id) -> (String, String) {
        block_on(async {
            let Ok(proxy) = proxy_key(&self.conn, id).await else {
                return (String::new(), "unknown".into());
            };
            let name = proxy.name().await.unwrap_or_default();
            let role = proxy
                .get_role_name()
                .await
                .unwrap_or_else(|_| "unknown".into());
            (name, role)
        })
    }

    fn child_count(&self, id: &Self::Id) -> u32 {
        block_on(async {
            let Ok(proxy) = proxy_key(&self.conn, id).await else {
                return 0;
            };
            match proxy.child_count().await {
                Ok(n) if n > 0 => n as u32,
                _ => 0,
            }
        })
    }

    fn child_at(&self, id: &Self::Id, index: u32) -> Option<Self::Id> {
        block_on(async {
            let proxy = proxy_key(&self.conn, id).await.ok()?;
            let child = proxy.get_child_at_index(index as i32).await.ok()?;
            key_for(&child)
        })
    }

    fn actions(&self, id: &Self::Id) -> Vec<String> {
        block_on(async {
            let Ok(proxy) = proxy_key(&self.conn, id).await else {
                return Vec::new();
            };
            let Ok(builder) =
                ActionProxy::builder(&self.conn).destination(proxy.inner().destination())
            else {
                return Vec::new();
            };
            let Ok(builder) = builder.path(proxy.inner().path()) else {
                return Vec::new();
            };
            let Ok(action) = builder.build().await else {
                return Vec::new();
            };
            match action.get_actions().await {
                Ok(list) => list.into_iter().map(|a| a.name).collect(),
                Err(_) => Vec::new(),
            }
        })
    }
}

pub fn mint_blocking(
    atspi_name: &str,
    target_id: DesktopId,
    complexity: Complexity,
    budget: MintBudget,
    seed_name: Option<&str>,
    cancel: Option<&AtomicBool>,
    on_progress: impl FnMut(MintProgress),
) -> MintOutcome {
    let conn = match block_on(a11y_connection()) {
        Ok(c) => c,
        Err(e) => {
            return MintOutcome::Refused(Refused {
                reason: format!("a11y bus unavailable: {e}"),
                tip: "Accessibility must be on for Coach — check Settings → Accessibility.".into(),
            });
        }
    };

    let root = match block_on(find_app_root(&conn, atspi_name)) {
        Ok(r) => r,
        Err(_) => {
            return MintOutcome::Refused(Refused {
                reason: format!("AT-SPI app {atspi_name:?} not found"),
                tip: format!("Open {atspi_name}, then Build again."),
            });
        }
    };

    let key = NodeKey {
        dest: root.inner().destination().to_owned(),
        path: root.inner().path().to_owned(),
    };
    let tree = AtspiTree { conn };
    walk_budgeted(
        &tree,
        key,
        WalkParams {
            target_id,
            atspi_name: atspi_name.into(),
            complexity,
            budget,
            seed_name,
            cancel,
        },
        on_progress,
    )
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
    // Desktop-root children only for locate — then deep walk is budgeted BFS.
    let children = desktop.get_children().await.map_err(|e| e.to_string())?;
    let app_l = app.to_lowercase();
    let app_compact = compact_name(&app_l);
    for child in children {
        let Some(proxy) = proxy_for(conn, &child).await? else {
            continue;
        };
        let name = proxy.name().await.unwrap_or_default();
        let name_l = name.to_lowercase();
        let name_compact = compact_name(&name_l);
        if name == app || name_l == app_l || name_compact == app_compact {
            let dest = proxy.inner().destination().to_owned();
            let path = proxy.inner().path().to_owned();
            return AccessibleProxy::builder(conn)
                .destination(dest)
                .map_err(|e| e.to_string())?
                .path(path)
                .map_err(|e| e.to_string())?
                .build()
                .await
                .map_err(|e| e.to_string());
        }
    }
    Err(format!("refused: AT-SPI app {app:?} not found"))
}

async fn proxy_for<'a>(
    conn: &'a ZbusConnection,
    obj: &'a ObjectRefOwned,
) -> Result<Option<AccessibleProxy<'a>>, String> {
    let Some(name) = obj.name_as_str() else {
        return Ok(None);
    };
    let path = obj.path_as_str();
    if path == "/org/a11y/atspi/null" {
        return Ok(None);
    }
    let proxy = AccessibleProxy::builder(conn)
        .destination(name)
        .map_err(|e| e.to_string())?
        .path(path)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(proxy))
}

async fn proxy_key(
    conn: &ZbusConnection,
    key: &NodeKey,
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

fn key_for(obj: &ObjectRefOwned) -> Option<NodeKey> {
    let name = obj.name_as_str()?;
    let path = obj.path_as_str();
    if path == "/org/a11y/atspi/null" {
        return None;
    }
    let dest = zbus::names::BusName::try_from(name.to_owned()).ok()?;
    let path = zbus::zvariant::ObjectPath::try_from(path.to_owned()).ok()?;
    Some(NodeKey { dest, path })
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

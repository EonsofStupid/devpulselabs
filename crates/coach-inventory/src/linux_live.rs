//! Phase 1 — AT-SPI desktop-root census only.
//!
//! **Forbidden:** `GetChildren` on any node below the desktop root.
//! Deep walks freeze apps (LibreOffice Calc / fama11y-tree landmine).

use crate::LiveApp;
use atspi::proxy::accessible::AccessibleProxy;
use atspi::ObjectRefOwned;
use std::future::Future;
use zbus::proxy;
use zbus::Connection as ZbusConnection;

pub fn live_census_blocking() -> Result<Vec<LiveApp>, String> {
    block_on(live_census())
}

async fn live_census() -> Result<Vec<LiveApp>, String> {
    let conn = a11y_connection().await.map_err(|e| e.to_string())?;
    let desktop = AccessibleProxy::builder(&conn)
        .destination("org.a11y.atspi.Registry")
        .map_err(|e| e.to_string())?
        .path("/org/a11y/atspi/accessible/root")
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    // Desktop root ONLY — never recurse into application children here.
    let children = desktop.get_children().await.map_err(|e| e.to_string())?;
    let mut apps = Vec::with_capacity(children.len());
    for child in children {
        let Some(proxy) = proxy_for(&conn, &child).await? else {
            continue;
        };
        let name = proxy.name().await.unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        // PID is optional in C2d — avoid extra interfaces; join uses name.
        apps.push(LiveApp { name, pid: None });
    }
    Ok(apps)
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

fn block_on<F: Future>(fut: F) -> F::Output {
    async_io::block_on(fut)
}

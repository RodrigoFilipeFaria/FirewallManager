use zbus::{Connection, Result, proxy};
use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, OwnedObjectPath};

pub type ServiceSettings = (
    String,
    String,
    String,
    Vec<(String, String)>,
    Vec<String>,
    HashMap<String, String>,
    Vec<String>,
    Vec<(String, String)>,
);

#[proxy(
    interface = "org.fedoraproject.FirewallD1",
    default_service = "org.fedoraproject.FirewallD1",
    default_path = "/org/fedoraproject/FirewallD1"
)]
trait Firewalld {
    #[zbus(name = "getDefaultZone")]
    fn get_default_zone(&self) -> zbus::Result<String>;

    #[zbus(property, name = "state")]
    fn get_state(&self) -> zbus::Result<String>;

    #[zbus(name = "listServices")]
    fn list_services(&self) -> zbus::Result<Vec<String>>;

    #[zbus(name = "reload")]
    fn reload(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.fedoraproject.FirewallD1.zone",
    default_service = "org.fedoraproject.FirewallD1",
    default_path = "/org/fedoraproject/FirewallD1"
)]
trait FirewalldZone {
    #[zbus(name = "getZones")]
    fn get_zones(&self) -> zbus::Result<Vec<String>>;

    #[zbus(name = "getZoneSettings2")]
    fn get_zone_settings(&self, zone: &str) -> zbus::Result<HashMap<String, OwnedValue>>;
}

#[proxy(
    interface = "org.fedoraproject.FirewallD1.config",
    default_service = "org.fedoraproject.FirewallD1",
    default_path = "/org/fedoraproject/FirewallD1/config"
)]
trait FirewalldConfig {
    #[zbus(name = "listServices")]
    fn list_services(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[zbus(name = "addService")]
    fn add_service(
        &self,
        service: &str,
        settings: (
            &str,
            &str,
            &str,
            Vec<(String, String)>,
            Vec<String>,
            HashMap<String, String>,
        ),
    ) -> zbus::Result<OwnedObjectPath>;

    #[zbus(name = "getServiceByName")]
    fn get_service_by_name(&self, service: &str) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.fedoraproject.FirewallD1.config.service",
    default_service = "org.fedoraproject.FirewallD1",
)]
trait FirewalldConfigService {
    #[zbus(name = "getSettings")]
    fn get_settings(&self) -> zbus::Result<ServiceSettings>;

    #[zbus(name = "update")]
    fn update(
        &self,
        settings: (
            &str,
            &str,
            &str,
            Vec<(String, String)>,
            Vec<String>,
            HashMap<String, String>,
        ),
    ) -> zbus::Result<()>;

    #[zbus(name = "remove")]
    fn remove(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Systemd {
    #[zbus(name = "StopUnit", allow_interactive_auth)]
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;

    #[zbus(name = "StartUnit", allow_interactive_auth)]
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
}

// ── Public API ────────────────────────────────────────────────────────────────

pub async fn fetch_default_zone() -> Result<String> {
    let connection = Connection::system().await?;
    let proxy = FirewalldProxy::new(&connection).await?;
    proxy.get_default_zone().await
}

pub async fn fetch_state() -> Result<String> {
    let connection = Connection::system().await?;
    let proxy = FirewalldProxy::new(&connection).await?;
    proxy.get_state().await
}

pub async fn fetch_zones() -> Result<Vec<String>> {
    let connection = Connection::system().await?;
    let proxy = FirewalldZoneProxy::new(&connection).await?;
    proxy.get_zones().await
}

pub async fn fetch_zone_settings(zone_name: &str) -> Result<HashMap<String, OwnedValue>> {
    let connection = Connection::system().await?;
    let proxy = FirewalldZoneProxy::new(&connection).await?;
    proxy.get_zone_settings(zone_name).await
}

pub async fn fetch_services() -> Result<Vec<String>> {
    let connection = Connection::system().await?;
    let proxy = FirewalldProxy::new(&connection).await?;
    let mut services = proxy.list_services().await?;
    services.sort();
    Ok(services)
}

pub async fn fetch_service_settings(service_name: &str) -> Result<ServiceSettings> {
    let connection = Connection::system().await?;
    let config = FirewalldConfigProxy::new(&connection).await?;
    let path = config.get_service_by_name(service_name).await?;
    let svc = FirewalldConfigServiceProxy::builder(&connection)
        .path(path)?
        .build()
        .await?;
    svc.get_settings().await
}

pub async fn add_service(
    name: &str,
    description: &str,
    ports: Vec<(String, String)>,
) -> Result<()> {
    let connection = Connection::system().await?;
    let proxy = FirewalldConfigProxy::new(&connection).await?;
    proxy
        .add_service(
            name,
            ("", name, description, ports, vec![], HashMap::new()),
        )
        .await?;
    let runtime = FirewalldProxy::new(&connection).await?;
    runtime.reload().await?;
    Ok(())
}

pub async fn remove_service(service_name: &str) -> Result<()> {
    let connection = Connection::system().await?;
    let config = FirewalldConfigProxy::new(&connection).await?;
    let path = config.get_service_by_name(service_name).await?;
    let svc = FirewalldConfigServiceProxy::builder(&connection)
        .path(path)?
        .build()
        .await?;
    svc.remove().await?;
    let runtime = FirewalldProxy::new(&connection).await?;
    runtime.reload().await?;
    Ok(())
}

pub async fn edit_service(
    service_name: &str,
    new_description: &str,
    new_ports: Vec<(String, String)>,
) -> Result<()> {
    let connection = Connection::system().await?;
    let config = FirewalldConfigProxy::new(&connection).await?;
    let path = config.get_service_by_name(service_name).await?;
    let svc = FirewalldConfigServiceProxy::builder(&connection)
        .path(path)?
        .build()
        .await?;
    let (version, short, _old_desc, _old_ports, modules, destinations, _includes, _src_ports) =
        svc.get_settings().await?;
    svc.update((
        &version,
        &short,
        new_description,
        new_ports,
        modules,
        destinations,
    ))
    .await?;
    let runtime = FirewalldProxy::new(&connection).await?;
    runtime.reload().await?;
    Ok(())
}

pub async fn disable_firewall() -> Result<()> {
    let connection = Connection::system().await?;
    let proxy = SystemdProxy::new(&connection).await?;
    proxy.stop_unit("firewalld.service", "replace").await?;
    Ok(())
}

pub async fn enable_firewall() -> Result<()> {
    let connection = Connection::system().await?;
    let proxy = SystemdProxy::new(&connection).await?;
    proxy.start_unit("firewalld.service", "replace").await?;
    Ok(())
}


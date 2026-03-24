use zbus::{Connection, Result, proxy};
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

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
}

pub async fn fetch_default_zone() -> Result<String> {
    let connection = Connection::system().await?;
    let proxy = FirewalldProxy::new(&connection).await?;
    let zone = proxy.get_default_zone().await?;
    Ok(zone)
}

pub async fn fetch_state() -> Result<String> {
    let connection = Connection::system().await?;
    let proxy = FirewalldProxy::new(&connection).await?;
    let state = proxy.get_state().await?;
    Ok(state)
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

pub async fn fetch_zones() -> Result<Vec<String>> {
    let connection = Connection::system().await?;
    let proxy = FirewalldZoneProxy::new(&connection).await?;
    let zones = proxy.get_zones().await?;
    Ok(zones)
}

pub async fn fetch_zone_settings(zone_name: &str) -> Result<HashMap<String, OwnedValue>> {
    let connection = Connection::system().await?;
    let proxy = FirewalldZoneProxy::new(&connection).await?;
    let settings = proxy.get_zone_settings(zone_name).await?;
    Ok(settings)
}

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Systemd {
    // The magic switch is built right into the macro!
    #[zbus(name = "StopUnit", allow_interactive_auth)]
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    #[zbus(name = "StartUnit", allow_interactive_auth)]
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

pub async fn disable_firewall() -> Result<()> {
    let connection = Connection::system().await?;
    let proxy = SystemdProxy::new(&connection).await?;

    // Now we just call it normally, and the macro handles the flags
    proxy.stop_unit("firewalld.service", "replace").await?;

    Ok(())
}

pub async fn enable_firewall() -> Result<()> {
    let connection = Connection::system().await?;
    let proxy = SystemdProxy::new(&connection).await?;

    proxy.start_unit("firewalld.service", "replace").await?;

    Ok(())
}

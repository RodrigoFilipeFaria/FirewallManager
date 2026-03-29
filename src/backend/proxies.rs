use std::collections::HashMap;
use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

pub type ServiceSettings = (
    String, String, String, Vec<(String, String)>,
    Vec<String>, HashMap<String, String>, Vec<String>, Vec<(String, String)>,
);

#[proxy(
    interface = "org.fedoraproject.FirewallD1",
    default_service = "org.fedoraproject.FirewallD1",
    default_path = "/org/fedoraproject/FirewallD1"
)]
pub(crate) trait Firewalld {
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
pub(crate) trait FirewalldZone {
    #[zbus(name = "getZones")]
    fn get_zones(&self) -> zbus::Result<Vec<String>>;
    #[zbus(name = "getZoneSettings2")]
    fn get_zone_settings(&self, zone: &str) -> zbus::Result<HashMap<String, OwnedValue>>;
    #[zbus(name = "getInterfaces")]
    fn get_interfaces(&self, zone: &str) -> zbus::Result<Vec<String>>;
    #[zbus(name = "getZoneOfInterface")]
    fn get_zone_of_interface(&self, interface: &str) -> zbus::Result<String>;
    #[zbus(name = "changeZoneOfInterface")]
    fn set_zone_interface(&self, zone: &str, interface: &str) -> zbus::Result<String>;
    #[zbus(name = "addService")]
    fn add_service_zone(&self, zone: &str, service: &str, timeout: i32) -> zbus::Result<String>;
    #[zbus(name = "removeService")]
    fn remove_service_zone(&self, zone: &str, service: &str) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.fedoraproject.FirewallD1.config",
    default_service = "org.fedoraproject.FirewallD1",
    default_path = "/org/fedoraproject/FirewallD1/config"
)]
pub(crate) trait FirewalldConfig {
    #[zbus(name = "listServices")]
    fn list_services(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    #[zbus(name = "addService")]
    fn add_service(&self, service: &str, settings: (&str, &str, &str, Vec<(String, String)>, Vec<String>, HashMap<String, String>)) -> zbus::Result<OwnedObjectPath>;
    #[zbus(name = "getServiceByName")]
    fn get_service_by_name(&self, service: &str) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.fedoraproject.FirewallD1.config.service",
    default_service = "org.fedoraproject.FirewallD1",
)]
pub(crate) trait FirewalldConfigService {
    #[zbus(name = "getSettings")]
    fn get_settings(&self) -> zbus::Result<ServiceSettings>;
    #[zbus(name = "update")]
    fn update(&self, settings: (&str, &str, &str, Vec<(String, String)>, Vec<String>, HashMap<String, String>)) -> zbus::Result<()>;
    #[zbus(name = "remove")]
    fn remove(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub(crate) trait Systemd {
    #[zbus(name = "StopUnit", allow_interactive_auth)]
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    #[zbus(name = "StartUnit", allow_interactive_auth)]
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
}

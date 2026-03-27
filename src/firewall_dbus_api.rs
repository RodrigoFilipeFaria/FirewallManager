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

    #[zbus(name = "getInterfaces")]
    fn get_interfaces(&self, zone: &str) -> zbus::Result<Vec<String>>;

    #[zbus(name = "getZoneOfInterface")]
    fn get_zone_of_interface(&self, interface: &str) -> zbus::Result<String>;

    #[zbus(name = "changeZoneOfInterface")]
    fn set_zone_interface(&self, zone : &str, interface: &str) -> zbus::Result<String>;

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

#[derive(Clone)]
pub struct FirewallClient {
    connection: Connection,
}

impl FirewallClient {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    pub async fn fetch_default_zone(&self) -> Result<String> {
        let proxy = FirewalldProxy::new(&self.connection).await?;
        proxy.get_default_zone().await
    }

    pub async fn fetch_state(&self) -> Result<String> {
        let proxy = FirewalldProxy::new(&self.connection).await?;
        proxy.get_state().await
    }

    pub async fn fetch_zones(&self) -> Result<Vec<String>> {
        let proxy = FirewalldZoneProxy::new(&self.connection).await?;
        proxy.get_zones().await
    }

    pub async fn fetch_zone_settings(&self, zone_name: &str) -> Result<HashMap<String, OwnedValue>> {
        let proxy = FirewalldZoneProxy::new(&self.connection).await?;
        proxy.get_zone_settings(zone_name).await
    }

    pub async fn change_zone_interface(&self, zone: &str, interface: &str) -> Result<String> {
        let proxy = FirewalldZoneProxy::new(&self.connection).await?;
        let change = proxy.set_zone_interface(zone, interface).await?;
        Ok(change)
    }

    pub async fn fetch_interfaces(&self) -> Result<Vec<String>> {
        let mut interfaces = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name != "lo" {
                        interfaces.push(name);
                    }
                }
            }
        }
        interfaces.sort();
        Ok(interfaces)
    }

    pub async fn fetch_zone_of_interface(&self, interface: &str) -> Result<String> {
        let proxy = FirewalldZoneProxy::new(&self.connection).await?;
        proxy.get_zone_of_interface(interface).await
    }

    pub async fn add_service_to_zone(&self, zone: &str, service: &str, timeout: i32) -> Result<String> {
        let proxy = FirewalldZoneProxy::new(&self.connection).await?;
        let services = proxy.add_service_zone(zone, service, timeout).await?;
        Ok(services)
    }

    pub async fn remove_service_to_zone(&self, zone: &str, service: &str) -> Result<String> {
        let proxy = FirewalldZoneProxy::new(&self.connection).await?;
        let services = proxy.remove_service_zone(zone, service).await?;
        Ok(services)
    }

    pub async fn fetch_services(&self) -> Result<Vec<String>> {
        let proxy = FirewalldProxy::new(&self.connection).await?;
        let mut services = proxy.list_services().await?;
        services.sort();
        Ok(services)
    }

    pub async fn fetch_service_settings(&self, service_name: &str) -> Result<ServiceSettings> {
        let config = FirewalldConfigProxy::new(&self.connection).await?;
        let path = config.get_service_by_name(service_name).await?;
        let svc = FirewalldConfigServiceProxy::builder(&self.connection)
            .path(path)?
            .build()
            .await?;
        svc.get_settings().await
    }

    pub async fn add_service(
        &self,
        name: &str,
        description: &str,
        ports: Vec<(String, String)>,
    ) -> Result<()> {
        let proxy = FirewalldConfigProxy::new(&self.connection).await?;
        proxy
            .add_service(
                name,
                ("", name, description, ports, vec![], HashMap::new()),
            )
            .await?;
        let runtime = FirewalldProxy::new(&self.connection).await?;
        runtime.reload().await?;
        Ok(())
    }

    pub async fn remove_service(&self, service_name: &str) -> Result<()> {
        let config = FirewalldConfigProxy::new(&self.connection).await?;
        let path = config.get_service_by_name(service_name).await?;
        let svc = FirewalldConfigServiceProxy::builder(&self.connection)
            .path(path)?
            .build()
            .await?;
        svc.remove().await?;
        let runtime = FirewalldProxy::new(&self.connection).await?;
        runtime.reload().await?;
        Ok(())
    }

    pub async fn edit_service(
        &self,
        service_name: &str,
        new_description: &str,
        new_ports: Vec<(String, String)>,
    ) -> Result<()> {
        let config = FirewalldConfigProxy::new(&self.connection).await?;
        let path = config.get_service_by_name(service_name).await?;
        let svc = FirewalldConfigServiceProxy::builder(&self.connection)
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

        let runtime = FirewalldProxy::new(&self.connection).await?;
        runtime.reload().await?;
        Ok(())
    }

    pub async fn disable_firewall(&self) -> Result<()> {
        let proxy = SystemdProxy::new(&self.connection).await?;
        proxy.stop_unit("firewalld.service", "replace").await?;
        Ok(())
    }

    pub async fn enable_firewall(&self) -> Result<()> {
        let proxy = SystemdProxy::new(&self.connection).await?;
        proxy.start_unit("firewalld.service", "replace").await?;
        Ok(())
    }
}

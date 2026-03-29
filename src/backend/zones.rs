use zbus::Result;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;
use super::client::FirewallClient;
use super::proxies::FirewalldZoneProxy;

impl FirewallClient {
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
        proxy.add_service_zone(zone, service, timeout).await
    }

    pub async fn remove_service_to_zone(&self, zone: &str, service: &str) -> Result<String> {
        let proxy = FirewalldZoneProxy::new(&self.connection).await?;
        proxy.remove_service_zone(zone, service).await
    }
}

use zbus::Result;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;
use super::client::FirewallClient;
use super::proxies::{FirewalldZoneProxy, FirewalldConfigProxy, FirewalldConfigZoneProxy};

impl FirewallClient {
    pub async fn fetch_zones(&self) -> Result<Vec<String>> {
        if self.is_permanent_mode() {
            let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
            let mut zones = config_proxy.get_zone_names().await?;
            zones.sort();
            Ok(zones)
        } else {
            let proxy = FirewalldZoneProxy::new(&self.connection).await?;
            let mut zones = proxy.get_zones().await?;
            zones.sort();
            Ok(zones)
        }
    }

    pub async fn fetch_zone_settings(&self, zone_name: &str) -> Result<HashMap<String, OwnedValue>> {
        if self.is_permanent_mode() {
            let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
            let path = config_proxy.get_zone_by_name(zone_name).await?;
            let zone_proxy = FirewalldConfigZoneProxy::builder(&self.connection)
                .path(path)?
                .build()
                .await?;
            zone_proxy.get_settings().await
        } else {
            let proxy = FirewalldZoneProxy::new(&self.connection).await?;
            proxy.get_zone_settings(zone_name).await
        }
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
        if self.is_permanent_mode() {
            let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
            config_proxy.get_zone_of_interface(interface).await
        } else {
            let proxy = FirewalldZoneProxy::new(&self.connection).await?;
            proxy.get_zone_of_interface(interface).await
        }
    }

    pub async fn change_zone_interface(&self, zone: &str, interface: &str) -> Result<String> {
        if self.is_permanent_mode() {
            let old_zone = self.fetch_zone_of_interface(interface).await?;
            if !old_zone.is_empty() {
                let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
                if let Ok(old_path) = config_proxy.get_zone_by_name(&old_zone).await {
                    if let Ok(old_zp) = FirewalldConfigZoneProxy::builder(&self.connection).path(old_path).unwrap().build().await {
                        let _ = old_zp.remove_interface(interface).await;
                    }
                }
            }
            
            let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
            let new_path = config_proxy.get_zone_by_name(zone).await?;
            let new_zp = FirewalldConfigZoneProxy::builder(&self.connection).path(new_path)?.build().await?;
            new_zp.add_interface(interface).await?;
            Ok(zone.to_string())
        } else {
            let proxy = FirewalldZoneProxy::new(&self.connection).await?;
            let change = proxy.set_zone_interface(zone, interface).await?;
            self.mark_unsaved();
            Ok(change)
        }
    }

    pub async fn add_service_to_zone(&self, zone: &str, service: &str, timeout: i32) -> Result<String> {
        if self.is_permanent_mode() {
            let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
            let path = config_proxy.get_zone_by_name(zone).await?;
            let zp = FirewalldConfigZoneProxy::builder(&self.connection).path(path)?.build().await?;
            zp.add_service(service).await?;
            Ok("".to_string())
        } else {
            let proxy = FirewalldZoneProxy::new(&self.connection).await?;
            let res = proxy.add_service_zone(zone, service, timeout).await?;
            self.mark_unsaved();
            Ok(res)
        }
    }

    pub async fn remove_service_to_zone(&self, zone: &str, service: &str) -> Result<String> {
        if self.is_permanent_mode() {
            let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
            let path = config_proxy.get_zone_by_name(zone).await?;
            let zp = FirewalldConfigZoneProxy::builder(&self.connection).path(path)?.build().await?;
            zp.remove_service(service).await?;
            Ok("".to_string())
        } else {
            let proxy = FirewalldZoneProxy::new(&self.connection).await?;
            let res = proxy.remove_service_zone(zone, service).await?;
            self.mark_unsaved();
            Ok(res)
        }
    }
}

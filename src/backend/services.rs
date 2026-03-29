use zbus::Result;
use std::collections::HashMap;
use super::client::FirewallClient;
use super::proxies::{FirewalldProxy, FirewalldConfigProxy, FirewalldConfigServiceProxy, ServiceSettings};

impl FirewallClient {
    pub async fn fetch_services(&self) -> Result<Vec<String>> {
        if self.is_permanent_mode() {
            let config_proxy = FirewalldConfigProxy::new(&self.connection).await?;
            let mut services = config_proxy.get_service_names().await?;
            services.sort();
            Ok(services)
        } else {
            let proxy = FirewalldProxy::new(&self.connection).await?;
            let mut services = proxy.list_services().await?;
            services.sort();
            Ok(services)
        }
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

        Ok(())
    }
}

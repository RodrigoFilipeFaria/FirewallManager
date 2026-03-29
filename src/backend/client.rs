use zbus::{Connection, Result};
use super::proxies::{FirewalldProxy, SystemdProxy};

#[derive(Clone)]
pub struct FirewallClient {
    pub(crate) connection: Connection,
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

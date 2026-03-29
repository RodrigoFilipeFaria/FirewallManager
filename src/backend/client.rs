use zbus::{Connection, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use super::proxies::{FirewalldProxy, SystemdProxy};

#[derive(Clone)]
pub struct FirewallClient {
    pub(crate) connection: Connection,
    pub is_permanent: Arc<AtomicBool>,
}

impl FirewallClient {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection, is_permanent: Arc::new(AtomicBool::new(false)) })
    }

    pub fn set_permanent_mode(&self, permanent: bool) {
        self.is_permanent.store(permanent, Ordering::SeqCst);
    }

    pub fn is_permanent_mode(&self) -> bool {
        self.is_permanent.load(Ordering::SeqCst)
    }

    pub async fn reload_firewall(&self) -> Result<()> {
        let proxy = FirewalldProxy::new(&self.connection).await?;
        proxy.reload().await
    }

    pub async fn runtime_to_permanent(&self) -> Result<()> {
        let proxy = FirewalldProxy::new(&self.connection).await?;
        proxy.runtime_to_permanent().await
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

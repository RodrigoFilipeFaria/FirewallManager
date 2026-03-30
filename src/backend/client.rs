use zbus::{Connection, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::watch;
use super::proxies::{FirewalldProxy, SystemdProxy};

#[derive(Clone)]
pub struct FirewallClient {
    pub(crate) connection: Connection,
    pub is_permanent: Arc<AtomicBool>,
    pub unsaved_changes_tx: Arc<watch::Sender<bool>>,
    pub unsaved_changes_rx: watch::Receiver<bool>,
}

impl FirewallClient {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        let (tx, rx) = watch::channel(false);
        Ok(Self { 
            connection, 
            is_permanent: Arc::new(AtomicBool::new(false)),
            unsaved_changes_tx: Arc::new(tx),
            unsaved_changes_rx: rx,
        })
    }

    pub fn mark_unsaved(&self) {
        if !self.is_permanent_mode() {
            let _ = self.unsaved_changes_tx.send(true);
        }
    }

    pub fn clear_unsaved(&self) {
        let _ = self.unsaved_changes_tx.send(false);
    }

    pub fn set_permanent_mode(&self, permanent: bool) {
        self.is_permanent.store(permanent, Ordering::SeqCst);
    }

    pub fn is_permanent_mode(&self) -> bool {
        self.is_permanent.load(Ordering::SeqCst)
    }

    pub async fn reload_firewall(&self) -> Result<()> {
        let proxy = FirewalldProxy::new(&self.connection).await?;
        proxy.reload().await?;
        self.clear_unsaved();
        Ok(())
    }

    pub async fn runtime_to_permanent(&self) -> Result<()> {
        let proxy = FirewalldProxy::new(&self.connection).await?;
        proxy.runtime_to_permanent().await?;
        self.clear_unsaved();
        Ok(())
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

use crate::config::{OutboundConfig, OutboundType};
use crate::error::{ProxyError, Result};
use crate::protocols::{
    BlackholeProtocol, DirectProtocol, Protocol, Socks5Protocol, VlessProtocol,
};
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::TcpStream;

#[async_trait]
pub trait OutboundConnector: Send + Sync {
    async fn connect(&self, target: SocketAddr) -> Result<TcpStream>;
}

// 旧的outbound实现已移动到protocols模块中

use std::collections::HashMap;
use std::sync::Arc;

pub struct OutboundManager {
    connectors: HashMap<String, Arc<dyn Protocol>>,
}

impl OutboundManager {
    pub fn from_configs(configs: &[OutboundConfig]) -> Result<Self> {
        let mut map: HashMap<String, Arc<dyn Protocol>> = HashMap::new();
        for cfg in configs {
            let name = cfg.name.clone();
            let protocol: Arc<dyn Protocol> = match &cfg.kind {
                OutboundType::Direct => Arc::new(DirectProtocol::new()),
                OutboundType::Blackhole => Arc::new(BlackholeProtocol::new()),
                OutboundType::Socks5 { address } => {
                    let addr: SocketAddr = address.parse().map_err(|e| ProxyError::Protocol(format!("Invalid socks5 address: {}", e)))?;
                    Arc::new(Socks5Protocol::with_server(addr))
                }
                OutboundType::Vless { address, uuid, tls } => {
                    let addr: SocketAddr = address.parse().map_err(|e| ProxyError::Protocol(format!("Invalid vless address: {}", e)))?;
                    Arc::new(VlessProtocol::with_config(addr, uuid.clone(), *tls))
                }
            };
            map.insert(name, protocol);
        }
        Ok(Self { connectors: map })
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Protocol>> {
        self.connectors.get(name).cloned()
    }
}

static mut GLOBAL_OUTBOUND_MANAGER: Option<OutboundManager> = None;

pub fn init_global_outbound_manager(cfgs: &[OutboundConfig]) -> Result<()> {
    let m = OutboundManager::from_configs(cfgs)?;
    unsafe { GLOBAL_OUTBOUND_MANAGER = Some(m); }
    Ok(())
}

pub fn get_global_outbound_manager() -> &'static OutboundManager {
    unsafe { GLOBAL_OUTBOUND_MANAGER.as_ref().expect("OutboundManager not initialized") }
}



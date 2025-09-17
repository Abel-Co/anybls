use std::net::SocketAddr;
use async_trait::async_trait;
use tokio::net::TcpStream;
use crate::error::{ProxyError, Result};
use crate::config::{OutboundConfig, OutboundType};

#[async_trait]
pub trait OutboundConnector: Send + Sync {
    async fn connect(&self, target: SocketAddr) -> Result<TcpStream>;
}

pub struct DirectOutbound;

#[async_trait]
impl OutboundConnector for DirectOutbound {
    async fn connect(&self, target: SocketAddr) -> Result<TcpStream> {
        TcpStream::connect(target).await.map_err(|e| ProxyError::ConnectionFailed(e.to_string()))
    }
}

pub struct Socks5Outbound {
    pub server_addr: SocketAddr,
}

#[async_trait]
impl OutboundConnector for Socks5Outbound {
    async fn connect(&self, target: SocketAddr) -> Result<TcpStream> {
        // Minimal: establish TCP to SOCKS5 server, send connect for target
        let mut stream = TcpStream::connect(self.server_addr).await
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;

        // Greeting: no auth
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        stream.write_all(&[0x05u8, 0x01, 0x00]).await?;
        let mut buf = [0u8; 2];
        stream.read_exact(&mut buf).await?;
        if buf != [0x05, 0x00] { return Err(ProxyError::Protocol("SOCKS5 auth failed".into())); }

        // Build connect request
        let mut req = Vec::with_capacity(32);
        req.push(0x05); // ver
        req.push(0x01); // cmd=connect
        req.push(0x00); // rsv
        match target.ip() {
            std::net::IpAddr::V4(ipv4) => {
                req.push(0x01);
                req.extend_from_slice(&ipv4.octets());
            }
            std::net::IpAddr::V6(ipv6) => {
                req.push(0x04);
                req.extend_from_slice(&ipv6.octets());
            }
        }
        req.extend_from_slice(&target.port().to_be_bytes());
        stream.write_all(&req).await?;

        // Read reply header (ver, rep, rsv, atyp)
        let mut head = [0u8; 4];
        stream.read_exact(&mut head).await?;
        if head[1] != 0x00 { return Err(ProxyError::ConnectionFailed(format!("SOCKS5 connect failed: {:x}", head[1]))); }
        // Consume bound addr per atyp
        let to_read = match head[3] { 0x01 => 4, 0x04 => 16, 0x03 => { let mut l=[0u8;1]; stream.read_exact(&mut l).await?; l[0] as usize }, _ => 0 };
        let mut addr = vec![0u8; to_read];
        if to_read>0 { stream.read_exact(&mut addr).await?; }
        let mut port = [0u8;2];
        stream.read_exact(&mut port).await?;

        Ok(stream)
    }
}

pub struct BlackholeOutbound;

#[async_trait]
impl OutboundConnector for BlackholeOutbound {
    async fn connect(&self, _target: SocketAddr) -> Result<TcpStream> {
        Err(ProxyError::ConnectionFailed("Blackhole outbound".into()))
    }
}

pub struct VlessOutbound {
    pub _server_addr: SocketAddr,
}

#[async_trait]
impl OutboundConnector for VlessOutbound {
    async fn connect(&self, _target: SocketAddr) -> Result<TcpStream> {
        Err(ProxyError::ConnectionFailed("VLESS not implemented".into()))
    }
}

use std::collections::HashMap;
use std::sync::Arc;

pub struct OutboundManager {
    connectors: HashMap<String, Arc<dyn OutboundConnector>>, 
}

impl OutboundManager {
    pub fn from_configs(configs: &[OutboundConfig]) -> Result<Self> {
        let mut map: HashMap<String, Arc<dyn OutboundConnector>> = HashMap::new();
        for cfg in configs {
            let name = cfg.name.clone();
            let connector: Arc<dyn OutboundConnector> = match &cfg.kind {
                OutboundType::Direct => Arc::new(DirectOutbound),
                OutboundType::Blackhole => Arc::new(BlackholeOutbound),
                OutboundType::Socks5 { address } => {
                    let addr: SocketAddr = address.parse().map_err(|e| ProxyError::Protocol(format!("Invalid socks5 address: {}", e)))?;
                    Arc::new(Socks5Outbound { server_addr: addr })
                }
                OutboundType::Vless { address, .. } => {
                    let addr: SocketAddr = address.parse().map_err(|e| ProxyError::Protocol(format!("Invalid vless address: {}", e)))?;
                    Arc::new(VlessOutbound { _server_addr: addr })
                }
            };
            map.insert(name, connector);
        }
        Ok(Self { connectors: map })
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn OutboundConnector>> {
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



use super::Protocol;
use async_trait::async_trait;
use std::net::SocketAddr;
use crate::error::{ProxyError, Result};
use tokio::net::TcpStream;

pub struct VlessProtocol {
    server_addr: Option<SocketAddr>,
    uuid: Option<String>,
    tls: bool,
}

impl VlessProtocol {
    pub fn new() -> Self {
        Self { 
            server_addr: None, 
            uuid: None, 
            tls: false 
        }
    }
    
    pub fn with_config(server_addr: SocketAddr, uuid: String, tls: bool) -> Self {
        Self { 
            server_addr: Some(server_addr), 
            uuid: Some(uuid), 
            tls 
        }
    }
}

#[async_trait]
impl Protocol for VlessProtocol {
    fn name(&self) -> &str {
        "vless"
    }
    
    async fn connect_outbound(&self, _target: SocketAddr) -> Result<TcpStream> {
        Err(ProxyError::Protocol("VLESS protocol not implemented yet".to_string()))
    }
    
    async fn start_inbound(&self, _bind_addr: SocketAddr) -> Result<()> {
        Err(ProxyError::Protocol("VLESS protocol not implemented yet".to_string()))
    }
}

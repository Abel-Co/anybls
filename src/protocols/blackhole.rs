use super::Protocol;
use crate::error::{ProxyError, Result};
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub struct BlackholeProtocol;

impl BlackholeProtocol {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Protocol for BlackholeProtocol {
    fn name(&self) -> &str {
        "blackhole"
    }

    async fn connect_outbound(&self, _target: SocketAddr) -> Result<TcpStream> {
        Err(ProxyError::ConnectionFailed("Blackhole outbound - connection dropped".to_string()))
    }

    async fn start_inbound(&self, _bind_addr: SocketAddr) -> Result<()> {
        // Blackhole作为inbound没有意义
        Err(ProxyError::Protocol("Blackhole protocol cannot be used as inbound".to_string()))
    }
}

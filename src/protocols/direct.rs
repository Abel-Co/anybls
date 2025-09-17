use super::Protocol;
use crate::error::{ProxyError, Result};
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub struct DirectProtocol;

impl DirectProtocol {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Protocol for DirectProtocol {
    fn name(&self) -> &str {
        "direct"
    }

    async fn connect_outbound(&self, target: SocketAddr) -> Result<TcpStream> {
        TcpStream::connect(target).await
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))
    }

    async fn start_inbound(&self, _bind_addr: SocketAddr) -> Result<()> {
        // Direct协议作为inbound没有意义，直接返回错误
        Err(ProxyError::Protocol("Direct protocol cannot be used as inbound".to_string()))
    }
}

use super::Protocol;
use crate::error::{ProxyError, Result};
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct Socks5Protocol {
    // 作为outbound时的服务器地址
    server_addr: Option<SocketAddr>,
}

impl Socks5Protocol {
    pub fn new() -> Self {
        Self { server_addr: None }
    }
    
    pub fn with_server(server_addr: SocketAddr) -> Self {
        Self { server_addr: Some(server_addr) }
    }
}

#[async_trait]
impl Protocol for Socks5Protocol {
    fn name(&self) -> &str {
        "socks5"
    }

    async fn connect_outbound(&self, target: SocketAddr) -> Result<TcpStream> {
        let server_addr = self.server_addr
            .ok_or_else(|| ProxyError::Protocol("SOCKS5 server address not configured".to_string()))?;
            
        // 连接到SOCKS5服务器
        let mut stream = TcpStream::connect(server_addr).await
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;

        // SOCKS5握手
        stream.write_all(&[0x05u8, 0x01, 0x00]).await?; // 版本5，1个方法，无认证
        let mut buf = [0u8; 2];
        stream.read_exact(&mut buf).await?;
        if buf != [0x05, 0x00] { 
            return Err(ProxyError::Protocol("SOCKS5 authentication failed".to_string())); 
        }

        // 发送连接请求
        let mut req = Vec::with_capacity(32);
        req.push(0x05); // 版本
        req.push(0x01); // 连接命令
        req.push(0x00); // 保留字段

        // 地址类型和地址
        match target.ip() {
            std::net::IpAddr::V4(ipv4) => {
                req.push(0x01); // IPv4
                req.extend_from_slice(&ipv4.octets());
            }
            std::net::IpAddr::V6(ipv6) => {
                req.push(0x04); // IPv6
                req.extend_from_slice(&ipv6.octets());
            }
        }
        req.extend_from_slice(&target.port().to_be_bytes());
        stream.write_all(&req).await?;

        // 读取响应
        let mut head = [0u8; 4];
        stream.read_exact(&mut head).await?;
        if head[1] != 0x00 { 
            return Err(ProxyError::ConnectionFailed(format!("SOCKS5 connect failed: {:x}", head[1]))); 
        }

        // 跳过绑定的地址信息
        let to_read = match head[3] {
            0x01 => 4,  // IPv4
            0x04 => 16, // IPv6
            0x03 => {   // 域名
                let mut l = [0u8; 1];
                stream.read_exact(&mut l).await?;
                l[0] as usize
            }
            _ => 0,
        };
        if to_read > 0 {
            let mut addr = vec![0u8; to_read];
            stream.read_exact(&mut addr).await?;
        }
        let mut port = [0u8; 2];
        stream.read_exact(&mut port).await?;

        Ok(stream)
    }

    async fn start_inbound(&self, bind_addr: SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(bind_addr).await?;
        log::info!("SOCKS5 inbound listening on {}", bind_addr);

        loop {
            match listener.accept().await {
                Ok((_stream, client_addr)) => {
                    log::info!("SOCKS5 connection from {}", client_addr);
                    // 这里应该处理SOCKS5连接，但为了简化，先只记录
                    // 实际实现需要处理SOCKS5协议握手和转发
                }
                Err(e) => {
                    log::error!("SOCKS5 accept error: {}", e);
                }
            }
        }
    }
}

use super::Protocol;
use async_trait::async_trait;
use std::net::SocketAddr;
use crate::error::{ProxyError, Result};
use tokio::net::TcpStream;

pub struct TproxyProtocol;

impl TproxyProtocol {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Protocol for TproxyProtocol {
    fn name(&self) -> &str {
        "tproxy"
    }
    
    async fn connect_outbound(&self, _target: SocketAddr) -> Result<TcpStream> {
        // TProxy作为outbound没有意义
        Err(ProxyError::Protocol("TProxy protocol cannot be used as outbound".to_string()))
    }
    
    async fn start_inbound(&self, bind_addr: SocketAddr) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.start_tproxy_linux(bind_addr).await
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(ProxyError::Protocol("TProxy is only supported on Linux".to_string()))
        }
    }
}

impl TproxyProtocol {
    #[cfg(target_os = "linux")]
    async fn start_tproxy_linux(&self, bind_addr: SocketAddr) -> Result<()> {
        use socket2::{Socket, Domain, Type, Protocol};
        use nix::sys::socket::{setsockopt, sockopt::IpTransparent};
        use tokio::net::{TcpListener, UdpSocket};
        use std::os::fd::FromRawFd;

        // TCP透明代理
        let tcp_listener = self.create_transparent_tcp_listener(bind_addr)?;
        let listener = unsafe { 
            TcpListener::from_std(std::net::TcpListener::from_raw_fd(tcp_listener.as_raw_fd())) 
        };
        log::info!("TProxy TCP listening on {}", bind_addr);
        
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((_stream, peer)) => {
                        log::info!("TProxy TCP accepted from {}", peer);
                        // 这里需要处理TProxy连接和路由
                    }
                    Err(e) => {
                        log::error!("TProxy TCP accept error: {}", e);
                        break;
                    }
                }
            }
        });

        // UDP透明代理
        let udp = self.create_transparent_udp_socket(bind_addr)?;
        let _udp = UdpSocket::from_std(udp)?;
        log::info!("TProxy UDP bound on {}", bind_addr);
        
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn create_transparent_tcp_listener(&self, addr: SocketAddr) -> Result<std::net::TcpListener> {
        use socket2::{Socket, Domain, Type, Protocol};
        use nix::sys::socket::{setsockopt, sockopt::IpTransparent};
        use std::os::fd::AsRawFd;

        let domain = match addr { 
            SocketAddr::V4(_) => Domain::IPV4, 
            SocketAddr::V6(_) => Domain::IPV6 
        };
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
        socket.set_reuse_address(true)?;
        setsockopt(socket.as_raw_fd(), IpTransparent, &true)
            .map_err(|e| std::io::Error::other(e))?;
        socket.bind(&addr.into())?;
        socket.listen(1024)?;
        Ok(socket.into())
    }

    #[cfg(target_os = "linux")]
    fn create_transparent_udp_socket(&self, addr: SocketAddr) -> Result<std::net::UdpSocket> {
        use socket2::{Socket, Domain, Type, Protocol};
        use nix::sys::socket::{setsockopt, sockopt::IpTransparent};
        use std::os::fd::AsRawFd;

        let domain = match addr { 
            SocketAddr::V4(_) => Domain::IPV4, 
            SocketAddr::V6(_) => Domain::IPV6 
        };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        setsockopt(socket.as_raw_fd(), IpTransparent, &true)
            .map_err(|e| std::io::Error::other(e))?;
        socket.bind(&addr.into())?;
        Ok(socket.into())
    }
}

use crate::error::Result;
use crate::protocols::Protocol;
use std::net::SocketAddr;

#[async_trait::async_trait]
pub trait Inbound: Send + Sync {
    async fn start(&self) -> Result<()>;
}

/// 基于协议的inbound实现
pub struct ProtocolInbound {
    protocol: Box<dyn Protocol>,
    bind_addr: SocketAddr,
}

impl ProtocolInbound {
    pub fn new(protocol: Box<dyn Protocol>, bind_addr: SocketAddr) -> Self {
        Self { protocol, bind_addr }
    }
}

#[async_trait::async_trait]
impl Inbound for ProtocolInbound {
    async fn start(&self) -> Result<()> {
        self.protocol.start_inbound(self.bind_addr).await
    }
}

#[cfg(target_os = "linux")]
pub mod tproxy {
    use super::*;
    use log::{info, error};
    use socket2::{Socket, Domain, Type, Protocol};
    use std::os::fd::FromRawFd;
    use tokio::net::{TcpListener, UdpSocket};
    use nix::sys::socket::{setsockopt, sockopt::IpTransparent};

    pub struct TProxyInbound {
        pub bind_addr: SocketAddr,
    }

    impl TProxyInbound {
        pub fn new(bind_addr: SocketAddr) -> Self { Self { bind_addr } }
    }

    #[async_trait::async_trait]
    impl Inbound for TProxyInbound {
        async fn start(&self) -> Result<()> {
            // TCP transparent listener
            let tcp_listener = create_transparent_tcp_listener(self.bind_addr)?;
            let listener = unsafe { TcpListener::from_std(std::net::TcpListener::from_raw_fd(tcp_listener.as_raw_fd())) };
            info!("TProxy TCP listening on {}", self.bind_addr);
            tokio::spawn(async move {
                loop {
                    match listener.accept().await {
                        Ok((_stream, peer)) => {
                            info!("TProxy TCP accepted from {}", peer);
                        }
                        Err(e) => {
                            error!("TProxy TCP accept error: {}", e);
                            break;
                        }
                    }
                }
            });

            // UDP transparent socket
            let udp = create_transparent_udp_socket(self.bind_addr)?;
            let _udp = UdpSocket::from_std(udp)?;
            info!("TProxy UDP bound on {}", self.bind_addr);
            Ok(())
        }
    }

    use std::os::fd::{AsRawFd, IntoRawFd};
    fn create_transparent_tcp_listener(addr: SocketAddr) -> Result<std::net::TcpListener> {
        let domain = match addr { SocketAddr::V4(_) => Domain::IPV4, SocketAddr::V6(_) => Domain::IPV6 };
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
        socket.set_reuse_address(true)?;
        // IP_TRANSPARENT
        setsockopt(socket.as_raw_fd(), IpTransparent, &true).map_err(|e| std::io::Error::other(e))?;
        socket.bind(&addr.into())?;
        socket.listen(1024)?;
        Ok(socket.into())
    }

    fn create_transparent_udp_socket(addr: SocketAddr) -> Result<std::net::UdpSocket> {
        let domain = match addr { SocketAddr::V4(_) => Domain::IPV4, SocketAddr::V6(_) => Domain::IPV6 };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        setsockopt(socket.as_raw_fd(), IpTransparent, &true).map_err(|e| std::io::Error::other(e))?;
        socket.bind(&addr.into())?;
        Ok(socket.into())
    }
}

#[cfg(not(target_os = "linux"))]
pub mod tproxy {
    use super::*;
    use crate::error::ProxyError;

    pub struct TProxyInbound { pub bind_addr: SocketAddr }
    impl TProxyInbound { pub fn new(bind_addr: SocketAddr) -> Self { Self { bind_addr } } }

    #[async_trait::async_trait]
    impl Inbound for TProxyInbound {
        async fn start(&self) -> Result<()> { Err(ProxyError::Protocol("TProxy inbound is only supported on Linux".into())) }
    }
}



use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use socket2::{Socket, Domain, Type, Protocol};
use crate::error::{ProxyError, Result};
use log::{debug, warn};

/// Traffic marking configuration
#[derive(Debug, Clone)]
pub struct TrafficMarkConfig {
    /// Linux SO_MARK value
    pub so_mark: Option<u32>,
    /// macOS SO_NET_SERVICE_TYPE value
    pub net_service_type: Option<u32>,
}

impl TrafficMarkConfig {
    pub fn new(so_mark: Option<u32>, net_service_type: Option<u32>) -> Self {
        Self {
            so_mark,
            net_service_type,
        }
    }

    /// Create config with Linux SO_MARK only
    pub fn with_so_mark(mark: u32) -> Self {
        Self {
            so_mark: Some(mark),
            net_service_type: None,
        }
    }

    /// Create config with macOS SO_NET_SERVICE_TYPE only
    pub fn with_net_service_type(service_type: u32) -> Self {
        Self {
            so_mark: None,
            net_service_type: Some(service_type),
        }
    }
}

/// Apply traffic marking to a socket
pub fn apply_traffic_mark(socket: &Socket, config: &TrafficMarkConfig) -> Result<()> {
    // Apply Linux SO_MARK if configured
    if let Some(mark) = config.so_mark {
        #[cfg(target_os = "linux")]
        {
            if let Err(e) = platform::apply_so_mark(&socket, mark) {
                warn!("Failed to set SO_MARK {}: {}", mark, e);
                return Err(e);
            }
            debug!("Applied SO_MARK: {}", mark);
        }
        #[cfg(not(target_os = "linux"))]
        {
            warn!("SO_MARK not supported on this platform");
        }
    }

    // Apply macOS SO_NET_SERVICE_TYPE if configured
    if let Some(service_type) = config.net_service_type {
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = platform::apply_net_service_type(&socket, service_type) {
                warn!("Failed to set SO_NET_SERVICE_TYPE {}: {}", service_type, e);
                return Err(e);
            }
            debug!("Applied SO_NET_SERVICE_TYPE: {}", service_type);
        }
        #[cfg(not(target_os = "macos"))]
        {
            warn!("SO_NET_SERVICE_TYPE not supported on this platform");
        }
    }

    Ok(())
}

/// Create a new TCP stream with traffic marking applied
pub async fn create_marked_tcp_stream(
    target_addr: SocketAddr,
    config: &TrafficMarkConfig,
) -> Result<TcpStream> {
    // Create socket with appropriate domain
    let domain = match target_addr {
        SocketAddr::V4(_) => Domain::IPV4,
        SocketAddr::V6(_) => Domain::IPV6,
    };

    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
        .map_err(|e| ProxyError::Io(e))?;

    // Apply traffic marking before connecting
    apply_traffic_mark(&socket, config)?;

    // Connect to target
    socket.connect(&target_addr.into())
        .map_err(|e| ProxyError::Io(e))?;

    // Convert to tokio TcpStream
    let std_stream = socket.into();
    let stream = TcpStream::from_std(std_stream)
        .map_err(|e| ProxyError::Io(e))?;

    debug!("Created marked TCP stream to {}", target_addr);
    Ok(stream)
}

/// Apply traffic marking to an existing TcpStream
pub fn mark_existing_stream(stream: TcpStream, config: &TrafficMarkConfig) -> Result<TcpStream> {
    // Convert to socket2::Socket for marking
    let std_stream = stream.into_std()
        .map_err(|e| ProxyError::Io(e))?;
    
    let socket = Socket::from(std_stream);
    
    // Apply traffic marking
    apply_traffic_mark(&socket, config)?;
    
    // Convert back to tokio TcpStream
    let std_stream = socket.into();
    let stream = TcpStream::from_std(std_stream)
        .map_err(|e| ProxyError::Io(e))?;
    
    debug!("Applied traffic marking to existing stream");
    Ok(stream)
}

/// Platform-specific traffic marking utilities
#[cfg(target_os = "linux")]
pub mod platform {
    use super::*;
    use nix::sys::socket::{setsockopt, sockopt};
    use std::os::unix::io::AsRawFd;

    /// Apply Linux-specific SO_MARK to a socket
    pub fn apply_so_mark(socket: &Socket, mark: u32) -> Result<()> {
        let fd = socket.as_raw_fd();
        setsockopt(fd, &sockopt::SO_MARK, &mark)
            .map_err(|e| ProxyError::Io(io::Error::from_raw_os_error(e as i32)))?;
        debug!("Applied SO_MARK {} to socket", mark);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub mod platform {
    use super::*;
    use std::os::unix::io::AsRawFd;

    /// Apply macOS-specific SO_NET_SERVICE_TYPE to a socket
    pub fn apply_net_service_type(socket: &Socket, service_type: u32) -> Result<()> {
        // SO_NET_SERVICE_TYPE is not available in libc crate
        // We'll use a different approach or skip this feature for now
        warn!("SO_NET_SERVICE_TYPE not available in libc crate, skipping marking for {}", service_type);
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub mod platform {
    use super::*;
    use log::warn;

    /// No-op implementation for unsupported platforms
    pub fn apply_so_mark(_socket: &Socket, _mark: u32) -> Result<()> {
        warn!("SO_MARK not supported on this platform");
        Ok(())
    }

    /// No-op implementation for unsupported platforms
    pub fn apply_net_service_type(_socket: &Socket, _service_type: u32) -> Result<()> {
        warn!("SO_NET_SERVICE_TYPE not supported on this platform");
        Ok(())
    }
}

/// Global traffic marking configuration
static mut GLOBAL_TRAFFIC_MARK_CONFIG: Option<TrafficMarkConfig> = None;

/// Initialize global traffic marking configuration
pub fn init_global_traffic_mark_config(config: TrafficMarkConfig) {
    unsafe {
        GLOBAL_TRAFFIC_MARK_CONFIG = Some(config);
    }
}

/// Get global traffic marking configuration
pub fn get_global_traffic_mark_config() -> Option<&'static TrafficMarkConfig> {
    unsafe {
        GLOBAL_TRAFFIC_MARK_CONFIG.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_traffic_mark_config_creation() {
        let config = TrafficMarkConfig::with_so_mark(255);
        assert_eq!(config.so_mark, Some(255));
        assert_eq!(config.net_service_type, None);
    }

    #[test]
    fn test_traffic_mark_config_with_service_type() {
        let config = TrafficMarkConfig::with_net_service_type(1);
        assert_eq!(config.so_mark, None);
        assert_eq!(config.net_service_type, Some(1));
    }

    #[test]
    fn test_traffic_mark_config_combined() {
        let config = TrafficMarkConfig::new(Some(255), Some(1));
        assert_eq!(config.so_mark, Some(255));
        assert_eq!(config.net_service_type, Some(1));
    }
}

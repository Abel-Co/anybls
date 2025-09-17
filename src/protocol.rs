use crate::error::{ProxyError, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone)]
pub enum Address {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
    Domain(String),
}

impl Address {
    pub fn from_bytes(buf: &mut Bytes) -> Result<(Address, u16)> {
        let addr_type = buf.get_u8();
        match addr_type {
            0x01 => {
                // IPv4
                let ip = Ipv4Addr::new(buf.get_u8(), buf.get_u8(), buf.get_u8(), buf.get_u8());
                let port = buf.get_u16();
                Ok((Address::V4(ip), port))
            }
            0x03 => {
                // Domain name
                let len = buf.get_u8() as usize;
                let mut domain = vec![0u8; len];
                buf.copy_to_slice(&mut domain);
                let domain = String::from_utf8(domain)
                    .map_err(|_| ProxyError::Protocol("Invalid domain name".to_string()))?;
                let port = buf.get_u16();
                Ok((Address::Domain(domain), port))
            }
            0x04 => {
                // IPv6
                let mut ip_bytes = [0u8; 16];
                buf.copy_to_slice(&mut ip_bytes);
                let ip = Ipv6Addr::from(ip_bytes);
                let port = buf.get_u16();
                Ok((Address::V6(ip), port))
            }
            _ => Err(ProxyError::InvalidAddressType(addr_type)),
        }
    }

    pub fn to_socket_addr(&self, port: u16) -> Result<SocketAddr> {
        match self {
            Address::V4(ip) => Ok(SocketAddr::new(IpAddr::V4(*ip), port)),
            Address::V6(ip) => Ok(SocketAddr::new(IpAddr::V6(*ip), port)),
            Address::Domain(_) => Err(ProxyError::Protocol("Domain resolution requires async context".to_string())),
        }
    }

    pub async fn to_socket_addr_async(&self, port: u16) -> Result<SocketAddr> {
        match self {
            Address::V4(ip) => Ok(SocketAddr::new(IpAddr::V4(*ip), port)),
            Address::V6(ip) => Ok(SocketAddr::new(IpAddr::V6(*ip), port)),
            Address::Domain(domain) => {
                use crate::dns::get_global_dns_resolver;
                get_global_dns_resolver().resolve_domain(domain, port).await
            }
        }
    }
}

#[derive(Debug)]
pub struct Socks5Request {
    pub command: u8,
    pub address: Address,
    pub port: u16,
}

impl Socks5Request {
    pub fn from_bytes(buf: &mut Bytes) -> Result<Self> {
        if buf.len() < 4 {
            return Err(ProxyError::Protocol("Incomplete SOCKS5 request".to_string()));
        }

        let version = buf.get_u8();
        if version != 0x05 {
            return Err(ProxyError::Protocol(format!("Unsupported SOCKS version: {}", version)));
        }

        let command = buf.get_u8();
        if command != 0x01 {
            return Err(ProxyError::UnsupportedCommand(command));
        }

        buf.get_u8(); // Reserved byte

        let (address, port) = Address::from_bytes(buf)?;

        Ok(Socks5Request {
            command,
            address,
            port,
        })
    }
}

pub struct Socks5Response {
    pub status: u8,
    pub address: Address,
    pub port: u16,
}

impl Socks5Response {
    pub fn new(status: u8, address: Address, port: u16) -> Self {
        Self { status, address, port }
    }

    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(256);

        // Version
        buf.put_u8(0x05);
        // Status
        buf.put_u8(self.status);
        // Reserved
        buf.put_u8(0x00);

        // Address
        match &self.address {
            Address::V4(ip) => {
                buf.put_u8(0x01); // IPv4
                buf.put_slice(&ip.octets());
            }
            Address::V6(ip) => {
                buf.put_u8(0x04); // IPv6
                buf.put_slice(&ip.octets());
            }
            Address::Domain(domain) => {
                buf.put_u8(0x03); // Domain
                buf.put_u8(domain.len() as u8);
                buf.put_slice(domain.as_bytes());
            }
        }

        // Port
        buf.put_u16(self.port);

        buf.freeze()
    }
}

pub async fn handle_socks5_handshake<T>(stream: &mut T) -> Result<()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).await?;

    if n < 3 {
        return Err(ProxyError::Protocol("Incomplete handshake".to_string()));
    }

    let version = buf[0];
    if version != 0x05 {
        return Err(ProxyError::Protocol(format!("Unsupported SOCKS version: {}", version)));
    }

    let nmethods = buf[1] as usize;
    if n < 2 + nmethods {
        return Err(ProxyError::Protocol("Incomplete handshake".to_string()));
    }

    // Check if no authentication is supported
    let no_auth_supported = buf[2..2 + nmethods].contains(&0x00);

    if !no_auth_supported {
        // Send "no acceptable methods" response
        let response = [0x05, 0xFF];
        stream.write_all(&response).await?;
        return Err(ProxyError::AuthFailed);
    }

    // Send "no authentication required" response
    let response = [0x05, 0x00];
    stream.write_all(&response).await?;

    Ok(())
}

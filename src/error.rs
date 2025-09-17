use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SOCKS5 protocol error: {0}")]
    Protocol(String),

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Unsupported command: {0}")]
    UnsupportedCommand(u8),

    #[error("Invalid address type: {0}")]
    InvalidAddressType(u8),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("DNS resolution failed: {0}")]
    DnsResolution(String),
}

pub type Result<T> = std::result::Result<T, ProxyError>;

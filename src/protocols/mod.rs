// 协议模块 - 统一的协议trait，支持inbound和outbound
use async_trait::async_trait;
use std::net::SocketAddr;
use crate::error::Result;
use tokio::net::TcpStream;

/// 统一的协议trait
/// 所有协议（direct、socks5、vless、blackhole等）都实现这个trait
/// 既可以作为inbound也可以作为outbound使用
#[async_trait]
pub trait Protocol: Send + Sync {
    /// 协议名称
    fn name(&self) -> &str;
    
    /// 作为outbound连接时使用
    async fn connect_outbound(&self, target: SocketAddr) -> Result<TcpStream>;
    
    /// 作为inbound启动时使用
    async fn start_inbound(&self, bind_addr: SocketAddr) -> Result<()>;
}

pub mod direct;
pub mod socks5;
pub mod vless;
pub mod blackhole;
pub mod tproxy;

pub use direct::DirectProtocol;
pub use socks5::Socks5Protocol;
pub use vless::VlessProtocol;
pub use blackhole::BlackholeProtocol;
pub use tproxy::TproxyProtocol;

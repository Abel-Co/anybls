pub mod config;
pub mod connection_pool;
pub mod dns;
pub mod error;
pub mod inbound;
pub mod outbound;
pub mod protocol;
pub mod protocols;
pub mod proxy;
pub mod ron_config;
pub mod routing;
pub mod traffic_mark;
pub mod zero_copy;

pub use error::{ProxyError, Result};
pub use inbound::{Inbound, ProtocolInbound};
pub use outbound::{OutboundConnector, OutboundManager};
pub use protocol::{Address, Socks5Request, Socks5Response};
pub use protocols::{
    BlackholeProtocol, DirectProtocol, Protocol, Socks5Protocol, TproxyProtocol, VlessProtocol,
};
pub use proxy::Socks5Proxy;
pub use routing::rule_sets::{DomainRuleSet, IpRuleSet, RuleSetManager};
pub use routing::{HighPerformanceRouter, RouteRule};
pub use zero_copy::{OptimizedCopier, ZeroCopyBuffer, ZeroCopyRelay};

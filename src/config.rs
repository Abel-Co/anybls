use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use std::path::Path;
use std::fs;
use crate::error::{ProxyError, Result};
use log::info;

/// Configuration for the SOCKS5 proxy server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Connection pool configuration
    pub connection_pool: ConnectionPoolConfig,
    /// DNS configuration
    pub dns: DnsConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Traffic marking configuration
    pub traffic_mark: TrafficMarkConfig,

    /// Outbound configurations
    pub outbounds: Vec<OutboundConfig>,

    /// Router configuration
    pub router: RouterConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: IpAddr,
    /// Port to listen on
    pub port: u16,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout_secs: u64,
    /// Keep-alive timeout
    pub keep_alive_timeout_secs: u64,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum connections per target
    pub max_connections_per_target: usize,
    /// Maximum total connections
    pub max_total_connections: usize,
    /// Connection timeout
    pub connection_timeout_secs: u64,
    /// Idle timeout
    pub idle_timeout_secs: u64,
    /// Cleanup interval
    pub cleanup_interval_secs: u64,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS servers to use
    pub servers: Vec<String>,
    /// DNS timeout
    pub timeout_secs: u64,
    /// Enable IPv6 resolution
    pub enable_ipv6: bool,
    /// Cache TTL
    pub cache_ttl_secs: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Enable structured logging
    pub structured: bool,
    /// Log file path (optional)
    pub file: Option<String>,
    /// Enable performance metrics
    pub enable_metrics: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Buffer size for zero-copy operations
    pub buffer_size: usize,
    /// Enable TCP_NODELAY
    pub tcp_nodelay: bool,
    /// Enable SO_REUSEADDR
    pub reuse_addr: bool,
    /// Enable SO_KEEPALIVE
    pub keep_alive: bool,
    /// Worker thread count (0 for auto)
    pub worker_threads: usize,
}

/// Traffic marking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficMarkConfig {
    /// Linux SO_MARK value (0 to disable)
    pub so_mark: u32,
    /// macOS SO_NET_SERVICE_TYPE value (0 to disable)
    pub net_service_type: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            connection_pool: ConnectionPoolConfig::default(),
            dns: DnsConfig::default(),
            logging: LoggingConfig::default(),
            performance: PerformanceConfig::default(),
            traffic_mark: TrafficMarkConfig::default(),
            outbounds: vec![OutboundConfig::direct("direct")],
            router: RouterConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 1080,
            max_connections: 1000,
            connection_timeout_secs: 30,
            keep_alive_timeout_secs: 300,
        }
    }
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_target: 10,
            max_total_connections: 500,
            connection_timeout_secs: 10,
            idle_timeout_secs: 300,
            cleanup_interval_secs: 60,
        }
    }
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            servers: vec![
                "8.8.8.8:53".to_string(),
                "8.8.4.4:53".to_string(),
                "1.1.1.1:53".to_string(),
            ],
            timeout_secs: 5,
            enable_ipv6: true,
            cache_ttl_secs: 300,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            structured: false,
            file: None,
            enable_metrics: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            buffer_size: 65536, // 64KB
            tcp_nodelay: true,
            reuse_addr: true,
            keep_alive: true,
            worker_threads: 0, // Auto-detect
        }
    }
}

impl Default for TrafficMarkConfig {
    fn default() -> Self {
        Self {
            so_mark: 0, // Disabled by default
            net_service_type: 0, // Disabled by default
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutboundType {
    Direct,
    Socks5 { address: String },
    Vless { address: String, uuid: String, tls: bool },
    Blackhole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundConfig {
    pub name: String,
    #[serde(flatten)]
    pub kind: OutboundType,
}

impl OutboundConfig {
    pub fn direct(name: &str) -> Self {
        Self { name: name.to_string(), kind: OutboundType::Direct }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainLists {
    pub domain: Vec<String>,
    pub domain_suffix: Vec<String>,
    pub domain_keyword: Vec<String>,
    pub domain_regex: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterRuleConfig {
    pub outbound: String,
    #[serde(default)]
    pub domains: DomainLists,
    #[serde(default)]
    pub ip_cidr: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub default_outbound: String,
    #[serde(default)]
    pub rules: Vec<RouterRuleConfig>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self { default_outbound: "direct".to_string(), rules: Vec::new() }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| ProxyError::Io(e))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| ProxyError::Protocol(format!("Invalid configuration: {}", e)))?;
        
        info!("Configuration loaded from file");
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ProxyError::Protocol(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(path, content)
            .map_err(|e| ProxyError::Io(e))?;
        
        info!("Configuration saved to file");
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.server.port == 0 {
            return Err(ProxyError::Protocol("Invalid port number".to_string()));
        }

        if self.connection_pool.max_connections_per_target == 0 {
            return Err(ProxyError::Protocol("max_connections_per_target must be > 0".to_string()));
        }

        if self.connection_pool.max_total_connections == 0 {
            return Err(ProxyError::Protocol("max_total_connections must be > 0".to_string()));
        }

        if self.performance.buffer_size == 0 {
            return Err(ProxyError::Protocol("buffer_size must be > 0".to_string()));
        }

        // Validate log level
        match self.logging.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(ProxyError::Protocol("Invalid log level".to_string())),
        }

        // Validate outbounds
        if self.outbounds.is_empty() {
            return Err(ProxyError::Protocol("At least one outbound must be configured".to_string()));
        }

        Ok(())
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.server.connection_timeout_secs)
    }

    /// Get keep-alive timeout as Duration
    pub fn keep_alive_timeout(&self) -> Duration {
        Duration::from_secs(self.server.keep_alive_timeout_secs)
    }

    /// Get DNS timeout as Duration
    pub fn dns_timeout(&self) -> Duration {
        Duration::from_secs(self.dns.timeout_secs)
    }

    /// Get connection pool timeout as Duration
    pub fn pool_connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_pool.connection_timeout_secs)
    }

    /// Get connection pool idle timeout as Duration
    pub fn pool_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_pool.idle_timeout_secs)
    }

    /// Get cleanup interval as Duration
    pub fn cleanup_interval(&self) -> Duration {
        Duration::from_secs(self.connection_pool.cleanup_interval_secs)
    }
}

/// Global configuration
static mut GLOBAL_CONFIG: Option<Config> = None;

/// Initialize global configuration
pub fn init_global_config(config: Config) -> Result<()> {
    config.validate()?;
    unsafe {
        GLOBAL_CONFIG = Some(config);
    }
    info!("Global configuration initialized");
    Ok(())
}

/// Get global configuration
pub fn get_global_config() -> &'static Config {
    unsafe {
        GLOBAL_CONFIG.as_ref()
            .expect("Global configuration not initialized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 1080);
        assert_eq!(config.server.host, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml).unwrap();
        assert_eq!(config.server.port, parsed.server.port);
    }
}

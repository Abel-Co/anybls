use clap::Parser;
use log::{info, error};
use std::net::{IpAddr, SocketAddr};
use socks5_proxy::proxy::Socks5Proxy;
use socks5_proxy::error::Result;
use socks5_proxy::dns::init_global_dns_resolver;
use socks5_proxy::config::{Config, init_global_config};
use socks5_proxy::connection_pool::{init_global_connection_pool, start_connection_pool_cleanup};
use socks5_proxy::traffic_mark::{init_global_traffic_mark_config, TrafficMarkConfig};

#[derive(Parser)]
#[command(name = "socks5-proxy")]
#[command(about = "A high-performance SOCKS5 proxy server with zero-copy optimization")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "1080")]
    port: u16,

    /// IP address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let mut config = if let Some(config_path) = &args.config {
        Config::from_file(config_path)?
    } else {
        Config::default()
    };

    // Override config with command line arguments
    if args.debug {
        config.logging.level = "debug".to_string();
    }
    config.server.host = args.host;
    config.server.port = args.port;

    // Initialize global configuration
    init_global_config(config.clone())?;

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.logging.level))
        .init();

    // Initialize DNS resolver
    init_global_dns_resolver()?;
    info!("DNS resolver initialized");

    // Initialize connection pool
    init_global_connection_pool(
        config.connection_pool.max_connections_per_target,
        config.connection_pool.max_total_connections,
        config.pool_connection_timeout(),
        config.pool_idle_timeout(),
    )?;
    info!("Connection pool initialized");

    // Start connection pool cleanup task
    start_connection_pool_cleanup(config.cleanup_interval()).await;

    // Initialize traffic marking
    let traffic_mark_config = TrafficMarkConfig::new(
        if config.traffic_mark.so_mark > 0 { Some(config.traffic_mark.so_mark) } else { None },
        if config.traffic_mark.net_service_type > 0 { Some(config.traffic_mark.net_service_type) } else { None },
    );
    init_global_traffic_mark_config(traffic_mark_config);
    info!("Traffic marking initialized");

    info!("Starting SOCKS5 proxy server...");
    info!("Configuration:");
    info!("  Host: {}", config.server.host);
    info!("  Port: {}", config.server.port);
    info!("  Max connections: {}", config.server.max_connections);
    info!("  SO_MARK: {}", config.traffic_mark.so_mark);
    info!("  SO_NET_SERVICE_TYPE: {}", config.traffic_mark.net_service_type);
    info!("  Debug: {}", args.debug);

    let bind_addr = SocketAddr::new(config.server.host, config.server.port);
    let proxy = Socks5Proxy::new(bind_addr);

    // Start the proxy server
    if let Err(e) = proxy.start().await {
        error!("Proxy server error: {}", e);
        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_proxy_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1080);
        let proxy = Socks5Proxy::new(addr);
        // Test passes if proxy is created successfully
        assert!(true);
    }
}

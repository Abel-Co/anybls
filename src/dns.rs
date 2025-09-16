use std::net::{IpAddr, SocketAddr};
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    TokioAsyncResolver,
};
use crate::error::{ProxyError, Result};
use log::{debug, warn};

/// DNS resolver for SOCKS5 proxy
pub struct DnsResolver {
    resolver: TokioAsyncResolver,
}

impl DnsResolver {
    /// Create a new DNS resolver with default configuration
    pub fn new() -> Result<Self> {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );

        Ok(Self { resolver })
    }

    /// Create a new DNS resolver with custom configuration
    pub fn with_config(config: ResolverConfig, opts: ResolverOpts) -> Result<Self> {
        let resolver = TokioAsyncResolver::tokio(config, opts);

        Ok(Self { resolver })
    }

    /// Resolve a domain name to an IP address
    pub async fn resolve_domain(&self, domain: &str, port: u16) -> Result<SocketAddr> {
        debug!("Resolving domain: {}:{}", domain, port);

        // Try IPv4 first
        match self.resolver.lookup_ip(domain).await {
            Ok(lookup) => {
                for ip in lookup.iter() {
                    debug!("Resolved {} to IP: {}", domain, ip);
                    return Ok(SocketAddr::new(ip, port));
                }
                Err(ProxyError::DnsResolution(format!("No IP addresses found for {}", domain)))
            }
            Err(e) => {
                warn!("DNS resolution failed for {}: {}", domain, e);
                Err(ProxyError::DnsResolution(e.to_string()))
            }
        }
    }

    /// Resolve a domain name to IPv4 address only
    pub async fn resolve_domain_v4(&self, domain: &str, port: u16) -> Result<SocketAddr> {
        debug!("Resolving domain to IPv4: {}:{}", domain, port);

        match self.resolver.ipv4_lookup(domain).await {
            Ok(lookup) => {
                for ipv4 in lookup.iter() {
                    debug!("Resolved {} to IPv4: {}", domain, ipv4);
                    return Ok(SocketAddr::new(IpAddr::V4(**ipv4), port));
                }
                Err(ProxyError::DnsResolution(format!("No IPv4 addresses found for {}", domain)))
            }
            Err(e) => {
                warn!("DNS resolution failed for {}: {}", domain, e);
                Err(ProxyError::DnsResolution(e.to_string()))
            }
        }
    }

    /// Resolve a domain name to IPv6 address only
    pub async fn resolve_domain_v6(&self, domain: &str, port: u16) -> Result<SocketAddr> {
        debug!("Resolving domain to IPv6: {}:{}", domain, port);

        match self.resolver.ipv6_lookup(domain).await {
            Ok(lookup) => {
                for ipv6 in lookup.iter() {
                    debug!("Resolved {} to IPv6: {}", domain, ipv6);
                    return Ok(SocketAddr::new(IpAddr::V6(**ipv6), port));
                }
                Err(ProxyError::DnsResolution(format!("No IPv6 addresses found for {}", domain)))
            }
            Err(e) => {
                warn!("DNS resolution failed for {}: {}", domain, e);
                Err(ProxyError::DnsResolution(e.to_string()))
            }
        }
    }
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new().expect("Failed to create default DNS resolver")
    }
}

/// Global DNS resolver instance
static mut GLOBAL_DNS_RESOLVER: Option<DnsResolver> = None;

/// Initialize the global DNS resolver
pub fn init_global_dns_resolver() -> Result<()> {
    unsafe {
        GLOBAL_DNS_RESOLVER = Some(DnsResolver::new()?);
    }
    Ok(())
}

/// Get the global DNS resolver
pub fn get_global_dns_resolver() -> &'static DnsResolver {
    unsafe {
        GLOBAL_DNS_RESOLVER.as_ref()
            .expect("Global DNS resolver not initialized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_dns_resolution() {
        let resolver = DnsResolver::new().unwrap();
        
        // Test with a well-known domain
        let result = resolver.resolve_domain("google.com", 80).await;
        assert!(result.is_ok());
        
        let socket_addr = result.unwrap();
        assert_eq!(socket_addr.port(), 80);
    }

    #[tokio::test]
    async fn test_dns_resolution_v4() {
        let resolver = DnsResolver::new().unwrap();
        
        let result = resolver.resolve_domain_v4("google.com", 443).await;
        assert!(result.is_ok());
        
        let socket_addr = result.unwrap();
        assert_eq!(socket_addr.port(), 443);
        assert!(matches!(socket_addr.ip(), IpAddr::V4(_)));
    }
}

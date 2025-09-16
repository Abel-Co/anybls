use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use crate::error::{ProxyError, Result};
use log::{debug, info};

/// Connection pool for managing TCP connections
pub struct ConnectionPool {
    /// Maximum number of connections per target
    max_connections_per_target: usize,
    /// Connection timeout
    connection_timeout: Duration,
    /// Idle timeout for connections
    idle_timeout: Duration,
    /// Semaphore to limit total connections
    semaphore: Arc<Semaphore>,
    /// Pool of connections by target address
    pools: Arc<RwLock<HashMap<SocketAddr, Vec<PooledConnection>>>>,
}

/// A pooled TCP connection with metadata
pub struct PooledConnection {
    stream: TcpStream,
    created_at: Instant,
    last_used: Instant,
    target_addr: SocketAddr,
}

impl PooledConnection {
    pub fn new(stream: TcpStream, target_addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            stream,
            created_at: now,
            last_used: now,
            target_addr,
        }
    }

    pub fn is_expired(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }

    pub fn update_last_used(&mut self) {
        self.last_used = Instant::now();
    }

    pub fn into_stream(self) -> TcpStream {
        self.stream
    }

    pub fn target_addr(&self) -> SocketAddr {
        self.target_addr
    }
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(
        max_connections_per_target: usize,
        max_total_connections: usize,
        connection_timeout: Duration,
        idle_timeout: Duration,
    ) -> Self {
        Self {
            max_connections_per_target,
            connection_timeout,
            idle_timeout,
            semaphore: Arc::new(Semaphore::new(max_total_connections)),
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a connection from the pool or create a new one
    pub async fn get_connection(&self, target_addr: SocketAddr) -> Result<PooledConnection> {
        // First, try to get an existing connection from the pool
        if let Some(connection) = self.get_from_pool(target_addr).await? {
            debug!("Reusing pooled connection to {}", target_addr);
            return Ok(connection);
        }

        // If no pooled connection available, create a new one
        debug!("Creating new connection to {}", target_addr);
        let _permit = self.semaphore.acquire().await
            .map_err(|_| ProxyError::ConnectionFailed("Connection pool exhausted".to_string()))?;

        let stream = timeout(
            self.connection_timeout,
            TcpStream::connect(target_addr)
        ).await
        .map_err(|_| ProxyError::ConnectionFailed("Connection timeout".to_string()))?
        .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;

        Ok(PooledConnection::new(stream, target_addr))
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, mut connection: PooledConnection) {
        let target_addr = connection.target_addr();
        
        // Check if the connection is still valid
        if connection.is_expired(self.idle_timeout) {
            debug!("Connection to {} expired, dropping", target_addr);
            return;
        }

        // Update last used time
        connection.update_last_used();

        // Add to pool if there's space
        let mut pools = self.pools.write().await;
        let pool = pools.entry(target_addr).or_insert_with(Vec::new);
        
        if pool.len() < self.max_connections_per_target {
            debug!("Returning connection to pool for {}", target_addr);
            pool.push(connection);
        } else {
            debug!("Pool for {} is full, dropping connection", target_addr);
        }
    }

    /// Get a connection from the pool for a specific target
    async fn get_from_pool(&self, target_addr: SocketAddr) -> Result<Option<PooledConnection>> {
        let mut pools = self.pools.write().await;
        
        if let Some(pool) = pools.get_mut(&target_addr) {
            // Remove expired connections
            pool.retain(|conn| !conn.is_expired(self.idle_timeout));
            
            // Return the first available connection
            if let Some(connection) = pool.pop() {
                debug!("Found pooled connection to {}", target_addr);
                return Ok(Some(connection));
            }
        }
        
        Ok(None)
    }

    /// Clean up expired connections
    pub async fn cleanup_expired(&self) {
        let mut pools = self.pools.write().await;
        let mut total_cleaned = 0;
        
        for (_target_addr, pool) in pools.iter_mut() {
            let before = pool.len();
            pool.retain(|conn| !conn.is_expired(self.idle_timeout));
            let after = pool.len();
            total_cleaned += before - after;
        }
        
        if total_cleaned > 0 {
            info!("Cleaned up {} expired connections", total_cleaned);
        }
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let pools = self.pools.read().await;
        let mut total_connections = 0;
        let mut targets = 0;
        
        for pool in pools.values() {
            total_connections += pool.len();
            if !pool.is_empty() {
                targets += 1;
            }
        }
        
        PoolStats {
            total_connections,
            targets,
            available_permits: self.semaphore.available_permits(),
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub targets: usize,
    pub available_permits: usize,
}

/// Global connection pool
static mut GLOBAL_CONNECTION_POOL: Option<ConnectionPool> = None;

/// Initialize the global connection pool
pub fn init_global_connection_pool(
    max_connections_per_target: usize,
    max_total_connections: usize,
    connection_timeout: Duration,
    idle_timeout: Duration,
) -> Result<()> {
    unsafe {
        GLOBAL_CONNECTION_POOL = Some(ConnectionPool::new(
            max_connections_per_target,
            max_total_connections,
            connection_timeout,
            idle_timeout,
        ));
    }
    Ok(())
}

/// Get the global connection pool
pub fn get_global_connection_pool() -> &'static ConnectionPool {
    unsafe {
        GLOBAL_CONNECTION_POOL.as_ref()
            .expect("Global connection pool not initialized")
    }
}

/// Start the connection pool cleanup task
pub async fn start_connection_pool_cleanup(interval: Duration) {
    let pool = get_global_connection_pool();
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(interval);
        loop {
            interval.tick().await;
            pool.cleanup_expired().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let pool = ConnectionPool::new(10, 100, Duration::from_secs(5), Duration::from_secs(30));
        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.targets, 0);
    }

    #[tokio::test]
    async fn test_connection_pool_stats() {
        let pool = ConnectionPool::new(5, 50, Duration::from_secs(5), Duration::from_secs(30));
        let stats = pool.stats().await;
        assert_eq!(stats.available_permits, 50);
    }
}

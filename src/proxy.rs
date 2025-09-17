use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use crate::error::{ProxyError, Result};
use crate::protocol::{handle_socks5_handshake, Socks5Request, Socks5Response};
use crate::zero_copy::ZeroCopyRelay;
use crate::traffic_mark::{create_marked_tcp_stream, get_global_traffic_mark_config};
use crate::router::get_global_router;
use crate::outbound::get_global_outbound_manager;
use log::{info, warn, error, debug};

pub struct Socks5Proxy {
    bind_addr: SocketAddr,
}

impl Socks5Proxy {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self { bind_addr }
    }

    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("SOCKS5 proxy listening on {}", self.bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    info!("New connection from {}", client_addr);
                    
                    // Spawn a new task for each connection
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, client_addr).await {
                            error!("Error handling connection from {}: {}", client_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(mut client_stream: TcpStream, client_addr: SocketAddr) -> Result<()> {
        debug!("Handling connection from {}", client_addr);

        // Perform SOCKS5 handshake
        handle_socks5_handshake(&mut client_stream).await?;
        debug!("SOCKS5 handshake completed for {}", client_addr);

        // Read the SOCKS5 request
        let mut request_buf = [0u8; 256];
        let n = client_stream.read(&mut request_buf).await?;
        let mut request_bytes = bytes::Bytes::from(request_buf[..n].to_vec());
        
        let request = Socks5Request::from_bytes(&mut request_bytes)?;
        debug!("SOCKS5 request: {:?}", request);

        // Connect to the target
        let target_addr = request.address.to_socket_addr_async(request.port).await?;

        debug!("Connecting to target: {}", target_addr);
        // Decide outbound based on domain/ip
        let outbound_name = match &request.address {
            crate::protocol::Address::Domain(d) => get_global_router().select_outbound_for_domain(d),
            crate::protocol::Address::V4(ip) => get_global_router().select_outbound_for_ip(std::net::IpAddr::V4(*ip)),
            crate::protocol::Address::V6(ip) => get_global_router().select_outbound_for_ip(std::net::IpAddr::V6(*ip)),
        };
        let ob_manager = get_global_outbound_manager();
        let connector = ob_manager.get(&outbound_name).ok_or_else(|| crate::error::ProxyError::Protocol(format!("Outbound not found: {}", outbound_name)))?;

        let target_stream = match connector.connect(target_addr).await {
            Ok(stream) => stream,
            Err(e) => {
                warn!("Failed to connect to {}: {}", target_addr, e);
                let response = Socks5Response::new(0x04, request.address.clone(), request.port);
                let response_bytes = response.to_bytes();
                let _ = client_stream.write_all(&response_bytes).await;
                return Err(ProxyError::ConnectionFailed(e.to_string()));
            }
        };

        info!("Connected to target {} for client {}", target_addr, client_addr);

        // Send success response
        let response = Socks5Response::new(0x00, request.address, request.port);
        let response_bytes = response.to_bytes();
        client_stream.write_all(&response_bytes).await?;

        // Start zero-copy relay
        let relay = ZeroCopyRelay::new(client_stream, target_stream);
        relay.start().await?;

        info!("Connection from {} completed", client_addr);
        Ok(())
    }
}

/// Create a TCP connection with traffic marking applied
async fn create_marked_connection(target_addr: SocketAddr) -> Result<TcpStream> {
    // Check if traffic marking is configured
    if let Some(traffic_config) = get_global_traffic_mark_config() {
        // Check if any marking is enabled
        if traffic_config.so_mark.is_some() || traffic_config.net_service_type.is_some() {
            debug!("Creating marked connection to {}", target_addr);
            return create_marked_tcp_stream(target_addr, traffic_config).await;
        }
    }
    
    // Fall back to regular connection if no marking is configured
    debug!("Creating regular connection to {}", target_addr);
    TcpStream::connect(target_addr).await
        .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))
}

/// Connection handler for individual client connections
pub struct ConnectionHandler {
    client_stream: TcpStream,
    client_addr: SocketAddr,
}

impl ConnectionHandler {
    pub fn new(client_stream: TcpStream, client_addr: SocketAddr) -> Self {
        Self {
            client_stream,
            client_addr,
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        debug!("Starting connection handler for {}", self.client_addr);

        // Perform SOCKS5 handshake
        handle_socks5_handshake(&mut self.client_stream).await?;
        debug!("SOCKS5 handshake completed for {}", self.client_addr);

        // Read and parse SOCKS5 request
        let request = self.read_socks5_request().await?;
        debug!("SOCKS5 request: {:?}", request);

        // Connect to target
        let target_stream = self.connect_to_target(&request).await?;

        // Send success response
        self.send_success_response(&request).await?;

        // Start relay
        self.start_relay(target_stream).await?;

        Ok(())
    }

    async fn read_socks5_request(&mut self) -> Result<Socks5Request> {
        let mut request_buf = [0u8; 256];
        let n = self.client_stream.read(&mut request_buf).await?;
        let mut request_bytes = bytes::Bytes::from(request_buf[..n].to_vec());
        
        Socks5Request::from_bytes(&mut request_bytes)
    }

    async fn connect_to_target(&self, request: &Socks5Request) -> Result<TcpStream> {
        let target_addr = request.address.to_socket_addr_async(request.port).await?;

        debug!("Connecting to target: {}", target_addr);
        create_marked_connection(target_addr).await
    }

    async fn send_success_response(&mut self, request: &Socks5Request) -> Result<()> {
        let response = Socks5Response::new(0x00, request.address.clone(), request.port);
        let response_bytes = response.to_bytes();
        self.client_stream.write_all(&response_bytes).await?;
        Ok(())
    }

    async fn start_relay(self, target_stream: TcpStream) -> Result<()> {
        let relay = ZeroCopyRelay::new(self.client_stream, target_stream);
        relay.start().await
    }
}

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use remoc::prelude::*;

use crate::error::Result;
use crate::gemini::GeminiClient;
use crate::rpc::service::{DesktopServiceImpl, DesktopServiceClient, DesktopServiceServerSharedMut};

/// RPC server configuration
pub struct RpcServerConfig {
    pub port: u16,
    pub gemini_client: GeminiClient,
    pub allowed_executables: Vec<String>,
}

/// Start the RPC server
pub async fn start_server(config: RpcServerConfig) -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::DesktopCliError::IoError(e))?;
    
    tracing::info!("Desktop daemon listening on {}", addr);
    
    // Create the service implementation
    let service_impl = Arc::new(RwLock::new(DesktopServiceImpl::new(
        config.gemini_client,
        config.allowed_executables,
    )));
    
    loop {
        let (socket, remote_addr) = listener.accept().await
            .map_err(|e| crate::error::DesktopCliError::IoError(e))?;
        
        tracing::info!("New connection from {}", remote_addr);
        
        let service_impl = service_impl.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, service_impl).await {
                tracing::error!("Connection error from {}: {}", remote_addr, e);
            }
        });
    }
}

async fn handle_connection(
    socket: tokio::net::TcpStream,
    service_impl: Arc<RwLock<DesktopServiceImpl>>,
) -> anyhow::Result<()> {
    let (socket_rx, socket_tx) = socket.into_split();
    
    // Establish remoc connection
    let (conn, mut tx, _rx): (_, rch::base::Sender<DesktopServiceClient>, rch::base::Receiver<()>) =
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx).await?;
    
    // Spawn the connection handler
    tokio::spawn(conn);
    
    // Create server and client for the service
    let (server, client) = DesktopServiceServerSharedMut::new(service_impl, 16);
    
    // Send the client to the remote endpoint
    tx.send(client).await?;
    
    // Serve requests
    server.serve(true).await?;
    
    Ok(())
}

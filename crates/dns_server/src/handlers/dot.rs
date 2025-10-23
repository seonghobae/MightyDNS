use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, rustls};
use hickory_proto::op::Message;
use tracing::{debug, error, info, warn};

use crate::{resolver, AppState};

/// Serve DNS over TLS (DoT) on port 853
pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    // TODO: Load TLS certificates (for now, we'll skip TLS and just log a warning)
    // In production, load from config.dns.tls_cert_path and config.dns.tls_key_path
    warn!("DoT server starting WITHOUT TLS (certificates not configured)");
    warn!("For production, configure TLS certificates in environment variables");

    let port = state.config.dns.dot_port;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    info!("DoT server listening on port {} (TLS disabled for development)", port);

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept DoT connection: {}", e);
                continue;
            }
        };

        let state = state.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_dot_connection(state, stream, addr).await {
                error!("DoT connection error from {}: {}", addr, e);
            }
        });
    }
}

/// Handle a DoT connection (without TLS for now)
async fn handle_dot_connection(
    state: Arc<AppState>,
    mut stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
) -> anyhow::Result<()> {
    debug!("DoT connection from {}", addr);

    // TODO: Extract tenant ID from SNI (TLS Server Name Indication)
    // For now, we'll use a placeholder tenant
    let tenant_id = "default".to_string();

    loop {
        // Read 2-byte length prefix
        let mut length_bytes = [0u8; 2];
        match stream.read_exact(&mut length_bytes).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                debug!("DoT connection closed by client: {}", addr);
                break;
            }
            Err(e) => {
                error!("Failed to read length prefix: {}", e);
                break;
            }
        }

        let length = u16::from_be_bytes(length_bytes) as usize;

        if length == 0 || length > 65535 {
            warn!("Invalid DNS message length: {}", length);
            break;
        }

        // Read DNS message
        let mut buffer = vec![0u8; length];
        if let Err(e) = stream.read_exact(&mut buffer).await {
            error!("Failed to read DNS message: {}", e);
            break;
        }

        // Parse DNS query
        let dns_query = match Message::from_vec(&buffer) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to parse DNS message: {}", e);
                continue;
            }
        };

        // Resolve DNS query
        let response = match resolver::resolve_dns_query(&state, &tenant_id, dns_query, "dot").await {
            Ok(resp) => resp,
            Err(e) => {
                error!("DNS query resolution failed: {}", e);
                continue;
            }
        };

        // Serialize response
        let response_bytes = match response.to_vec() {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to serialize DNS response: {}", e);
                continue;
            }
        };

        // Write length prefix + response
        let response_length = response_bytes.len() as u16;
        if let Err(e) = stream.write_all(&response_length.to_be_bytes()).await {
            error!("Failed to write response length: {}", e);
            break;
        }

        if let Err(e) = stream.write_all(&response_bytes).await {
            error!("Failed to write response: {}", e);
            break;
        }

        if let Err(e) = stream.flush().await {
            error!("Failed to flush stream: {}", e);
            break;
        }
    }

    Ok(())
}

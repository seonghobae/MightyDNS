use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, rustls};
use rustls_pemfile::{certs, pkcs8_private_keys};
use hickory_proto::op::Message;
use tracing::{debug, error, info, warn};

use crate::{resolver, AppState};

/// Serve DNS over TLS (DoT) on port 853
pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let port = state.config.dns.dot_port;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    // Try to load TLS configuration
    let tls_acceptor = match (&state.config.dns.tls_cert_path, &state.config.dns.tls_key_path) {
        (Some(cert_path), Some(key_path)) => {
            match load_tls_config(cert_path, key_path) {
                Ok(acceptor) => {
                    info!("DoT server listening on port {} with TLS enabled", port);
                    Some(acceptor)
                }
                Err(e) => {
                    warn!("Failed to load TLS certificates: {}. DoT server will run WITHOUT TLS.", e);
                    warn!("For production, configure valid TLS certificates");
                    None
                }
            }
        }
        _ => {
            warn!("DoT server starting WITHOUT TLS (certificates not configured)");
            warn!("For production, set MIGHTYDNS__DNS__TLS_CERT_PATH and MIGHTYDNS__DNS__TLS_KEY_PATH");
            info!("DoT server listening on port {} (TLS disabled)", port);
            None
        }
    };

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept DoT connection: {}", e);
                continue;
            }
        };

        let state = state.clone();
        let tls_acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            if let Some(acceptor) = tls_acceptor {
                // TLS connection
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        if let Err(e) = handle_dot_connection_generic(state, tls_stream, addr).await {
                            error!("DoT TLS connection error from {}: {}", addr, e);
                        }
                    }
                    Err(e) => {
                        error!("TLS handshake failed from {}: {}", addr, e);
                    }
                }
            } else {
                // Non-TLS connection (fallback)
                if let Err(e) = handle_dot_connection_generic(state, stream, addr).await {
                    error!("DoT connection error from {}: {}", addr, e);
                }
            }
        });
    }
}

/// Load TLS configuration from certificate and key files
fn load_tls_config(cert_path: &str, key_path: &str) -> anyhow::Result<TlsAcceptor> {
    // Load certificates
    let cert_file = File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()?;

    if certs.is_empty() {
        anyhow::bail!("No certificates found in {}", cert_path);
    }

    // Load private key
    let key_file = File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let mut keys = pkcs8_private_keys(&mut key_reader)
        .collect::<Result<Vec<_>, _>>()?;

    if keys.is_empty() {
        anyhow::bail!("No private key found in {}", key_path);
    }

    let key = keys.remove(0);

    // Build TLS configuration
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key.into())?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Handle a DoT connection (generic over TLS and non-TLS streams)
async fn handle_dot_connection_generic<S>(
    state: Arc<AppState>,
    mut stream: S,
    addr: std::net::SocketAddr,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    debug!("DoT connection from {}", addr);

    // TODO: Extract tenant ID from SNI (TLS Server Name Indication)
    // For now, use a placeholder tenant
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

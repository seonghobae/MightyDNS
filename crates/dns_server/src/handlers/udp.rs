use hickory_proto::op::Message;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, error, info};

use crate::{resolver, AppState};

/// Serve DNS over UDP on port 53
pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let port = state.config.dns.udp_port;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;

    info!("UDP DNS server listening on port {}", port);

    let mut buffer = vec![0u8; 512]; // DNS over UDP is limited to 512 bytes

    loop {
        let (len, src_addr) = match socket.recv_from(&mut buffer).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to receive UDP packet: {}", e);
                continue;
            }
        };

        let query_bytes = buffer[..len].to_vec();
        let socket_clone = socket.clone();
        let state_clone = state.clone();

        // Handle each query in a separate task (non-blocking)
        tokio::spawn(async move {
            if let Err(e) = handle_udp_query(state_clone, socket_clone, query_bytes, src_addr).await {
                error!("Failed to handle UDP query from {}: {}", src_addr, e);
            }
        });
    }
}

/// Handle a single UDP DNS query
async fn handle_udp_query(
    state: Arc<AppState>,
    socket: UdpSocket,
    query_bytes: Vec<u8>,
    src_addr: SocketAddr,
) -> anyhow::Result<()> {
    debug!("UDP query from {}, {} bytes", src_addr, query_bytes.len());

    // Parse DNS query
    let dns_query = Message::from_vec(&query_bytes)?;

    // Look up tenant by source IP
    let tenant_info = state.cache.get_tenant_by_ip(&src_addr.ip().to_string()).await?;

    let tenant_id = match tenant_info {
        Some((tenant, _)) => tenant.tenant_identifier,
        None => {
            // No tenant binding for this IP - use default or reject
            debug!("No tenant binding for IP {}, using default", src_addr.ip());
            "default".to_string()
        }
    };

    // Resolve DNS query
    let response = resolver::resolve_dns_query(&state, &tenant_id, dns_query, "udp").await?;

    // Serialize response
    let response_bytes = response.to_vec()?;

    // Send response back to client
    socket.send_to(&response_bytes, src_addr).await?;

    Ok(())
}

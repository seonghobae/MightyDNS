use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::AppState;
use common::metrics::{DNS_BLOCKED_TOTAL, DNS_QUERIES_TOTAL, DNS_QUERY_DURATION, DNS_WHITELISTED_TOTAL};
use common::models::DnsResultType;
use common::nats_client::messages::DnsQueryLogMessage;

/// Resolve a DNS query with tenant context
pub async fn resolve_dns_query(
    state: &Arc<AppState>,
    tenant_id: &str,
    query: Message,
    protocol: &str,
) -> anyhow::Result<Message> {
    let start_time = Instant::now();

    // Extract query domain
    let domain = if let Some(q) = query.queries().first() {
        q.name().to_string().trim_end_matches('.').to_string()
    } else {
        error!("DNS query has no questions");
        return build_error_response(&query, ResponseCode::FormErr);
    };

    debug!("Resolving domain: {} for tenant: {} via {}", domain, tenant_id, protocol);

    // Get tenant account
    let tenant = match state.cache.get_tenant_by_identifier(tenant_id).await? {
        Some(t) => t,
        None => {
            warn!("Tenant not found: {}", tenant_id);
            DNS_QUERIES_TOTAL.with_label_values(&[protocol, "error"]).inc();
            return build_error_response(&query, ResponseCode::Refused);
        }
    };

    // Get tenant's default config (simplified - in production, support multiple configs)
    let configs = state.database.get_tenant_configs(&tenant.tenant_id).await?;
    let config = match configs.first() {
        Some(c) => c,
        None => {
            warn!("No config found for tenant: {}", tenant_id);
            DNS_QUERIES_TOTAL.with_label_values(&[protocol, "error"]).inc();
            return build_error_response(&query, ResponseCode::Refused);
        }
    };

    // Check whitelist first (highest priority)
    if state.cache.is_domain_whitelisted(&tenant.tenant_id, &domain).await? {
        info!("Domain {} is whitelisted for tenant {}", domain, tenant_id);
        DNS_WHITELISTED_TOTAL.with_label_values(&[&tenant.tenant_id.to_string()]).inc();
        DNS_QUERIES_TOTAL.with_label_values(&[protocol, "whitelisted"]).inc();

        // Forward to upstream DNS
        let response = forward_to_upstream(state, &query).await?;

        // Log query asynchronously
        let _ = log_query_async(
            state,
            &tenant.tenant_id,
            Some(&config.config_id),
            &domain,
            DnsResultType::Whitelisted,
            protocol,
        )
        .await;

        let duration = start_time.elapsed();
        DNS_QUERY_DURATION.with_label_values(&[protocol]).observe(duration.as_secs_f64());

        return Ok(response);
    }

    // Check blocklist
    let filter_result = state.cache.check_domain_filter(&tenant.tenant_id, &config.config_id, &domain).await?;

    match filter_result {
        DnsResultType::Blocked => {
            info!("Domain {} is blocked for tenant {}", domain, tenant_id);
            DNS_BLOCKED_TOTAL.with_label_values(&["unknown"]).inc();  // TODO: Add category
            DNS_QUERIES_TOTAL.with_label_values(&[protocol, "blocked"]).inc();

            // Log query asynchronously
            let _ = log_query_async(
                state,
                &tenant.tenant_id,
                Some(&config.config_id),
                &domain,
                DnsResultType::Blocked,
                protocol,
            )
            .await;

            let duration = start_time.elapsed();
            DNS_QUERY_DURATION.with_label_values(&[protocol]).observe(duration.as_secs_f64());

            // Return NXDOMAIN or custom blocked IP
            build_blocked_response(&query, &config.blocked_response_ip)
        }
        DnsResultType::Allowed => {
            debug!("Domain {} is allowed for tenant {}", domain, tenant_id);
            DNS_QUERIES_TOTAL.with_label_values(&[protocol, "allowed"]).inc();

            // Forward to upstream DNS
            let response = forward_to_upstream(state, &query).await?;

            // Log query asynchronously (if logging enabled)
            if config.is_logging_enabled {
                let _ = log_query_async(
                    state,
                    &tenant.tenant_id,
                    Some(&config.config_id),
                    &domain,
                    DnsResultType::Allowed,
                    protocol,
                )
                .await;
            }

            let duration = start_time.elapsed();
            DNS_QUERY_DURATION.with_label_values(&[protocol]).observe(duration.as_secs_f64());

            Ok(response)
        }
        _ => {
            // Shouldn't reach here, but handle gracefully
            warn!("Unexpected filter result: {:?}", filter_result);
            forward_to_upstream(state, &query).await
        }
    }
}

/// Forward DNS query to upstream resolver (Cloudflare, Google, etc.)
async fn forward_to_upstream(state: &Arc<AppState>, query: &Message) -> anyhow::Result<Message> {
    // For now, return a simple NXDOMAIN response
    // TODO: Implement actual upstream forwarding using hickory-client
    warn!("Upstream forwarding not implemented yet, returning NXDOMAIN");
    build_error_response(query, ResponseCode::NXDomain)
}

/// Build a blocked response (NXDOMAIN or custom IP)
fn build_blocked_response(query: &Message, blocked_ip: &str) -> anyhow::Result<Message> {
    if blocked_ip == "0.0.0.0" {
        // Return NXDOMAIN
        build_error_response(query, ResponseCode::NXDomain)
    } else {
        // Return custom IP
        let mut response = Message::new();
        response.set_id(query.id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_response_code(ResponseCode::NoError);
        response.add_queries(query.queries().to_vec());

        // Add answer record with custom IP
        if let Some(q) = query.queries().first() {
            let ip = Ipv4Addr::from_str(blocked_ip).unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
            let record = Record::from_rdata(
                q.name().clone(),
                60, // TTL
                RData::A(ip.into()),
            );
            response.add_answer(record);
        }

        Ok(response)
    }
}

/// Build an error response
fn build_error_response(query: &Message, rcode: ResponseCode) -> anyhow::Result<Message> {
    let mut response = Message::new();
    response.set_id(query.id());
    response.set_message_type(MessageType::Response);
    response.set_op_code(OpCode::Query);
    response.set_response_code(rcode);
    response.add_queries(query.queries().to_vec());

    Ok(response)
}

/// Log DNS query asynchronously via NATS
async fn log_query_async(
    state: &Arc<AppState>,
    tenant_id: &uuid::Uuid,
    config_id: Option<&uuid::Uuid>,
    domain: &str,
    result_type: DnsResultType,
    protocol: &str,
) -> anyhow::Result<()> {
    let message = DnsQueryLogMessage {
        tenant_id: *tenant_id,
        config_id: config_id.copied(),
        query_timestamp: chrono::Utc::now(),
        query_domain_name: domain.to_string(),
        query_type: common::models::DnsQueryType::A, // TODO: Get actual query type
        query_result_type: result_type,
        response_ip_address: None,
        query_latency_ms: None,
        query_source_protocol: Some(protocol.to_string()),
        query_source_ip: None,
    };

    // Publish to NATS (fire and forget)
    if let Err(e) = state.nats.publish("dns.query.log", &message).await {
        error!("Failed to publish DNS query log: {}", e);
    }

    Ok(())
}

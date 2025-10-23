use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, register_int_gauge_vec,
    HistogramVec, IntCounterVec, IntGauge, IntGaugeVec,
};

lazy_static! {
    // ============================================================================
    // DNS Query Metrics
    // ============================================================================

    /// Total number of DNS queries processed
    pub static ref DNS_QUERIES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "dns_queries_total",
        "Total number of DNS queries processed",
        &["protocol", "result"]  // doh/dot/udp, allowed/blocked/whitelisted/error
    )
    .unwrap();

    /// DNS query processing duration in seconds
    pub static ref DNS_QUERY_DURATION: HistogramVec = register_histogram_vec!(
        "dns_query_duration_seconds",
        "DNS query processing duration in seconds",
        &["protocol"],  // doh/dot/udp
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0]
    )
    .unwrap();

    /// Number of blocked DNS queries
    pub static ref DNS_BLOCKED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "dns_blocked_total",
        "Total number of blocked DNS queries",
        &["category"]  // advertising, malware, etc.
    )
    .unwrap();

    /// Number of whitelisted DNS queries
    pub static ref DNS_WHITELISTED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "dns_whitelisted_total",
        "Total number of whitelisted DNS queries",
        &["tenant_id"]
    )
    .unwrap();

    /// Active DNS connections
    pub static ref DNS_ACTIVE_CONNECTIONS: IntGaugeVec = register_int_gauge_vec!(
        "dns_active_connections",
        "Number of active DNS connections",
        &["protocol"]  // doh/dot/udp
    )
    .unwrap();

    // ============================================================================
    // Cache Metrics
    // ============================================================================

    /// Cache hits by layer and type
    pub static ref CACHE_HITS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_hits_total",
        "Total number of cache hits",
        &["layer", "type"]  // memory/valkey/database, tenant/blocklist/whitelist
    )
    .unwrap();

    /// Cache misses by layer and type
    pub static ref CACHE_MISSES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_misses_total",
        "Total number of cache misses",
        &["layer", "type"]
    )
    .unwrap();

    /// Memory cache entry count
    pub static ref CACHE_MEMORY_ENTRIES: IntGauge = register_int_gauge!(
        "cache_memory_entries",
        "Number of entries in memory cache"
    )
    .unwrap();

    /// Memory cache size in bytes
    pub static ref CACHE_MEMORY_BYTES: IntGauge = register_int_gauge!(
        "cache_memory_bytes",
        "Memory cache size in bytes"
    )
    .unwrap();

    // ============================================================================
    // Database Metrics
    // ============================================================================

    /// Number of active database connections
    pub static ref DB_CONNECTIONS_ACTIVE: IntGauge = register_int_gauge!(
        "db_connections_active",
        "Number of active database connections"
    )
    .unwrap();

    /// Number of idle database connections
    pub static ref DB_CONNECTIONS_IDLE: IntGauge = register_int_gauge!(
        "db_connections_idle",
        "Number of idle database connections"
    )
    .unwrap();

    /// Database query duration
    pub static ref DB_QUERY_DURATION: HistogramVec = register_histogram_vec!(
        "db_query_duration_seconds",
        "Database query duration in seconds",
        &["operation"],  // select/insert/update/delete
        vec![0.001, 0.005, 0.010, 0.050, 0.100, 0.500, 1.0, 5.0]
    )
    .unwrap();

    /// Database errors
    pub static ref DB_ERRORS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "db_errors_total",
        "Total number of database errors",
        &["operation", "error_type"]
    )
    .unwrap();

    // ============================================================================
    // API Metrics
    // ============================================================================

    /// Total API requests
    pub static ref API_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "api_requests_total",
        "Total number of API requests",
        &["method", "path", "status"]  // GET/POST/PUT/DELETE, /api/v1/*, 200/400/500
    )
    .unwrap();

    /// API request duration
    pub static ref API_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "api_request_duration_seconds",
        "API request duration in seconds",
        &["method", "path"],
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .unwrap();

    /// Active API connections
    pub static ref API_ACTIVE_CONNECTIONS: IntGauge = register_int_gauge!(
        "api_active_connections",
        "Number of active API connections"
    )
    .unwrap();

    // ============================================================================
    // Authentication Metrics
    // ============================================================================

    /// Authentication attempts
    pub static ref AUTH_ATTEMPTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "auth_attempts_total",
        "Total number of authentication attempts",
        &["method", "result"]  // email_otp/fido2/totp, success/failure
    )
    .unwrap();

    /// Active sessions
    pub static ref AUTH_ACTIVE_SESSIONS: IntGauge = register_int_gauge!(
        "auth_active_sessions",
        "Number of active authentication sessions"
    )
    .unwrap();

    // ============================================================================
    // Tenant Metrics
    // ============================================================================

    /// Total number of tenants
    pub static ref TENANTS_TOTAL: IntGaugeVec = register_int_gauge_vec!(
        "tenants_total",
        "Total number of tenants",
        &["tier", "status"]  // free/pro/family/business, active/expired
    )
    .unwrap();

    /// Tenant query quota usage
    pub static ref TENANT_QUERY_QUOTA_USAGE: IntGaugeVec = register_int_gauge_vec!(
        "tenant_query_quota_usage",
        "Tenant query quota usage percentage",
        &["tenant_id"]
    )
    .unwrap();

    // ============================================================================
    // Worker Metrics
    // ============================================================================

    /// NATS messages processed
    pub static ref NATS_MESSAGES_PROCESSED: IntCounterVec = register_int_counter_vec!(
        "nats_messages_processed",
        "Total number of NATS messages processed",
        &["subject", "result"]  // dns.query.log/email.send, success/failure
    )
    .unwrap();

    /// NATS queue depth
    pub static ref NATS_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec!(
        "nats_queue_depth",
        "Number of pending messages in NATS queue",
        &["subject"]
    )
    .unwrap();

    /// Background job duration
    pub static ref BACKGROUND_JOB_DURATION: HistogramVec = register_histogram_vec!(
        "background_job_duration_seconds",
        "Background job duration in seconds",
        &["job_type"],  // dns_logger/blocklist_updater/analytics_aggregator
        vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]
    )
    .unwrap();
}

/// Metrics exporter for Prometheus
pub fn metrics_handler() -> String {
    use prometheus::{Encoder, TextEncoder};

    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    String::from_utf8(buffer).unwrap()
}

/// Helper macros for timing operations
#[macro_export]
macro_rules! time_operation {
    ($histogram:expr, $labels:expr, $operation:expr) => {{
        let timer = $histogram.with_label_values($labels).start_timer();
        let result = $operation;
        timer.observe_duration();
        result
    }};
}

#[macro_export]
macro_rules! time_async_operation {
    ($histogram:expr, $labels:expr, $operation:expr) => {{
        let timer = $histogram.with_label_values($labels).start_timer();
        let result = $operation.await;
        timer.observe_duration();
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registration() {
        // All metrics should be registered successfully
        assert!(DNS_QUERIES_TOTAL.with_label_values(&["doh", "allowed"]).get() >= 0);
        assert!(CACHE_HITS_TOTAL.with_label_values(&["memory", "tenant"]).get() >= 0);
        assert!(DB_CONNECTIONS_ACTIVE.get() >= 0);
    }
}

# System Architecture Document
## MightyDNS - High-Performance DNS Sinkhole Service

**Version:** 1.0
**Last Updated:** 2025-10-22
**Status:** Draft

---

## 1. Executive Summary

This document describes the complete system architecture for MightyDNS, a multi-tenant DNS sinkhole service designed to handle 10 million concurrent users with sub-10ms query latency. The architecture emphasizes horizontal scalability, fault tolerance, and tenant isolation.

### 1.1 Design Goals

| Goal | Target | Implementation Strategy |
|------|--------|------------------------|
| **Scalability** | 10M concurrent users | Horizontal scaling, stateless services, partitioned database |
| **Performance** | <10ms DNS query latency (p95) | In-memory caching, Redis, async I/O, geo-distributed nodes |
| **Reliability** | 99.9% uptime | Multi-region deployment, auto-failover, health checks |
| **Security** | Zero trust, encryption everywhere | TLS 1.3, JWT auth, tenant isolation, rate limiting |
| **Maintainability** | Easy deployment & monitoring | IaC, GitOps, centralized logging, metrics |

---

## 2. High-Level Architecture

### 2.1 System Layers

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Client Layer                                │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐          │
│  │ Browsers │ OS DNS   │ Mobile   │ Routers  │ IoT      │          │
│  │ (DoH)    │ (DoT/UDP)│ Apps     │ (UDP/53) │ Devices  │          │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘          │
└─────────────────────────────────────────────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Edge/CDN Layer                                │
│  ┌─────────────────────────────────────────────────────┐            │
│  │  Cloudflare (DDoS Protection, TLS Termination)      │            │
│  │  - Global Anycast IP (UDP/53)                       │            │
│  │  - TLS 1.3 termination (DoH/DoT)                    │            │
│  │  - WAF (Web Application Firewall)                   │            │
│  └─────────────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                  Load Balancer Layer                                │
│  ┌─────────────────────────────────────────────────────┐            │
│  │  Kubernetes Ingress + Service Mesh (Linkerd)        │            │
│  │  - Layer 4/7 load balancing                         │            │
│  │  - Health checks                                     │            │
│  │  - Request routing                                   │            │
│  └─────────────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                  Application Layer                                  │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┐     │
│  │  DNS Server  │  DNS Server  │  DNS Server  │  DNS Server  │     │
│  │  Pod (Rust)  │  Pod (Rust)  │  Pod (Rust)  │  Pod (Rust)  │     │
│  │  - DoH       │  - DoH       │  - DoH       │  - DoH       │     │
│  │  - DoT       │  - DoT       │  - DoT       │  - DoT       │     │
│  │  - UDP/53    │  - UDP/53    │  - UDP/53    │  - UDP/53    │     │
│  └──────────────┴──────────────┴──────────────┴──────────────┘     │
│                                                                      │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┐     │
│  │  API Server  │  API Server  │  API Server  │  API Server  │     │
│  │  Pod (Rust)  │  Pod (Rust)  │  Pod (Rust)  │  Pod (Rust)  │     │
│  │  - REST API  │  - REST API  │  - REST API  │  - REST API  │     │
│  │  - WebSocket │  - WebSocket │  - WebSocket │  - WebSocket │     │
│  │  - Auth      │  - Auth      │  - Auth      │  - Auth      │     │
│  └──────────────┴──────────────┴──────────────┴──────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Caching Layer                                    │
│  ┌────────────────────────┬────────────────────────┐               │
│  │  Redis Cluster         │  Valkey Cluster        │               │
│  │  (DNS Cache, Blocklist)│  (Sessions, Auth)      │               │
│  │  - 6 nodes (3+3)       │  - 6 nodes (3+3)       │               │
│  │  - Sharded by key      │  - Sharded by key      │               │
│  │  - 100K ops/sec        │  - 100K ops/sec        │               │
│  └────────────────────────┴────────────────────────┘               │
└─────────────────────────────────────────────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                  Data Storage Layer                                 │
│  ┌─────────────────────────────────────────────────────┐            │
│  │  PostgreSQL 16 Cluster (Primary + 2 Replicas)       │            │
│  │  - Hash partitioning (tenant_id, 1024 partitions)   │            │
│  │  - Range partitioning (timestamps)                  │            │
│  │  - TimescaleDB extension (time-series)              │            │
│  │  - pg_cron (scheduled jobs)                         │            │
│  │  - PgBouncer (connection pooling)                   │            │
│  └─────────────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                  Message Queue Layer                                │
│  ┌─────────────────────────────────────────────────────┐            │
│  │  NATS JetStream Cluster                             │            │
│  │  - Async DNS query logging                          │            │
│  │  - Analytics aggregation jobs                       │            │
│  │  - Email notifications                              │            │
│  │  - Blocklist update events                          │            │
│  └─────────────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────────────┘
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                 Observability Layer                                 │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┐     │
│  │  Prometheus  │  Grafana     │  Loki        │  Jaeger      │     │
│  │  (Metrics)   │  (Dashboards)│  (Logs)      │  (Tracing)   │     │
│  └──────────────┴──────────────┴──────────────┴──────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Detailed Component Architecture

### 3.1 DNS Server Component

```rust
// src/dns_server/main.rs
// High-level structure of DNS server

use tokio::net::{TcpListener, UdpSocket};
use hickory_server::ServerFuture;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize shared state
    let app_state = Arc::new(AppState {
        db_pool: init_database_pool().await?,
        redis_pool: init_redis_pool().await?,
        nats_client: init_nats_client().await?,
        tenant_cache: Arc::new(TenantCache::new()),
    });

    // Spawn DNS servers concurrently
    tokio::try_join!(
        serve_doh(app_state.clone()),      // DNS over HTTPS
        serve_dot(app_state.clone()),      // DNS over TLS
        serve_udp(app_state.clone()),      // Traditional UDP/53
        serve_tcp(app_state.clone()),      // Traditional TCP/53
    )?;

    Ok(())
}
```

#### 3.1.1 DNS over HTTPS (DoH) Handler

```rust
// src/dns_server/handlers/doh.rs

use axum::{
    Router,
    routing::{get, post},
    extract::{Path, Query, State},
    body::Bytes,
    http::StatusCode,
};
use hickory_proto::op::Message;

pub fn doh_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dns-query/:tenant_id", get(handle_doh_get))
        .route("/dns-query/:tenant_id", post(handle_doh_post))
}

// RFC 8484: GET /dns-query?dns=base64url(query)
async fn handle_doh_get(
    Path(tenant_id): Path<String>,
    Query(params): Query<DohQueryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Vec<u8>, DnsError> {
    // 1. Decode base64url DNS query
    let query_bytes = base64_url::decode(&params.dns)?;
    let dns_message = Message::from_vec(&query_bytes)?;

    // 2. Resolve with tenant context
    let response = resolve_dns_query(tenant_id, dns_message, state).await?;

    // 3. Return DNS response as application/dns-message
    Ok(response.to_vec()?)
}

// RFC 8484: POST /dns-query with body
async fn handle_doh_post(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<Vec<u8>, DnsError> {
    let dns_message = Message::from_vec(&body)?;
    let response = resolve_dns_query(tenant_id, dns_message, state).await?;
    Ok(response.to_vec()?)
}

// Core DNS resolution logic
async fn resolve_dns_query(
    tenant_id: String,
    query: Message,
    state: Arc<AppState>,
) -> Result<Message, DnsError> {
    let start_time = Instant::now();

    // 1. Get tenant config (cached)
    let tenant = state.tenant_cache.get_or_fetch(&tenant_id, &state.db_pool).await?;

    // 2. Extract query domain
    let query_name = query.queries()[0].name().to_string();

    // 3. Check whitelist (Redis cache)
    if is_whitelisted(&tenant.tenant_id, &query_name, &state.redis_pool).await? {
        metrics::DNS_WHITELISTED_TOTAL.inc();
        return forward_to_upstream(query).await;
    }

    // 4. Check blocklist (Redis + PostgreSQL)
    if let Some(block_reason) = check_blocklist(&tenant, &query_name, &state).await? {
        metrics::DNS_BLOCKED_TOTAL.inc();
        log_query_async(&tenant, &query_name, "blocked", &state.nats_client).await?;

        // Return NXDOMAIN or custom IP
        return Ok(build_blocked_response(&query, tenant.blocked_response_ip));
    }

    // 5. Forward to upstream DNS (Cloudflare 1.1.1.1, Google 8.8.8.8)
    let upstream_response = forward_to_upstream(query).await?;

    // 6. Cache response in Redis (TTL = DNS TTL)
    cache_dns_response(&query_name, &upstream_response, &state.redis_pool).await?;

    // 7. Log query async (via NATS)
    if tenant.is_logging_enabled {
        log_query_async(&tenant, &query_name, "allowed", &state.nats_client).await?;
    }

    // 8. Record latency
    let latency = start_time.elapsed();
    metrics::DNS_QUERY_DURATION.observe(latency.as_secs_f64());

    Ok(upstream_response)
}
```

#### 3.1.2 DNS over TLS (DoT) Handler

```rust
// src/dns_server/handlers/dot.rs

use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use hickory_proto::op::Message;

pub async fn serve_dot(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    // Load TLS certificates
    let tls_config = load_tls_config("certs/fullchain.pem", "certs/privkey.pem")?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let listener = TcpListener::bind("0.0.0.0:853").await?;
    info!("DoT server listening on port 853");

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let acceptor = tls_acceptor.clone();
        let state = state.clone();

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    if let Err(e) = handle_dot_connection(tls_stream, client_addr, state).await {
                        error!("DoT connection error: {}", e);
                    }
                }
                Err(e) => error!("TLS handshake failed: {}", e),
            }
        });
    }
}

async fn handle_dot_connection(
    stream: TlsStream<TcpStream>,
    client_addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), DnsError> {
    // Extract SNI (Server Name Indication) for tenant identification
    let (_, tls_session) = stream.get_ref();
    let sni = tls_session
        .server_name()
        .ok_or(DnsError::MissingSNI)?;

    // Parse tenant_id from SNI (e.g., "abc123.dns.mightydns.com" → "abc123")
    let tenant_id = parse_tenant_from_sni(sni)?;

    // Handle DNS queries over TLS stream
    let mut stream = stream;
    loop {
        // Read DNS message (2-byte length prefix + message)
        let length = stream.read_u16().await?;
        let mut buffer = vec![0u8; length as usize];
        stream.read_exact(&mut buffer).await?;

        let query = Message::from_vec(&buffer)?;

        // Resolve query with tenant context
        let response = resolve_dns_query(tenant_id.clone(), query, state.clone()).await?;

        // Write response (2-byte length + message)
        stream.write_u16(response.len() as u16).await?;
        stream.write_all(&response.to_vec()?).await?;
        stream.flush().await?;
    }
}
```

#### 3.1.3 UDP/53 Handler (Traditional DNS)

```rust
// src/dns_server/handlers/udp.rs

use tokio::net::UdpSocket;
use hickory_proto::op::Message;

pub async fn serve_udp(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:53").await?;
    info!("UDP DNS server listening on port 53");

    let mut buffer = vec![0u8; 512]; // DNS over UDP limited to 512 bytes

    loop {
        let (len, src_addr) = socket.recv_from(&mut buffer).await?;

        let query_bytes = buffer[..len].to_vec();
        let socket_clone = socket.clone();
        let state_clone = state.clone();

        // Handle query in separate task (non-blocking)
        tokio::spawn(async move {
            match handle_udp_query(query_bytes, src_addr, state_clone).await {
                Ok(response) => {
                    if let Err(e) = socket_clone.send_to(&response, src_addr).await {
                        error!("Failed to send UDP response: {}", e);
                    }
                }
                Err(e) => error!("UDP query error: {}", e),
            }
        });
    }
}

async fn handle_udp_query(
    query_bytes: Vec<u8>,
    src_addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<Vec<u8>, DnsError> {
    let query = Message::from_vec(&query_bytes)?;

    // Lookup tenant by source IP address
    let tenant = match lookup_tenant_by_ip(&src_addr.ip(), &state).await? {
        Some(t) => t,
        None => {
            // No tenant found for this IP - use default/public config or reject
            return Ok(build_error_response(&query, ResponseCode::Refused).to_vec()?);
        }
    };

    // Resolve query with tenant context
    let response = resolve_dns_query(tenant.tenant_identifier, query, state).await?;

    Ok(response.to_vec()?)
}

async fn lookup_tenant_by_ip(
    ip: &IpAddr,
    state: &Arc<AppState>,
) -> Result<Option<TenantConfig>, DnsError> {
    // Try Redis cache first
    let cache_key = format!("ip_binding:{}", ip);
    if let Some(cached) = state.redis_pool.get::<_, String>(&cache_key).await? {
        return Ok(Some(serde_json::from_str(&cached)?));
    }

    // Query database
    let tenant = sqlx::query_as!(
        TenantConfig,
        r#"
        SELECT
            ta.tenant_id,
            ta.tenant_identifier,
            tc.config_id,
            tc.is_logging_enabled,
            tc.blocked_response_ip
        FROM tenant_ip_binding tib
        JOIN tenant_account ta ON tib.tenant_id = ta.tenant_id
        JOIN tenant_config tc ON tib.config_id = tc.config_id
        WHERE tib.ip_address = $1
          AND tib.is_binding_active = TRUE
          AND ta.is_account_active = TRUE
        LIMIT 1
        "#,
        ip
    )
    .fetch_optional(&state.db_pool)
    .await?;

    // Cache result for 60 seconds
    if let Some(ref t) = tenant {
        let _ = state.redis_pool
            .set_ex(&cache_key, serde_json::to_string(t)?, 60)
            .await;
    }

    Ok(tenant)
}
```

---

### 3.2 API Service Component

```rust
// src/api_service/main.rs

use axum::{Router, middleware};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::init().await?);

    let app = Router::new()
        // Public routes (no auth required)
        .nest("/auth", auth_routes())

        // Protected routes (JWT required)
        .nest("/api/v1", api_v1_routes())
            .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))

        // Webhooks (signature verification)
        .nest("/webhooks", webhook_routes())
            .layer(middleware::from_fn(webhook_signature_middleware))

        // Metrics endpoint (Prometheus)
        .route("/metrics", get(metrics_handler))

        // Health checks
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))

        // CORS and state
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    info!("API server listening on {}", addr);

    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

#### 3.2.1 Authentication Routes

```rust
// src/api_service/routes/auth.rs

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/signup", post(signup_handler))
        .route("/login-request", post(login_request_handler))
        .route("/verify-otp", post(verify_otp_handler))
        .route("/verify-magic", get(verify_magic_link_handler))
        .route("/webauthn/register-start", post(webauthn_register_start))
        .route("/webauthn/register-finish", post(webauthn_register_finish))
        .route("/webauthn/login-start", post(webauthn_login_start))
        .route("/webauthn/login-finish", post(webauthn_login_finish))
        .route("/totp/register", post(totp_register_handler))
        .route("/totp/verify", post(totp_verify_handler))
        .route("/logout", post(logout_handler))
}

// Signup/Login request (passwordless)
async fn login_request_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    // 1. Check if user exists
    let tenant = sqlx::query_as!(
        TenantAccount,
        "SELECT * FROM tenant_account WHERE email_address = $1 AND is_account_active = TRUE",
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await?;

    let tenant_id = match tenant {
        Some(t) => t.tenant_id,
        None => {
            // Create new account
            let new_tenant = sqlx::query_as!(
                TenantAccount,
                r#"
                INSERT INTO tenant_account (email_address, tenant_identifier)
                VALUES ($1, $2)
                RETURNING *
                "#,
                payload.email,
                generate_tenant_identifier()
            )
            .fetch_one(&state.db_pool)
            .await?;

            new_tenant.tenant_id
        }
    };

    // 2. Generate OTP (6 digits)
    let otp = generate_otp();

    // 3. Store OTP in Redis (10-minute expiry)
    state.redis_pool
        .set_ex(format!("otp:{}", tenant_id), &otp, 600)
        .await?;

    // 4. Generate magic link token
    let magic_token = generate_secure_token();
    state.redis_pool
        .set_ex(format!("magic:{}", magic_token), &tenant_id.to_string(), 600)
        .await?;

    // 5. Send email (async via NATS)
    let email_payload = EmailPayload {
        to: payload.email.clone(),
        subject: "Your MightyDNS Login Code".to_string(),
        template: "login_otp".to_string(),
        variables: json!({
            "otp": otp,
            "magic_link": format!("https://mightydns.com/auth/verify-magic?token={}", magic_token)
        }),
    };
    state.nats_client.publish("email.send", serde_json::to_vec(&email_payload)?).await?;

    Ok(Json(LoginResponse {
        success: true,
        message: "Check your email for OTP or magic link".to_string(),
    }))
}

// Verify OTP and create session
async fn verify_otp_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyOtpRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    // 1. Get tenant by email
    let tenant = sqlx::query_as!(
        TenantAccount,
        "SELECT * FROM tenant_account WHERE email_address = $1",
        payload.email
    )
    .fetch_one(&state.db_pool)
    .await?;

    // 2. Verify OTP from Redis
    let stored_otp: String = state.redis_pool
        .get(format!("otp:{}", tenant.tenant_id))
        .await?
        .ok_or(ApiError::InvalidOtp)?;

    if stored_otp != payload.otp {
        return Err(ApiError::InvalidOtp);
    }

    // 3. Delete OTP (one-time use)
    state.redis_pool.del(format!("otp:{}", tenant.tenant_id)).await?;

    // 4. Create session
    let session = create_session(&tenant, &state).await?;

    Ok(Json(SessionResponse {
        token: session.jwt_token,
        tenant_id: tenant.tenant_id,
        email: tenant.email_address,
        subscription_tier: tenant.subscription_tier,
    }))
}

// Create JWT session
async fn create_session(
    tenant: &TenantAccount,
    state: &Arc<AppState>,
) -> Result<Session, ApiError> {
    // Generate JWT
    let claims = JwtClaims {
        sub: tenant.tenant_id.to_string(),
        email: tenant.email_address.clone(),
        tier: tenant.subscription_tier.clone(),
        exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
    };

    let jwt_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )?;

    // Hash token for storage
    let token_hash = hash_sha256(&jwt_token);

    // Store session in database
    sqlx::query!(
        r#"
        INSERT INTO auth_session (tenant_id, session_token_hash, session_ip_address, session_expires_at)
        VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour')
        "#,
        tenant.tenant_id,
        token_hash,
        "0.0.0.0" // TODO: Get real IP from request
    )
    .execute(&state.db_pool)
    .await?;

    // Cache session in Redis
    state.redis_pool
        .set_ex(format!("session:{}", token_hash), &tenant.tenant_id.to_string(), 3600)
        .await?;

    Ok(Session {
        jwt_token,
        token_hash,
    })
}
```

#### 3.2.2 Tenant Management Routes

```rust
// src/api_service/routes/tenants.rs

pub fn tenant_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/account", get(get_account_handler))
        .route("/configs", get(list_configs_handler))
        .route("/configs", post(create_config_handler))
        .route("/configs/:config_id", get(get_config_handler))
        .route("/configs/:config_id", put(update_config_handler))
        .route("/configs/:config_id", delete(delete_config_handler))
        .route("/ip-bindings", get(list_ip_bindings_handler))
        .route("/ip-bindings", post(create_ip_binding_handler))
        .route("/ip-bindings/:binding_id", delete(delete_ip_binding_handler))
}

// Get tenant account info
async fn get_account_handler(
    State(state): State<Arc<AppState>>,
    Extension(tenant_id): Extension<Uuid>, // Injected by auth middleware
) -> Result<Json<AccountInfo>, ApiError> {
    let account = sqlx::query_as!(
        AccountInfo,
        r#"
        SELECT
            ta.tenant_id,
            ta.email_address,
            ta.tenant_identifier,
            ta.subscription_tier,
            ta.subscription_status,
            ta.account_created_at,
            ts.monthly_query_limit,
            ts.config_limit,
            COUNT(DISTINCT tc.config_id)::INT as config_count,
            COUNT(DISTINCT ac.credential_id)::INT as credential_count
        FROM tenant_account ta
        LEFT JOIN tenant_subscription ts ON ta.tenant_id = ts.tenant_id
        LEFT JOIN tenant_config tc ON ta.tenant_id = tc.tenant_id AND tc.is_config_active = TRUE
        LEFT JOIN auth_credential ac ON ta.tenant_id = ac.tenant_id AND ac.is_credential_active = TRUE
        WHERE ta.tenant_id = $1
        GROUP BY ta.tenant_id, ts.monthly_query_limit, ts.config_limit
        "#,
        tenant_id
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(account))
}

// Create new DNS configuration
async fn create_config_handler(
    State(state): State<Arc<AppState>>,
    Extension(tenant_id): Extension<Uuid>,
    Json(payload): Json<CreateConfigRequest>,
) -> Result<Json<ConfigInfo>, ApiError> {
    // Check subscription limits
    let subscription = get_subscription(&tenant_id, &state).await?;
    let current_config_count = count_configs(&tenant_id, &state).await?;

    if current_config_count >= subscription.config_limit {
        return Err(ApiError::LimitExceeded(format!(
            "Config limit reached ({}/{}). Upgrade to create more.",
            current_config_count, subscription.config_limit
        )));
    }

    // Create config
    let config = sqlx::query_as!(
        ConfigInfo,
        r#"
        INSERT INTO tenant_config (tenant_id, config_name, config_description, is_logging_enabled, blocked_response_ip)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
        tenant_id,
        payload.config_name,
        payload.config_description,
        payload.is_logging_enabled.unwrap_or(true),
        payload.blocked_response_ip.unwrap_or("0.0.0.0".parse::<IpAddr>().unwrap())
    )
    .fetch_one(&state.db_pool)
    .await?;

    // Insert block categories
    if let Some(categories) = payload.block_categories {
        for category in categories {
            sqlx::query!(
                r#"
                INSERT INTO config_block_category (config_id, block_category, is_category_enabled)
                VALUES ($1, $2, TRUE)
                "#,
                config.config_id,
                category
            )
            .execute(&state.db_pool)
            .await?;
        }
    }

    // Invalidate tenant config cache
    state.redis_pool.del(format!("tenant_config:{}", tenant_id)).await?;

    Ok(Json(config))
}
```

---

### 3.3 Caching Strategy

#### 3.3.1 Multi-Layer Cache Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Layer 1: In-Memory Cache                     │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  LRU Cache (per DNS server process)                      │   │
│  │  - Capacity: 1M entries                                  │   │
│  │  - TTL: DNS response TTL                                 │   │
│  │  - Hit Rate: 90%+                                        │   │
│  │  - Lookup Time: <1μs                                     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              ▼ Cache Miss
┌─────────────────────────────────────────────────────────────────┐
│                    Layer 2: Redis Cache                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Redis Cluster (shared across all DNS servers)          │   │
│  │  - Blocklist: Redis Set (SISMEMBER O(1) lookup)         │   │
│  │  - Whitelist: Redis Set (per tenant)                    │   │
│  │  - DNS Responses: String with TTL                       │   │
│  │  - Tenant Configs: JSON with 60s TTL                    │   │
│  │  - Hit Rate: 95%+                                        │   │
│  │  - Lookup Time: <1ms                                     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              ▼ Cache Miss
┌─────────────────────────────────────────────────────────────────┐
│                 Layer 3: PostgreSQL Database                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Materialized Views (refreshed every 60s)               │   │
│  │  - mv_tenant_config_lookup                              │   │
│  │  - Indexed queries on partitioned tables                │   │
│  │  - Lookup Time: 5-10ms                                  │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

#### 3.3.2 Cache Implementation

```rust
// src/common/cache.rs

use moka::future::Cache;
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

pub struct MultiLayerCache {
    // L1: In-memory LRU cache
    memory_cache: Cache<String, CacheValue>,

    // L2: Redis cluster
    redis_pool: RedisPool,

    // L3: PostgreSQL
    pg_pool: PgPool,
}

impl MultiLayerCache {
    pub async fn get_blocklist_status(
        &self,
        tenant_id: &Uuid,
        domain: &str,
    ) -> Result<BlocklistStatus, CacheError> {
        let cache_key = format!("blocklist:{}:{}", tenant_id, domain);

        // L1: Check memory cache
        if let Some(cached) = self.memory_cache.get(&cache_key).await {
            metrics::CACHE_HITS_TOTAL.with_label_values(&["memory", "blocklist"]).inc();
            return Ok(cached.into_blocklist_status());
        }

        // L2: Check Redis
        let redis_conn = self.redis_pool.get().await?;
        if let Some(is_blocked) = redis_conn.get::<_, Option<bool>>(&cache_key).await? {
            metrics::CACHE_HITS_TOTAL.with_label_values(&["redis", "blocklist"]).inc();

            // Backfill L1 cache
            self.memory_cache.insert(
                cache_key.clone(),
                CacheValue::BlocklistStatus(is_blocked),
                Duration::from_secs(300),
            ).await;

            return Ok(BlocklistStatus { is_blocked });
        }

        // L3: Query database
        metrics::CACHE_MISSES_TOTAL.with_label_values(&["database", "blocklist"]).inc();

        let result = sqlx::query_as!(
            BlocklistStatus,
            "SELECT * FROM check_domain_blocklist($1, $2, $3)",
            domain,
            tenant_id,
            Uuid::nil() // config_id placeholder
        )
        .fetch_one(&self.pg_pool)
        .await?;

        // Backfill L2 (Redis) and L1 (memory) caches
        redis_conn.set_ex(&cache_key, result.is_blocked, 300).await?;
        self.memory_cache.insert(
            cache_key,
            CacheValue::BlocklistStatus(result.is_blocked),
            Duration::from_secs(300),
        ).await;

        Ok(result)
    }

    pub async fn invalidate(&self, key: &str) -> Result<(), CacheError> {
        // Invalidate all cache layers
        self.memory_cache.invalidate(key).await;
        let redis_conn = self.redis_pool.get().await?;
        redis_conn.del(key).await?;
        Ok(())
    }
}
```

---

### 3.4 Asynchronous Processing with NATS

```rust
// src/workers/dns_logger.rs

use async_nats::jetstream;

pub async fn start_dns_logger_worker(nats_client: async_nats::Client, pg_pool: PgPool) {
    let jetstream = jetstream::new(nats_client);

    // Create or get stream
    let stream = jetstream
        .get_or_create_stream(jetstream::stream::Config {
            name: "DNS_QUERIES".to_string(),
            subjects: vec!["dns.query.log".to_string()],
            max_age: Duration::from_secs(86400), // 24-hour retention
            ..Default::default()
        })
        .await
        .expect("Failed to create stream");

    // Create consumer
    let consumer = stream
        .create_consumer(jetstream::consumer::pull::Config {
            durable_name: Some("dns-logger".to_string()),
            ..Default::default()
        })
        .await
        .expect("Failed to create consumer");

    // Batch buffer
    let mut batch = Vec::with_capacity(1000);
    let mut last_flush = Instant::now();

    loop {
        let messages = consumer.messages().await.expect("Failed to get messages");

        while let Some(msg) = messages.next().await {
            let query_log: DnsQueryLog = match serde_json::from_slice(&msg.payload) {
                Ok(log) => log,
                Err(e) => {
                    error!("Failed to parse DNS query log: {}", e);
                    msg.ack().await.ok();
                    continue;
                }
            };

            batch.push(query_log);

            // Flush batch if:
            // 1. Batch size reaches 1000, OR
            // 2. 5 seconds have elapsed since last flush
            if batch.len() >= 1000 || last_flush.elapsed() > Duration::from_secs(5) {
                if let Err(e) = flush_batch(&batch, &pg_pool).await {
                    error!("Failed to flush DNS query logs: {}", e);
                } else {
                    // Acknowledge all messages in batch
                    msg.ack().await.ok();
                }

                batch.clear();
                last_flush = Instant::now();
            }
        }
    }
}

async fn flush_batch(logs: &[DnsQueryLog], pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for log in logs {
        sqlx::query!(
            r#"
            INSERT INTO dns_query_log (
                tenant_id, config_id, query_timestamp, query_domain_name,
                query_type, query_result_type, response_ip_address, query_latency_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            log.tenant_id,
            log.config_id,
            log.query_timestamp,
            log.query_domain_name,
            log.query_type,
            log.query_result_type,
            log.response_ip_address,
            log.query_latency_ms
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    info!("Flushed {} DNS query logs to database", logs.len());
    Ok(())
}
```

---

## 4. Deployment Architecture

### 4.1 Kubernetes Manifest

```yaml
# k8s/dns-server-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dns-server
  namespace: mightydns
spec:
  replicas: 10
  selector:
    matchLabels:
      app: dns-server
  template:
    metadata:
      labels:
        app: dns-server
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      containers:
      - name: dns-server
        image: mightydns/dns-server:latest
        ports:
        - containerPort: 853
          name: dot
          protocol: TCP
        - containerPort: 53
          name: dns-udp
          protocol: UDP
        - containerPort: 53
          name: dns-tcp
          protocol: TCP
        - containerPort: 8443
          name: doh
          protocol: TCP
        - containerPort: 9090
          name: metrics
          protocol: TCP
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-credentials
              key: url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: redis-credentials
              key: url
        - name: NATS_URL
          value: "nats://nats.mightydns.svc.cluster.local:4222"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "2000m"
        livenessProbe:
          tcpSocket:
            port: 853
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 10
          periodSeconds: 5

---
apiVersion: v1
kind: Service
metadata:
  name: dns-server
  namespace: mightydns
spec:
  type: LoadBalancer
  loadBalancerIP: 1.2.3.4  # Static anycast IP
  selector:
    app: dns-server
  ports:
  - name: dot
    port: 853
    targetPort: 853
    protocol: TCP
  - name: dns-udp
    port: 53
    targetPort: 53
    protocol: UDP
  - name: dns-tcp
    port: 53
    targetPort: 53
    protocol: TCP
  - name: doh
    port: 443
    targetPort: 8443
    protocol: TCP

---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: dns-server-hpa
  namespace: mightydns
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: dns-server
  minReplicas: 10
  maxReplicas: 1000
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: dns_queries_per_second
      target:
        type: AverageValue
        averageValue: "80k"  # Scale when >80K QPS per pod
```

### 4.2 Multi-Region Deployment

```
Global Deployment Topology:
───────────────────────────────────────────────────────────────────

Region: US-East (us-east-1)
├── Kubernetes Cluster
│   ├── DNS Servers: 100 pods
│   ├── API Servers: 50 pods
│   ├── Workers: 20 pods
├── PostgreSQL Primary (RDS)
├── Redis Cluster (6 nodes)
└── NATS Cluster (3 nodes)

Region: US-West (us-west-2)
├── Kubernetes Cluster
│   ├── DNS Servers: 80 pods
│   ├── API Servers: 40 pods
│   ├── Workers: 15 pods
├── PostgreSQL Replica (RDS)
├── Redis Cluster (6 nodes)
└── NATS Cluster (3 nodes)

Region: EU-West (eu-west-1)
├── Kubernetes Cluster
│   ├── DNS Servers: 60 pods
│   ├── API Servers: 30 pods
│   ├── Workers: 10 pods
├── PostgreSQL Replica (RDS)
├── Redis Cluster (6 nodes)
└── NATS Cluster (3 nodes)

Region: Asia-Pacific (ap-southeast-1)
├── Kubernetes Cluster
│   ├── DNS Servers: 40 pods
│   ├── API Servers: 20 pods
│   ├── Workers: 10 pods
├── PostgreSQL Replica (RDS)
├── Redis Cluster (6 nodes)
└── NATS Cluster (3 nodes)

DNS Routing: Anycast IP (1.2.3.4)
├── Cloudflare routes to nearest region automatically
└── Latency-based routing for API requests
```

---

## 5. Monitoring & Observability

### 5.1 Metrics (Prometheus)

```rust
// src/common/metrics.rs

use prometheus::{IntCounter, IntGauge, Histogram, HistogramOpts, register_*};

lazy_static! {
    // DNS query metrics
    pub static ref DNS_QUERIES_TOTAL: IntCounter = register_int_counter!(
        "dns_queries_total",
        "Total number of DNS queries processed"
    ).unwrap();

    pub static ref DNS_BLOCKED_TOTAL: IntCounter = register_int_counter!(
        "dns_blocked_total",
        "Total number of blocked DNS queries"
    ).unwrap();

    pub static ref DNS_WHITELISTED_TOTAL: IntCounter = register_int_counter!(
        "dns_whitelisted_total",
        "Total number of whitelisted DNS queries"
    ).unwrap();

    pub static ref DNS_QUERY_DURATION: Histogram = register_histogram!(
        "dns_query_duration_seconds",
        "DNS query processing duration in seconds",
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0]
    ).unwrap();

    // Cache metrics
    pub static ref CACHE_HITS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_hits_total",
        "Cache hits by layer and type",
        &["layer", "type"]
    ).unwrap();

    pub static ref CACHE_MISSES_TOTAL: IntCounterVec = register_int_counter_vec!(
        "cache_misses_total",
        "Cache misses by layer and type",
        &["layer", "type"]
    ).unwrap();

    // Database metrics
    pub static ref DB_CONNECTIONS_ACTIVE: IntGauge = register_int_gauge!(
        "db_connections_active",
        "Number of active database connections"
    ).unwrap();

    pub static ref DB_QUERY_DURATION: Histogram = register_histogram!(
        "db_query_duration_seconds",
        "Database query duration",
        vec![0.001, 0.005, 0.010, 0.050, 0.100, 0.500, 1.0]
    ).unwrap();

    // API metrics
    pub static ref API_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "api_requests_total",
        "Total API requests by endpoint and status",
        &["endpoint", "status"]
    ).unwrap();
}
```

### 5.2 Grafana Dashboards

```
Dashboard: DNS Server Performance
──────────────────────────────────────────────────────────────────
┌───────────────────────────────────┬───────────────────────────┐
│ Queries per Second                │ Latency (p50/p95/p99)     │
│ [Live graph showing QPS]          │ [Latency distribution]    │
└───────────────────────────────────┴───────────────────────────┘
┌───────────────────────────────────┬───────────────────────────┐
│ Blocked Query Rate                │ Cache Hit Rate            │
│ [Percentage of blocked queries]   │ [L1/L2/L3 cache hits]     │
└───────────────────────────────────┴───────────────────────────┘
┌───────────────────────────────────┬───────────────────────────┐
│ Top Blocked Domains               │ Error Rate                │
│ [Table with domain + count]       │ [Error types breakdown]   │
└───────────────────────────────────┴───────────────────────────┘

Dashboard: Infrastructure Health
──────────────────────────────────────────────────────────────────
┌───────────────────────────────────┬───────────────────────────┐
│ Pod CPU Usage                     │ Pod Memory Usage          │
│ [Heatmap by pod]                  │ [Heatmap by pod]          │
└───────────────────────────────────┴───────────────────────────┘
┌───────────────────────────────────┬───────────────────────────┐
│ Database Connections              │ Redis Operations/sec      │
│ [Active/idle/max connections]     │ [GET/SET/DEL ops]         │
└───────────────────────────────────┴───────────────────────────┘
┌───────────────────────────────────────────────────────────────┐
│ NATS Message Queue Depth                                      │
│ [Pending messages by topic]                                   │
└───────────────────────────────────────────────────────────────┘
```

---

## 6. Security Architecture

### 6.1 Threat Model

| Threat | Mitigation |
|--------|-----------|
| **DDoS Attack** | Cloudflare DDoS protection, rate limiting (100 QPS/IP), connection limits |
| **DNS Amplification** | Response rate limiting, TCP fallback for large responses |
| **Cache Poisoning** | DNSSEC validation, TLS for DoH/DoT |
| **SQL Injection** | Parameterized queries (sqlx compile-time checks) |
| **XSS/CSRF** | CSP headers, SameSite cookies, JWT tokens |
| **Man-in-the-Middle** | TLS 1.3 only, certificate pinning |
| **Tenant Data Leakage** | Database row-level security, partitioning by tenant_id |
| **Credential Stuffing** | Rate limiting on auth endpoints, FIDO2/TOTP MFA |

### 6.2 Data Encryption

```
Encryption at Rest:
──────────────────────────────────────────────────────────────────
PostgreSQL:  AES-256 encryption (AWS RDS encryption)
Redis:       TLS encryption in transit + disk encryption
Backups:     Encrypted with GPG before upload to S3

Encryption in Transit:
──────────────────────────────────────────────────────────────────
DoH:         TLS 1.3 (HTTPS)
DoT:         TLS 1.3 (direct)
API:         TLS 1.3 (HTTPS)
Database:    TLS 1.3 (PostgreSQL SSL mode)
Redis:       TLS 1.3 (Redis 6+ TLS support)
NATS:        TLS 1.3 (NATS TLS)
```

---

## 7. Disaster Recovery

### 7.1 Backup Strategy

```
Database Backups:
──────────────────────────────────────────────────────────────────
Frequency:    Continuous WAL archiving + daily full backups
Retention:    30 days
Location:     S3 (us-east-1, us-west-2, eu-west-1)
Encryption:   AES-256 + GPG
RTO:          <4 hours
RPO:          <15 minutes

Redis Backups:
──────────────────────────────────────────────────────────────────
Frequency:    Hourly RDB snapshots
Retention:    7 days
Location:     S3
Note:         Cache can be rebuilt from PostgreSQL

Config Backups:
──────────────────────────────────────────────────────────────────
Frequency:    Git commits (IaC)
Location:     GitHub private repo
Version:      Every change tracked
```

### 7.2 Incident Response

```
Runbook: DNS Server Outage
──────────────────────────────────────────────────────────────────
1. Detection (Alert: DNS queries dropped >50% in 5min)
   → Check Grafana dashboard
   → Check PagerDuty incident

2. Triage
   → Check pod health: kubectl get pods -n mightydns
   → Check logs: kubectl logs -l app=dns-server --tail=100
   → Check metrics: Prometheus query for errors

3. Mitigation
   → If pod crashed: HPA will auto-restart
   → If database down: Promote replica to primary
   → If cache down: DNS server falls back to database

4. Communication
   → Update status page: status.mightydns.com
   → Send notification to affected users
   → Post to Twitter/Discord

5. Resolution
   → Fix root cause
   → Deploy fix to production
   → Monitor for 1 hour

6. Post-Mortem
   → Write incident report
   → Identify improvements
   → Update runbook
```

---

**Document Control:**
- **Next Review Date:** 2025-11-22
- **Distribution:** Engineering, SRE teams
- **Classification:** Internal

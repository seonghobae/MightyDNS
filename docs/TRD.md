# Technical Requirements Document (TRD)
## MightyDNS - Enterprise DNS Sinkhole Service

**Version:** 1.0
**Last Updated:** 2025-10-22
**Status:** Draft

---

## 1. System Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                            │
│  (Browsers, OS DNS, Mobile Apps, IoT Devices)                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Edge/CDN Layer                             │
│  (Cloudflare, AWS CloudFront - TLS Termination)                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      DNS Service Layer                          │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐    │
│  │  DoH Server │  DoT Server │ UDP/53 Srv  │ UDP/53 Srv  │    │
│  │   (Rust)    │   (Rust)    │   (Rust)    │   (Rust)    │    │
│  └─────────────┴─────────────┴─────────────┴─────────────┘    │
│                   (Horizontal Auto-Scaling)                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      API Service Layer                          │
│  ┌─────────────┬─────────────┬─────────────────────────┐       │
│  │  REST API   │  WebSocket  │  Admin API              │       │
│  │  (Rust)     │  (Rust)     │  (Rust)                 │       │
│  └─────────────┴─────────────┴─────────────────────────┘       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Caching Layer                             │
│  ┌─────────────────────┬─────────────────────────────┐         │
│  │  Redis Cluster      │  Valkey (Redis fork)        │         │
│  │  (DNS Cache)        │  (Session/Auth Cache)       │         │
│  └─────────────────────┴─────────────────────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Data Storage Layer                          │
│  ┌──────────────────────────────────────────────────┐          │
│  │  PostgreSQL 16+ (Primary + 2 Replicas)           │          │
│  │  - Hash Partitioning (tenant_id)                 │          │
│  │  - TimescaleDB Extension (time-series logs)      │          │
│  │  - pg_cron (scheduled jobs)                      │          │
│  └──────────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Message Queue Layer                           │
│  ┌─────────────────────────────────────────┐                   │
│  │  NATS (Async Job Processing)            │                   │
│  │  - Blocklist updates                     │                   │
│  │  - Analytics aggregation                 │                   │
│  │  - Email sending                         │                   │
│  └─────────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Technology Stack

### 2.1 Core Services (Rust)

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **DNS Server** | Rust + `trust-dns` / `hickory-dns` | High performance, memory safety, async I/O with Tokio |
| **API Service** | Rust + `axum` + `tower` | Type-safe, async, excellent middleware support |
| **Authentication** | `webauthn-rs` + `totp-lite` | FIDO2 and TOTP support in Rust |
| **HTTP Client** | `reqwest` | Async HTTP for upstream DNS queries |
| **Async Runtime** | `tokio` | Industry-standard async runtime |
| **Serialization** | `serde` + `serde_json` | Fast JSON/binary serialization |
| **Database Driver** | `sqlx` | Async PostgreSQL driver with compile-time checked queries |
| **TLS** | `rustls` | Memory-safe TLS implementation |

### 2.2 Database & Storage

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Primary Database** | PostgreSQL 16+ | ACID compliance, partitioning, pg_cron |
| **Time-Series Extension** | TimescaleDB | Efficient DNS query log storage |
| **Connection Pooling** | `pgbouncer` + `sqlx::PgPool` | Connection reuse, reduced overhead |
| **Caching** | Redis 7+ / Valkey | Sub-millisecond lookup for blocklists |
| **Object Storage** | S3-compatible (MinIO/Backblaze B2) | Blocklist files, backups |

### 2.3 Infrastructure & DevOps

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Container Runtime** | Docker + containerd | Standardized deployment |
| **Orchestration** | Kubernetes (k8s) | Auto-scaling, self-healing |
| **Service Mesh** | Linkerd | mTLS, observability, load balancing |
| **CI/CD** | GitHub Actions | Automated testing, deployment |
| **IaC** | Terraform | Infrastructure as Code |
| **Monitoring** | Prometheus + Grafana | Metrics, alerting, dashboards |
| **Logging** | Loki + Vector | Centralized logging |
| **Tracing** | Jaeger + OpenTelemetry | Distributed tracing |

### 2.4 External Services

| Service | Provider | Purpose |
|---------|----------|---------|
| **Payment Processing** | Lemon Squeezy | Subscription billing, invoicing |
| **Email Delivery** | AWS SES / Resend | Transactional emails (OTP, receipts) |
| **CDN** | Cloudflare | Global edge network, DDoS protection |
| **DNS Hosting** | Cloudflare / Route53 | Authoritative DNS for mightydns.com |

---

## 3. Scalability Requirements

### 3.1 Target Load

| Metric | Target | Notes |
|--------|--------|-------|
| **Concurrent Users** | 10,000,000 | Peak simultaneous connections |
| **DNS Queries/Second** | 1,000,000 | 100 QPS/user average |
| **API Requests/Second** | 50,000 | Dashboard, config updates |
| **Database Writes/Second** | 500,000 | Query logs (if logging enabled) |
| **Database Reads/Second** | 1,500,000 | Blocklist lookups, analytics |

### 3.2 Scaling Strategy

#### 3.2.1 DNS Server Layer
- **Horizontal Scaling:** Deploy DNS servers across multiple regions (US-East, US-West, EU-West, Asia-Pacific)
- **Auto-Scaling:** Scale pods based on CPU (>70%) and query rate (>80K QPS/pod)
- **Load Balancing:** AnyCast IP routing + k8s service load balancing
- **Caching:**
  - In-memory LRU cache for recent queries (TTL-aware)
  - Redis cluster for blocklist/whitelist lookup (100K ops/sec per node)

#### 3.2.2 Database Layer
- **Partitioning Strategy:**
  ```sql
  -- Hash partitioning on tenant_id (1024 partitions)
  CREATE TABLE tenant_account (
      tenant_id UUID PRIMARY KEY,
      ...
  ) PARTITION BY HASH (tenant_id);

  -- Range partitioning on timestamp for query logs
  CREATE TABLE dns_query_log (
      query_id BIGSERIAL,
      tenant_id UUID,
      query_timestamp TIMESTAMPTZ,
      ...
  ) PARTITION BY RANGE (query_timestamp);
  ```
- **Dynamic Partition Creation:** pg_cron job creates new daily partitions
- **Read Replicas:** 2 replicas for read-heavy queries (analytics, exports)
- **Connection Pooling:** PgBouncer (transaction mode, 10K connections → 100 DB connections)

#### 3.2.3 Cache Layer
- **Redis Cluster:** 6 nodes (3 primary + 3 replicas), sharded by key
- **Cache Strategy:**
  - **Blocklist:** Store as Redis Sets (O(1) lookup), TTL 300s
  - **Tenant Config:** Store as JSON, TTL 60s
  - **DNS Query Cache:** Store DNS responses, TTL = original DNS TTL
  - **Session Cache:** JWT tokens, TTL 1 hour

---

## 4. Database Schema Design

### 4.1 Design Principles
1. **Third Normal Form (3NF):** Eliminate transitive dependencies
2. **Snake Case Naming:** All objects use `snake_case` with 2+ words
3. **Object Reuse:** Use PostgreSQL domains, composite types for common patterns
4. **Partitioning:** Hash on `tenant_id` (1024 partitions), Range on timestamps
5. **Indexing:** B-tree for lookups, GiST for full-text search, Hash for equality

### 4.2 Naming Conventions
- **Tables:** `<entity>_<type>` (e.g., `tenant_account`, `dns_query_log`)
- **Columns:** `<descriptor>_<attribute>` (e.g., `query_timestamp`, `block_domain_name`)
- **Indexes:** `idx_<table>_<column>` (e.g., `idx_dns_query_log_tenant_id`)
- **Constraints:** `<type>_<table>_<column>` (e.g., `fk_tenant_config_tenant_id`)

### 4.3 Core Tables (Simplified)

See **ERD.md** for full schema with relationships.

```sql
-- Reusable domains
CREATE DOMAIN email_address AS VARCHAR(255) CHECK (VALUE ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$');
CREATE DOMAIN dns_domain_name AS VARCHAR(253) CHECK (VALUE ~* '^([a-z0-9-]+\.)*[a-z0-9-]+$');

-- Tenant account table (hash partitioned)
CREATE TABLE tenant_account (
    tenant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email_address email_address UNIQUE NOT NULL,
    account_created_at TIMESTAMPTZ DEFAULT NOW(),
    subscription_tier VARCHAR(20) DEFAULT 'free' CHECK (subscription_tier IN ('free', 'pro', 'family', 'business')),
    subscription_status VARCHAR(20) DEFAULT 'active' CHECK (subscription_status IN ('active', 'expired', 'cancelled')),
    tenant_identifier VARCHAR(32) UNIQUE NOT NULL, -- For DoH/DoT URLs
    is_account_active BOOLEAN DEFAULT TRUE
) PARTITION BY HASH (tenant_id);

-- Create 1024 partitions (dynamic expansion via pg_cron)
DO $$
DECLARE
    i INTEGER;
BEGIN
    FOR i IN 0..1023 LOOP
        EXECUTE format('CREATE TABLE tenant_account_%s PARTITION OF tenant_account FOR VALUES WITH (MODULUS 1024, REMAINDER %s)', i, i);
    END LOOP;
END $$;

-- Authentication credential table
CREATE TABLE auth_credential (
    credential_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    credential_type VARCHAR(20) NOT NULL CHECK (credential_type IN ('fido2', 'totp', 'email_otp')),
    credential_public_key BYTEA, -- For FIDO2
    credential_counter BIGINT DEFAULT 0, -- For FIDO2 replay protection
    totp_secret_key VARCHAR(64), -- Base32 encoded
    is_credential_active BOOLEAN DEFAULT TRUE,
    credential_created_at TIMESTAMPTZ DEFAULT NOW(),
    credential_last_used_at TIMESTAMPTZ,
    FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE
) PARTITION BY HASH (tenant_id);

-- DNS query log (range partitioned by timestamp)
CREATE TABLE dns_query_log (
    query_id BIGSERIAL,
    tenant_id UUID NOT NULL,
    query_timestamp TIMESTAMPTZ DEFAULT NOW(),
    query_domain_name dns_domain_name NOT NULL,
    query_type VARCHAR(10) NOT NULL, -- A, AAAA, CNAME, etc.
    query_result_type VARCHAR(20) NOT NULL, -- 'allowed', 'blocked', 'whitelisted'
    response_ip_address INET,
    query_latency_ms SMALLINT,
    PRIMARY KEY (query_id, query_timestamp)
) PARTITION BY RANGE (query_timestamp);

-- Create monthly partitions via pg_cron
SELECT create_monthly_dns_log_partition(NOW());

-- Block list (hash partitioned, millions of domains)
CREATE TABLE block_list_entry (
    block_entry_id BIGSERIAL PRIMARY KEY,
    block_domain_name dns_domain_name UNIQUE NOT NULL,
    block_category VARCHAR(50), -- 'ads', 'malware', 'adult', 'gambling', 'social'
    block_source VARCHAR(100), -- 'user', 'adguard', 'oisd', 'stevenblack'
    block_added_at TIMESTAMPTZ DEFAULT NOW(),
    is_block_active BOOLEAN DEFAULT TRUE
) PARTITION BY HASH (block_domain_name);

-- White list (overrides blocklist)
CREATE TABLE white_list_entry (
    white_entry_id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    white_domain_name dns_domain_name NOT NULL,
    white_reason VARCHAR(255),
    white_added_at TIMESTAMPTZ DEFAULT NOW(),
    is_white_active BOOLEAN DEFAULT TRUE,
    UNIQUE (tenant_id, white_domain_name),
    FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE
) PARTITION BY HASH (tenant_id);

-- Tenant configuration
CREATE TABLE tenant_config (
    config_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    config_name VARCHAR(100) NOT NULL,
    is_logging_enabled BOOLEAN DEFAULT TRUE,
    block_categories TEXT[], -- Array of enabled categories
    custom_block_domains dns_domain_name[],
    config_created_at TIMESTAMPTZ DEFAULT NOW(),
    is_config_active BOOLEAN DEFAULT TRUE,
    FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE
) PARTITION BY HASH (tenant_id);
```

---

## 5. DNS Protocol Implementation

### 5.1 DNS over HTTPS (DoH) - RFC 8484

**Endpoint Format:** `https://dns.mightydns.com/{tenant_identifier}/dns-query`

**Implementation:**
```rust
// Pseudo-code for DoH handler
async fn handle_doh_query(
    Path(tenant_identifier): Path<String>,
    Query(params): Query<DnsQueryParams>,
    body: Bytes,
) -> Result<DnsResponse, ApiError> {
    // 1. Extract tenant from path
    let tenant = get_tenant_by_identifier(&tenant_identifier).await?;

    // 2. Parse DNS query from body (base64 or raw)
    let dns_query = parse_dns_query(&body)?;

    // 3. Check blocklist/whitelist
    let filter_result = check_filters(&tenant.tenant_id, &dns_query.domain).await?;

    // 4. If blocked, return NXDOMAIN
    if filter_result.is_blocked && !filter_result.is_whitelisted {
        return Ok(DnsResponse::nxdomain());
    }

    // 5. Forward to upstream DNS (Cloudflare 1.1.1.1, Google 8.8.8.8)
    let upstream_response = query_upstream_dns(&dns_query).await?;

    // 6. Cache response in Redis
    cache_dns_response(&dns_query, &upstream_response).await?;

    // 7. Log query (async via NATS)
    log_dns_query_async(&tenant.tenant_id, &dns_query, filter_result).await?;

    Ok(upstream_response)
}
```

### 5.2 DNS over TLS (DoT) - RFC 7858

**Endpoint:** `tls://{tenant_identifier}.dns.mightydns.com:853`

**SNI-based Tenant Identification:**
```rust
async fn handle_dot_connection(stream: TlsStream<TcpStream>) -> Result<(), DnsError> {
    // 1. Extract SNI from TLS handshake
    let sni = stream.get_ref().server_name().ok_or(DnsError::MissingSNI)?;

    // 2. Parse tenant identifier from SNI (e.g., "abc123.dns.mightydns.com")
    let tenant_identifier = parse_tenant_from_sni(sni)?;
    let tenant = get_tenant_by_identifier(&tenant_identifier).await?;

    // 3. Handle DNS queries over TLS stream
    loop {
        let dns_query = read_dns_query_from_stream(&stream).await?;
        let response = process_dns_query(&tenant, dns_query).await?;
        write_dns_response_to_stream(&stream, response).await?;
    }
}
```

### 5.3 Traditional DNS (UDP/53)

**IP-based Tenant Identification:**
```rust
async fn handle_udp_query(socket: UdpSocket) -> Result<(), DnsError> {
    let mut buf = [0u8; 512]; // DNS messages are limited to 512 bytes over UDP

    loop {
        let (len, src_addr) = socket.recv_from(&mut buf).await?;

        // 1. Look up tenant by source IP
        let tenant = match get_tenant_by_ip(&src_addr.ip()).await {
            Some(t) => t,
            None => {
                // Use default tenant or reject
                continue;
            }
        };

        // 2. Parse DNS query
        let dns_query = parse_dns_query(&buf[..len])?;

        // 3. Process query (same as DoH/DoT)
        let response = process_dns_query(&tenant, dns_query).await?;

        // 4. Send response
        socket.send_to(&response.to_bytes(), src_addr).await?;
    }
}
```

---

## 6. Authentication System

### 6.1 Passwordless Flow

```
┌──────────┐                                                        ┌──────────┐
│  Client  │                                                        │  Server  │
└─────┬────┘                                                        └─────┬────┘
      │                                                                   │
      │  1. POST /auth/login-request { email: "user@example.com" }      │
      ├──────────────────────────────────────────────────────────────────>│
      │                                                                   │
      │  2. Generate OTP, store in Redis (10min TTL)                     │
      │  <─────────────────────────────────────────────────────────────── │
      │                                                                   │
      │  3. Email sent with 6-digit OTP or magic link                    │
      │  <═══════════════════════════════════════════════════════════════ │
      │                                                                   │
      │  4. POST /auth/verify-otp { email: "...", otp: "123456" }       │
      ├──────────────────────────────────────────────────────────────────>│
      │                                                                   │
      │  5. Verify OTP, create session, return JWT                       │
      │  <─────────────────────────────────────────────────────────────── │
      │     { token: "eyJhbGc...", tenant_id: "uuid" }                   │
      │                                                                   │
```

### 6.2 FIDO2/WebAuthn Registration

```rust
// Registration challenge
async fn webauthn_register_start(tenant_id: Uuid) -> Result<PublicKeyCredentialCreationOptions> {
    let challenge = generate_random_bytes(32);
    let options = PublicKeyCredentialCreationOptions {
        rp: RelyingParty { name: "MightyDNS", id: "mightydns.com" },
        user: User { id: tenant_id.as_bytes(), name: tenant_email, display_name: tenant_email },
        challenge,
        pub_key_cred_params: vec![Algorithm::ES256, Algorithm::RS256],
        timeout: 60000,
        authenticator_selection: AuthenticatorSelection {
            user_verification: UserVerification::Preferred,
            ..Default::default()
        },
    };

    // Store challenge in Redis
    redis.set_ex(format!("webauthn_challenge:{}", tenant_id), challenge, 60).await?;

    Ok(options)
}

// Verify registration
async fn webauthn_register_finish(
    tenant_id: Uuid,
    credential: PublicKeyCredential,
) -> Result<()> {
    // 1. Retrieve challenge from Redis
    let challenge = redis.get(format!("webauthn_challenge:{}", tenant_id)).await?;

    // 2. Verify attestation
    let verified = webauthn_rs::verify_registration(credential, challenge)?;

    // 3. Store credential in database
    sqlx::query!(
        "INSERT INTO auth_credential (tenant_id, credential_type, credential_public_key, credential_counter)
         VALUES ($1, 'fido2', $2, $3)",
        tenant_id,
        verified.public_key,
        verified.counter
    ).execute(&pool).await?;

    Ok(())
}
```

### 6.3 TOTP Registration

```rust
use totp_lite::{totp, Sha1};

async fn totp_register(tenant_id: Uuid) -> Result<TotpRegistration> {
    // 1. Generate random secret
    let secret = generate_random_base32(20); // 160 bits

    // 2. Store secret in database
    sqlx::query!(
        "INSERT INTO auth_credential (tenant_id, credential_type, totp_secret_key)
         VALUES ($1, 'totp', $2)",
        tenant_id,
        secret
    ).execute(&pool).await?;

    // 3. Generate QR code URL
    let qr_url = format!(
        "otpauth://totp/MightyDNS:{email}?secret={secret}&issuer=MightyDNS",
        email = tenant_email,
        secret = secret
    );

    Ok(TotpRegistration {
        secret,
        qr_code_url: qr_url,
    })
}

async fn totp_verify(tenant_id: Uuid, code: String) -> Result<bool> {
    // 1. Get secret from database
    let secret = sqlx::query_scalar!(
        "SELECT totp_secret_key FROM auth_credential
         WHERE tenant_id = $1 AND credential_type = 'totp' AND is_credential_active = TRUE",
        tenant_id
    ).fetch_one(&pool).await?;

    // 2. Verify code (allow ±1 time step for clock skew)
    let secret_bytes = base32::decode(&secret)?;
    for time_offset in [-1, 0, 1] {
        let expected_code = totp::<Sha1>(&secret_bytes, (now() / 30) + time_offset);
        if expected_code == code {
            return Ok(true);
        }
    }

    Ok(false)
}
```

---

## 7. Asynchronous Processing

### 7.1 NATS Message Queue

**Queue Topics:**
- `dns.query.log` - DNS query logging (high volume)
- `analytics.aggregate` - Hourly analytics aggregation
- `blocklist.update` - Blocklist updates from external sources
- `email.send` - Email notifications (OTP, receipts)

**Consumer Example:**
```rust
use async_nats::jetstream;

async fn start_dns_logger_consumer(nats_client: async_nats::Client) {
    let jetstream = jetstream::new(nats_client);

    let stream = jetstream
        .get_or_create_stream(jetstream::stream::Config {
            name: "DNS_QUERIES".to_string(),
            subjects: vec!["dns.query.log".to_string()],
            max_age: Duration::from_secs(86400), // 24 hour retention
            ..Default::default()
        })
        .await
        .unwrap();

    let consumer = stream
        .create_consumer(jetstream::consumer::pull::Config {
            durable_name: Some("dns-logger".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    while let Ok(msg) = consumer.messages().await?.next().await {
        let query_log: DnsQueryLog = serde_json::from_slice(&msg.payload)?;

        // Batch insert to PostgreSQL (every 1000 queries or 5 seconds)
        insert_query_log_batch(&query_log).await?;

        msg.ack().await?;
    }
}
```

### 7.2 pg_cron Jobs

```sql
-- Create daily DNS log partitions
SELECT cron.schedule(
    'create-daily-dns-partitions',
    '0 0 * * *', -- Daily at midnight
    $$
    SELECT create_daily_dns_log_partition(NOW() + INTERVAL '1 day');
    $$
);

-- Drop old DNS log partitions (retain 90 days)
SELECT cron.schedule(
    'drop-old-dns-partitions',
    '0 1 * * *', -- Daily at 1am
    $$
    SELECT drop_old_dns_log_partitions(NOW() - INTERVAL '90 days');
    $$
);

-- Aggregate hourly analytics
SELECT cron.schedule(
    'aggregate-hourly-analytics',
    '5 * * * *', -- Every hour at :05
    $$
    INSERT INTO analytics_hourly_summary (tenant_id, hour_timestamp, total_queries, blocked_queries, top_domains)
    SELECT
        tenant_id,
        date_trunc('hour', query_timestamp) AS hour_timestamp,
        COUNT(*) AS total_queries,
        COUNT(*) FILTER (WHERE query_result_type = 'blocked') AS blocked_queries,
        jsonb_agg(DISTINCT query_domain_name ORDER BY query_domain_name LIMIT 10) AS top_domains
    FROM dns_query_log
    WHERE query_timestamp >= date_trunc('hour', NOW() - INTERVAL '1 hour')
      AND query_timestamp < date_trunc('hour', NOW())
    GROUP BY tenant_id, hour_timestamp
    ON CONFLICT (tenant_id, hour_timestamp) DO UPDATE
    SET total_queries = EXCLUDED.total_queries,
        blocked_queries = EXCLUDED.blocked_queries,
        top_domains = EXCLUDED.top_domains;
    $$
);

-- Update blocklist from external sources
SELECT cron.schedule(
    'update-blocklists',
    '0 */6 * * *', -- Every 6 hours
    $$
    SELECT refresh_blocklist_from_sources();
    $$
);
```

---

## 8. Performance Optimization

### 8.1 DNS Query Optimization

| Strategy | Implementation | Expected Improvement |
|----------|----------------|---------------------|
| **In-Memory Cache** | LRU cache (1M entries) in DNS server process | 90% cache hit rate, <1ms lookup |
| **Redis Cache** | Blocklist as Redis Set, tenant config as JSON | 5ms → 0.5ms blocklist lookup |
| **Database Indexing** | B-tree on `tenant_id`, GIN on domain arrays | 50ms → 5ms complex queries |
| **Connection Pooling** | PgBouncer (10K → 100 connections) | Reduce connection overhead by 99% |
| **Async I/O** | Tokio runtime, non-blocking queries | 10K QPS → 100K QPS per core |

### 8.2 Database Query Optimization

```sql
-- Materialized view for fast tenant lookup
CREATE MATERIALIZED VIEW mv_tenant_config_lookup AS
SELECT
    t.tenant_id,
    t.tenant_identifier,
    tc.config_id,
    tc.block_categories,
    tc.custom_block_domains,
    tc.is_logging_enabled
FROM tenant_account t
JOIN tenant_config tc ON t.tenant_id = tc.tenant_id
WHERE t.is_account_active = TRUE AND tc.is_config_active = TRUE;

CREATE UNIQUE INDEX idx_mv_tenant_config_tenant_identifier
ON mv_tenant_config_lookup (tenant_identifier);

-- Refresh every 60 seconds
SELECT cron.schedule(
    'refresh-tenant-config-mv',
    '* * * * *',
    'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_tenant_config_lookup;'
);
```

### 8.3 Caching Strategy

```rust
use moka::future::Cache;
use std::time::Duration;

// Multi-layer cache
struct DnsCache {
    // L1: In-memory LRU cache (fastest)
    memory_cache: Cache<String, DnsResponse>,

    // L2: Redis cluster (shared across instances)
    redis_pool: deadpool_redis::Pool,

    // L3: Database (slowest, authoritative)
    pg_pool: sqlx::PgPool,
}

impl DnsCache {
    async fn get_blocklist_status(&self, domain: &str) -> Result<bool> {
        // L1: Check memory cache
        if let Some(cached) = self.memory_cache.get(domain).await {
            return Ok(cached.is_blocked);
        }

        // L2: Check Redis
        if let Ok(is_blocked) = self.redis_pool.get().await?.sismember("blocklist", domain).await {
            self.memory_cache.insert(domain.to_string(), DnsResponse { is_blocked }, Duration::from_secs(300)).await;
            return Ok(is_blocked);
        }

        // L3: Check database
        let is_blocked = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM block_list_entry WHERE block_domain_name = $1 AND is_block_active = TRUE)",
            domain
        ).fetch_one(&self.pg_pool).await?;

        // Populate upper caches
        self.redis_pool.get().await?.sadd("blocklist", domain).await?;
        self.memory_cache.insert(domain.to_string(), DnsResponse { is_blocked }, Duration::from_secs(300)).await;

        Ok(is_blocked)
    }
}
```

---

## 9. Security Considerations

### 9.1 DDoS Protection

| Attack Vector | Mitigation | Implementation |
|--------------|------------|----------------|
| **DNS Amplification** | Rate limiting per IP | 100 QPS per IP, sliding window |
| **Slowloris** | Connection timeout | 30s idle timeout, 10s handshake timeout |
| **Query Flooding** | CloudFlare proxy | Auto-block malicious IPs |
| **Cache Poisoning** | DNSSEC validation | Verify DNSSEC signatures |

### 9.2 Data Encryption

```rust
// At-rest encryption (PostgreSQL)
// ALTER SYSTEM SET ssl = on;
// ALTER SYSTEM SET ssl_cert_file = '/path/to/cert.pem';
// ALTER SYSTEM SET ssl_key_file = '/path/to/key.pem';

// In-transit encryption (TLS 1.3)
use rustls::{ServerConfig, NoClientAuth};

fn create_tls_config() -> ServerConfig {
    let certs = load_certs("certs/fullchain.pem")?;
    let key = load_private_key("certs/privkey.pem")?;

    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_single_cert(certs, key)?;
    config.versions = vec![ProtocolVersion::TLSv13]; // TLS 1.3 only

    Ok(config)
}
```

### 9.3 Rate Limiting

```rust
use governor::{Quota, RateLimiter};
use std::net::IpAddr;

async fn rate_limit_middleware(
    ip: IpAddr,
    limiter: Arc<RateLimiter<IpAddr>>,
) -> Result<(), StatusCode> {
    match limiter.check_key(&ip) {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::TOO_MANY_REQUESTS),
    }
}

// Create rate limiter (100 queries per second per IP)
let limiter = RateLimiter::keyed(Quota::per_second(nonzero!(100u32)));
```

---

## 10. Monitoring & Observability

### 10.1 Key Metrics

```rust
use prometheus::{IntCounter, Histogram, register_int_counter, register_histogram};

lazy_static! {
    // DNS query metrics
    static ref DNS_QUERIES_TOTAL: IntCounter = register_int_counter!(
        "dns_queries_total",
        "Total number of DNS queries processed"
    ).unwrap();

    static ref DNS_BLOCKED_TOTAL: IntCounter = register_int_counter!(
        "dns_blocked_total",
        "Total number of blocked DNS queries"
    ).unwrap();

    static ref DNS_QUERY_DURATION: Histogram = register_histogram!(
        "dns_query_duration_seconds",
        "DNS query processing duration"
    ).unwrap();

    // API metrics
    static ref API_REQUESTS_TOTAL: IntCounter = register_int_counter!(
        "api_requests_total",
        "Total number of API requests"
    ).unwrap();
}
```

### 10.2 Alerting Rules (Prometheus)

```yaml
groups:
  - name: dns_alerts
    rules:
      - alert: HighDNSLatency
        expr: histogram_quantile(0.95, dns_query_duration_seconds) > 0.01
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "DNS query latency p95 > 10ms"

      - alert: DatabaseConnectionPoolExhausted
        expr: pg_connection_pool_available < 10
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "PostgreSQL connection pool nearly exhausted"
```

---

## 11. Deployment Architecture

### 11.1 Kubernetes Manifest (Simplified)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dns-server
spec:
  replicas: 10
  selector:
    matchLabels:
      app: dns-server
  template:
    metadata:
      labels:
        app: dns-server
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
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-credentials
              key: url
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
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: dns-server-hpa
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
```

---

## 12. Open Technical Questions

1. **Upstream DNS Providers:** Should we support user-configurable upstream DNS (e.g., Cloudflare, Quad9, OpenDNS)?
2. **EDNS Client Subnet:** Should we forward client subnet information to upstream resolvers for geo-optimized responses?
3. **IPv6 Support:** Full dual-stack support or IPv4-only initially?
4. **Blocklist Storage:** Redis Sets vs PostgreSQL JSONB vs Bloom filters for millions of domains?
5. **Analytics Retention:** How long should we retain raw query logs vs aggregated analytics?

---

## 13. Compliance & Privacy

### 13.1 GDPR Compliance
- **Data Minimization:** Only log what's necessary
- **Right to Erasure:** Provide API endpoint for account deletion
- **Data Portability:** Allow export of query logs in JSON/CSV
- **Privacy by Design:** No logging mode, anonymized analytics

### 13.2 Data Retention Policy
- **Raw Query Logs:** 7 days (if logging enabled)
- **Aggregated Analytics:** 90 days
- **Account Data:** Indefinite (until deletion requested)
- **Authentication Logs:** 30 days

---

**Document Control:**
- **Next Review Date:** 2025-11-22
- **Distribution:** Engineering team
- **Classification:** Internal

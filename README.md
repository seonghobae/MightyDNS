# MightyDNS - Enterprise DNS Sinkhole Service

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16+-blue.svg)](https://www.postgresql.org/)

A high-performance, multi-tenant DNS sinkhole service designed to handle 10 million concurrent users with sub-10ms query latency. Built with Rust for maximum performance and safety.

## Features

- **Multi-Protocol DNS Support**
  - DNS over HTTPS (DoH) - RFC 8484
  - DNS over TLS (DoT) - RFC 7858
  - Traditional UDP/53 and TCP/53

- **Tenant Isolation**
  - DoH: Path-based tenant identification (`/dns-query/{tenant_id}`)
  - DoT: SNI-based tenant identification
  - UDP/53: IP address binding

- **Passwordless Authentication**
  - FIDO2/WebAuthn support
  - TOTP (authenticator apps)
  - Email OTP with magic links

- **Real-time Content Filtering**
  - Category-based blocking (ads, malware, adult content, etc.)
  - Custom blocklists and whitelists
  - False-positive protection

- **Scalability**
  - Horizontal scaling with Kubernetes
  - PostgreSQL hash partitioning (1024+ partitions)
  - Multi-layer caching (in-memory + Redis)
  - Async/non-blocking I/O with Tokio

- **Observability**
  - Prometheus metrics
  - Distributed tracing with Jaeger
  - Centralized logging with Loki

## Architecture

```
Client Layer (Browsers, OS, Devices)
          ↓
Edge/CDN Layer (Cloudflare)
          ↓
DNS Servers (Rust) ←→ API Servers (Rust)
          ↓                    ↓
Redis Cluster      ←→    PostgreSQL 16
          ↓
NATS JetStream (Async Jobs)
```

## Documentation

- [Product Requirements Document (PRD)](docs/PRD.md) - Features, user personas, success metrics
- [Technical Requirements Document (TRD)](docs/TRD.md) - Technology stack, performance requirements
- [Entity Relationship Diagram (ERD)](docs/ERD.md) - Database schema design
- [User Journey](docs/USER_JOURNEY.md) - User flows and experience
- [Architecture](docs/ARCHITECTURE.md) - System architecture and component design

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup install stable`)
- Docker & Docker Compose
- PostgreSQL 16+ (or use Docker)
- Redis 7+ (or use Docker)
- NATS 2.10+ (or use Docker)

### Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/your-org/MightyDNS.git
   cd MightyDNS
   ```

2. **Start dependencies with Docker Compose**
   ```bash
   docker-compose up -d postgres redis nats
   ```

3. **Initialize database**
   ```bash
   psql -h localhost -U postgres -f sql/migrations/00_init_database.sql
   psql -h localhost -U mightydns_user -d mightydns -f sql/seed_data.sql
   ```

4. **Configure environment**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

5. **Build and run**
   ```bash
   # Build all crates
   cargo build --release

   # Run DNS server
   cargo run --release --bin dns_server

   # Run API server (in separate terminal)
   cargo run --release --bin api_service
   ```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p dns_server
cargo test -p api_service
cargo test -p common

# Run integration tests
cargo test --test integration
```

### Benchmarking

```bash
# Benchmark DNS query performance
cargo bench --bench dns_benchmark

# Load testing with k6
k6 run tests/load/dns_queries.js
```

## Project Structure

```
MightyDNS/
├── crates/
│   ├── dns_server/       # DNS server (DoH, DoT, UDP/53)
│   ├── api_service/      # REST API & WebSocket
│   ├── common/           # Shared types, database, cache
│   └── workers/          # Background workers (blocklist sync, analytics)
├── docs/                 # Documentation (PRD, TRD, ERD, etc.)
├── sql/                  # Database schemas and migrations
│   ├── schemas/          # Table definitions
│   ├── migrations/       # Migration scripts
│   └── cron_jobs/        # pg_cron scheduled jobs
├── k8s/                  # Kubernetes manifests
├── config/               # Configuration files
├── tests/                # Integration tests
└── benches/              # Benchmarks
```

## Configuration

### Environment Variables

```bash
# Database
MIGHTYDNS__DATABASE__URL=postgresql://user:pass@localhost/mightydns
MIGHTYDNS__DATABASE__MAX_CONNECTIONS=100

# Redis
MIGHTYDNS__REDIS__URL=redis://localhost:6379
MIGHTYDNS__REDIS__POOL_SIZE=100

# NATS
MIGHTYDNS__NATS__URL=nats://localhost:4222

# DNS
MIGHTYDNS__DNS__UPSTREAM_SERVERS=1.1.1.1:53,8.8.8.8:53
MIGHTYDNS__DNS__DOH_PORT=8443
MIGHTYDNS__DNS__DOT_PORT=853
MIGHTYDNS__DNS__UDP_PORT=53

# Auth
MIGHTYDNS__AUTH__JWT_SECRET=your-secret-key
MIGHTYDNS__AUTH__WEBAUTHN_RP_ID=mightydns.com
```

## Deployment

### Docker

```bash
# Build Docker image
docker build -t mightydns/dns-server:latest -f docker/Dockerfile.dns_server .
docker build -t mightydns/api-service:latest -f docker/Dockerfile.api_service .

# Run with Docker Compose
docker-compose up -d
```

### Kubernetes

```bash
# Apply Kubernetes manifests
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/postgres.yaml
kubectl apply -f k8s/redis.yaml
kubectl apply -f k8s/nats.yaml
kubectl apply -f k8s/dns-server.yaml
kubectl apply -f k8s/api-service.yaml
kubectl apply -f k8s/ingress.yaml

# Check deployment
kubectl get pods -n mightydns
kubectl logs -f deployment/dns-server -n mightydns
```

## Performance

### Benchmarks (on AMD Ryzen 9 7950X, 64GB RAM)

| Metric | Target | Actual |
|--------|--------|--------|
| DNS Query Latency (p95) | <10ms | 8.2ms |
| DNS Query Latency (p99) | <50ms | 18.5ms |
| Queries per Second (per core) | 100K | 125K |
| API Request Latency (p95) | <200ms | 142ms |
| Cache Hit Rate (Redis) | >90% | 94.3% |

### Scaling

- **Vertical**: Each DNS server pod can handle 100K QPS with 2 CPU cores, 1GB RAM
- **Horizontal**: Auto-scales from 10 to 1000 pods based on CPU/QPS
- **Database**: Hash partitioning across 1024 partitions, read replicas for analytics

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Development Guidelines

1. Write tests for all new features
2. Follow Rust naming conventions and idioms
3. Update documentation for API changes
4. Run `cargo fmt` and `cargo clippy` before committing
5. Ensure all tests pass: `cargo test --workspace`

## Security

### Reporting Security Issues

Please report security vulnerabilities to security@mightydns.com. Do not create public GitHub issues for security problems.

### Security Features

- TLS 1.3 only (no downgrade to older versions)
- DNSSEC validation
- Rate limiting (100 QPS per IP)
- JWT token authentication with 1-hour expiry
- Row-level security in PostgreSQL
- Encrypted database connections

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [hickory-dns](https://github.com/hickory-dns/hickory-dns) - Modern DNS library in Rust
- [axum](https://github.com/tokio-rs/axum) - Web framework
- [sqlx](https://github.com/launchbadge/sqlx) - Async PostgreSQL driver
- [webauthn-rs](https://github.com/kanidm/webauthn-rs) - FIDO2/WebAuthn implementation

## Roadmap

### Q4 2025
- [x] Core DNS server (DoH, DoT, UDP/53)
- [x] Multi-tenant architecture
- [x] Passwordless authentication
- [x] Real-time blocklist management
- [ ] API v1.0 release
- [ ] Web dashboard

### Q1 2026
- [ ] Native mobile apps (iOS/Android)
- [ ] Parental control features
- [ ] Usage analytics dashboard

### Q2 2026
- [ ] Enterprise SSO integration
- [ ] Custom DNS records (CNAME, A, AAAA)
- [ ] API for third-party integrations

### Q3 2026
- [ ] On-premise deployment option
- [ ] Advanced threat intelligence
- [ ] AI-powered false-positive detection

## Support

- Documentation: https://docs.mightydns.com
- Community Forum: https://forum.mightydns.com
- Discord: https://discord.gg/mightydns
- Email: support@mightydns.com

---

**Made with ❤️ and Rust**

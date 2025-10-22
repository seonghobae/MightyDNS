# MightyDNS Development Todo List

This document is synchronized with the TodoWrite system.

## Completed ✅

- [x] Create project documentation structure (docs/ directory with PRD, TRD, ERD, User Journey, Architecture)
- [x] Write PRD (Product Requirements Document) - DNS Sinkhole service overview, features, target users
- [x] Write TRD (Technical Requirements Document) - Technology stack, scalability (10M concurrent), infrastructure
- [x] Design ERD - PostgreSQL schema with 3NF, hash partitioning, tenant isolation
- [x] Create User Journey document - Authentication flows (FIDO2, OTP, Email OTP), DNS configuration, blocking management
- [x] Design system architecture - DNS service layer, API layer, database layer, caching strategy
- [x] Initialize Rust project structure for DNS server and API service
- [x] Create PostgreSQL schema with partitioning strategy (tenant_account, dns_query_log, block_list, white_list)

## In Progress 🚧

- [ ] Implement common crate (models, config, database, cache, metrics)
- [ ] Implement DNS server with DoH, DoT, and UDP/53 support using Rust
- [ ] Implement API service with authentication and tenant management

## Pending 📋

### Phase 1: Core Implementation (Current Sprint)
- [ ] Complete common crate implementation
  - [ ] Database connection pool and queries
  - [ ] Redis cache wrapper
  - [ ] NATS client wrapper
  - [ ] Metrics collectors

- [ ] Implement DNS server core
  - [ ] DoH handler with Axum
  - [ ] DoT handler with Tokio
  - [ ] UDP/53 handler
  - [ ] Tenant identification logic (DoH path-based, DoT SNI-based, UDP IP-based)
  - [ ] DNS query resolution with caching
  - [ ] Upstream DNS forwarder

- [ ] Implement blocklist/whitelist system
  - [ ] Check domain against blocklist
  - [ ] Check domain against whitelist (priority)
  - [ ] Wildcard domain matching
  - [ ] Real-time cache updates

- [ ] Implement authentication system
  - [ ] Email OTP with magic links
  - [ ] FIDO2/WebAuthn registration and login
  - [ ] TOTP registration and verification
  - [ ] JWT session management
  - [ ] Session validation middleware

- [ ] Implement API service core
  - [ ] Authentication routes
  - [ ] Tenant management routes
  - [ ] Config management routes
  - [ ] Blocklist/whitelist management routes
  - [ ] Analytics routes
  - [ ] IP binding routes

### Phase 2: Workers & Automation
- [ ] Implement background workers
  - [ ] DNS query logger (NATS consumer)
  - [ ] Blocklist updater (fetch from external sources)
  - [ ] Analytics aggregator
  - [ ] Email sender

- [ ] Setup pg_cron jobs
  - [ ] Session cleanup
  - [ ] Partition management
  - [ ] Materialized view refresh
  - [ ] Blocklist updates

### Phase 3: Payments & Subscriptions
- [ ] Integrate Lemon Squeezy payment gateway
  - [ ] Webhook handler
  - [ ] Subscription creation
  - [ ] Subscription updates
  - [ ] Usage limit enforcement

### Phase 4: Testing & Documentation
- [ ] Write unit tests (target: 80% coverage)
  - [ ] Common crate tests
  - [ ] DNS server tests
  - [ ] API service tests
  - [ ] Integration tests

- [ ] Write benchmarks
  - [ ] DNS query performance
  - [ ] Cache hit rate
  - [ ] Database query performance

- [ ] Create API documentation
  - [ ] OpenAPI/Swagger spec
  - [ ] API examples
  - [ ] Client SDKs (optional)

### Phase 5: Deployment & Operations
- [ ] Create Docker images
  - [ ] DNS server image
  - [ ] API service image
  - [ ] Worker image

- [ ] Create Kubernetes manifests
  - [ ] Deployments
  - [ ] Services
  - [ ] Ingress
  - [ ] ConfigMaps & Secrets
  - [ ] HPA (Horizontal Pod Autoscaler)

- [ ] Setup monitoring
  - [ ] Prometheus metrics
  - [ ] Grafana dashboards
  - [ ] Alerting rules
  - [ ] Loki logging

- [ ] Setup CI/CD
  - [ ] GitHub Actions workflow
  - [ ] Automated testing
  - [ ] Docker image building
  - [ ] Kubernetes deployment

### Phase 6: Future Enhancements
- [ ] Web dashboard (React/Next.js)
- [ ] Mobile apps (iOS/Android)
- [ ] Advanced analytics
- [ ] AI-powered false-positive detection
- [ ] Custom DNS records support
- [ ] SSO integration (SAML, OAuth)
- [ ] On-premise deployment option

## Notes

- All tables must follow 3NF and use snake_case naming with 2+ words
- All async operations must use Tokio runtime
- Database queries must use sqlx with compile-time verification
- Cache invalidation must be immediate for user changes
- DNS queries must be logged asynchronously via NATS
- Rate limiting must be enforced at edge layer (Cloudflare)

## Meeting Notes

### 2025-10-22 - Initial Architecture Review
- Confirmed Rust as primary language (Go as fallback)
- Decided on PostgreSQL 16 with TimescaleDB for time-series data
- Chose hash partitioning (16 partitions initially, expandable)
- NATS for async job processing instead of RabbitMQ
- Lemon Squeezy for payments (instead of Stripe)
- Passwordless authentication only (no passwords stored)

---

Last Updated: 2025-10-22

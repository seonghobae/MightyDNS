# Product Requirements Document (PRD)
## MightyDNS - Enterprise DNS Sinkhole Service

**Version:** 1.0
**Last Updated:** 2025-10-22
**Status:** Draft

---

## 1. Executive Summary

MightyDNS is a multi-tenant DNS sinkhole service designed for privacy-conscious users and organizations. Similar to NextDNS and AdGuard DNS, MightyDNS provides DNS-level content filtering with enhanced privacy, performance, and customization capabilities.

### 1.1 Vision
Provide a scalable, secure, and user-friendly DNS filtering solution that supports 10 million concurrent users while maintaining sub-10ms query response times.

### 1.2 Target Market
- **Primary:** Privacy-conscious individuals and families
- **Secondary:** Small to medium businesses (SMBs)
- **Tertiary:** Enterprise organizations requiring tenant isolation

---

## 2. Product Overview

### 2.1 Core Value Proposition
- **Privacy-First:** No logs, passwordless authentication, tenant data isolation
- **Performance:** Global anycast network, 10M+ concurrent user support
- **Customization:** Real-time blocklist/whitelist management with false-positive protection
- **Ease of Use:** Simple setup for DoH, DoT, and legacy DNS (UDP/53)

### 2.2 Key Features

#### 2.2.1 DNS Protocol Support
- **DNS over HTTPS (DoH):** RFC 8484 compliant, tenant identification via URL path
- **DNS over TLS (DoT):** RFC 7858 compliant, tenant identification via SNI
- **Traditional DNS (UDP/53):** Tenant identification via source IP address binding

#### 2.2.2 Authentication System (Passwordless)
- **FIDO2/WebAuthn:** Hardware security keys and platform authenticators
- **TOTP (Time-based OTP):** Standard authenticator app support (Google Authenticator, Authy, etc.)
- **Email OTP:** Magic link and 6-digit code for users without hardware tokens

#### 2.2.3 Content Filtering
- **Real-time Blocklist Management:**
  - User-customizable blocklists
  - Community-maintained lists (AdGuard, OISD, StevenBlack, etc.)
  - Category-based blocking (ads, malware, adult content, gambling, social media)

- **Whitelist System:**
  - Manual domain whitelisting
  - AI-powered false-positive detection
  - Whitelist priority over blocklists

#### 2.2.4 Tenant Management
- **Multi-Configuration Support:** Multiple DNS configurations per account
- **Device Profiling:** Different rules for different devices/networks
- **Access Control:** IP-based access restrictions
- **Usage Analytics:** Query logs, blocked domain statistics, performance metrics

#### 2.2.5 Subscription & Billing
- **Free Tier:** 300,000 queries/month, 1 configuration
- **Pro Tier ($4.99/month):** Unlimited queries, 10 configurations, priority support
- **Family Tier ($9.99/month):** 5 accounts, shared configurations
- **Business Tier (Custom):** SLA, dedicated support, on-premise option

Payment processing via **Lemon Squeezy** (Stripe alternative for SaaS).

---

## 3. User Personas

### 3.1 Personal User - "Privacy Pete"
- **Demographics:** 25-45, tech-savvy, values privacy
- **Goals:** Block ads and trackers across all devices
- **Pain Points:** Complex setup, VPN conflicts, DNS leaks
- **Use Case:** Configure DoH on browsers, DoT on mobile devices

### 3.2 Family Administrator - "Parent Paula"
- **Demographics:** 35-50, moderate tech skills, has children
- **Goals:** Protect children from inappropriate content
- **Pain Points:** Kids bypassing filters, managing multiple devices
- **Use Case:** Separate profiles for kids/adults, scheduled blocking

### 3.3 SMB IT Admin - "Admin Alex"
- **Demographics:** 30-50, IT professional, manages 10-100 devices
- **Goals:** Network-wide filtering, compliance, threat protection
- **Pain Points:** Employee productivity, malware infections
- **Use Case:** Configure router DNS, monitor employee activity (anonymized)

---

## 4. User Journey

### 4.1 Onboarding Flow
1. **Landing Page:** Value proposition, pricing, sign up CTA
2. **Email Input:** Enter email address
3. **Email Verification:** 6-digit OTP or magic link
4. **Setup Wizard:**
   - Choose default blocklist categories
   - Optional: Add FIDO2/TOTP for enhanced security
   - Generate DNS endpoints (DoH URL, DoT hostname, UDP IP)
5. **Device Setup Instructions:** OS-specific guides (Windows, macOS, iOS, Android, Linux, router)
6. **Verification:** Test DNS resolution, confirm filtering is active

### 4.2 Daily Usage Flow
1. **Dashboard:** View blocked queries, top blocked domains, query statistics
2. **Blocklist Management:** Add/remove domains, import/export lists, enable categories
3. **Whitelist Management:** Allowlist domains, review false positives
4. **Analytics:** Query trends, device breakdown, threat detection alerts

### 4.3 Billing Flow
1. **Upgrade Prompt:** Hit free tier limit or need advanced features
2. **Plan Selection:** Choose Pro/Family/Business tier
3. **Payment:** Lemon Squeezy checkout (card, PayPal, Google Pay)
4. **Confirmation:** Instant upgrade, receipt via email

---

## 5. Functional Requirements

### 5.1 Authentication (FR-AUTH)
- **FR-AUTH-001:** System MUST support FIDO2/WebAuthn registration and authentication
- **FR-AUTH-002:** System MUST support TOTP registration with QR code
- **FR-AUTH-003:** System MUST support email OTP with 10-minute expiry
- **FR-AUTH-004:** System MUST NOT store passwords (passwordless only)
- **FR-AUTH-005:** System MUST support session management with JWT tokens
- **FR-AUTH-006:** System MUST enforce rate limiting on OTP requests (max 3/hour)

### 5.2 DNS Resolution (FR-DNS)
- **FR-DNS-001:** System MUST support DoH on HTTPS port 443
- **FR-DNS-002:** System MUST support DoT on port 853
- **FR-DNS-003:** System MUST support UDP DNS on port 53
- **FR-DNS-004:** System MUST identify tenant from DoH URL path (e.g., `/dns-query/{tenant_id}`)
- **FR-DNS-005:** System MUST identify tenant from DoT SNI field
- **FR-DNS-006:** System MUST identify tenant from source IP address (for UDP)
- **FR-DNS-007:** System MUST respond to queries within 10ms (p95)
- **FR-DNS-008:** System MUST support DNSSEC validation
- **FR-DNS-009:** System MUST support EDNS Client Subnet (ECS) for geo-routing

### 5.3 Content Filtering (FR-FILTER)
- **FR-FILTER-001:** System MUST check blocklist before forwarding query
- **FR-FILTER-002:** System MUST prioritize whitelist over blocklist
- **FR-FILTER-003:** System MUST support wildcard domain blocking (e.g., `*.ads.example.com`)
- **FR-FILTER-004:** System MUST support regex-based blocking
- **FR-FILTER-005:** System MUST return NXDOMAIN or custom IP for blocked domains
- **FR-FILTER-006:** System MUST update blocklists in real-time (no cache delay)
- **FR-FILTER-007:** System MUST support category-based filtering (ads, malware, adult, etc.)

### 5.4 Tenant Management (FR-TENANT)
- **FR-TENANT-001:** System MUST isolate tenant data (queries, configs, analytics)
- **FR-TENANT-002:** System MUST support multiple configurations per tenant
- **FR-TENANT-003:** System MUST allow tenant to view query logs (if not disabled)
- **FR-TENANT-004:** System MUST support tenant-specific DNS endpoints
- **FR-TENANT-005:** System MUST enforce subscription limits (queries/month, configs)

### 5.5 Analytics (FR-ANALYTICS)
- **FR-ANALYTICS-001:** System MUST log DNS queries with timestamp, domain, result
- **FR-ANALYTICS-002:** System MUST aggregate analytics hourly (via pg_cron)
- **FR-ANALYTICS-003:** System MUST retain raw logs for 7 days, aggregated logs for 90 days
- **FR-ANALYTICS-004:** System MUST support query log export (CSV, JSON)

### 5.6 Billing (FR-BILLING)
- **FR-BILLING-001:** System MUST integrate with Lemon Squeezy webhooks
- **FR-BILLING-002:** System MUST track subscription status (active, expired, cancelled)
- **FR-BILLING-003:** System MUST enforce tier limits on downgrade/expiry
- **FR-BILLING-004:** System MUST send usage alerts (80%, 100% of quota)

---

## 6. Non-Functional Requirements

### 6.1 Performance (NFR-PERF)
- **NFR-PERF-001:** System MUST support 10 million concurrent users
- **NFR-PERF-002:** System MUST handle 100,000 queries per second per node
- **NFR-PERF-003:** DNS query response time MUST be <10ms (p95), <50ms (p99)
- **NFR-PERF-004:** API response time MUST be <200ms (p95)

### 6.2 Scalability (NFR-SCALE)
- **NFR-SCALE-001:** System MUST support horizontal scaling for DNS servers
- **NFR-SCALE-002:** System MUST support database partitioning (hash-based on tenant_id)
- **NFR-SCALE-003:** System MUST support auto-scaling based on load

### 6.3 Reliability (NFR-REL)
- **NFR-REL-001:** System MUST have 99.9% uptime SLA
- **NFR-REL-002:** System MUST have automatic failover for DNS servers
- **NFR-REL-003:** System MUST have database replication (primary + 2 replicas)

### 6.4 Security (NFR-SEC)
- **NFR-SEC-001:** System MUST encrypt DNS queries (DoH/DoT only on free tier)
- **NFR-SEC-002:** System MUST encrypt data at rest (AES-256)
- **NFR-SEC-003:** System MUST encrypt data in transit (TLS 1.3)
- **NFR-SEC-004:** System MUST implement rate limiting (100 queries/second per IP)
- **NFR-SEC-005:** System MUST implement DDoS protection

### 6.5 Privacy (NFR-PRIV)
- **NFR-PRIV-001:** System MUST NOT log queries if user opts out
- **NFR-PRIV-002:** System MUST NOT share data with third parties
- **NFR-PRIV-003:** System MUST support GDPR data deletion requests

---

## 7. Success Metrics

### 7.1 Product Metrics
- **Monthly Active Users (MAU):** Target 100K in Year 1
- **Conversion Rate (Free → Paid):** Target 5%
- **Churn Rate:** Target <3% monthly
- **Query Success Rate:** Target 99.9%

### 7.2 Performance Metrics
- **DNS Query Latency (p95):** <10ms
- **API Latency (p95):** <200ms
- **Uptime:** >99.9%

### 7.3 Business Metrics
- **Monthly Recurring Revenue (MRR):** Target $50K in Year 1
- **Customer Acquisition Cost (CAC):** Target <$20
- **Lifetime Value (LTV):** Target >$100

---

## 8. Out of Scope (v1.0)

- Mobile apps (iOS/Android native) - Use DoH/DoT instead
- On-premise deployment - Business tier in v2.0
- Custom DNS records (CNAME, A, AAAA) - v2.0
- Parental control dashboard - v2.0
- API for third-party integrations - v2.0

---

## 9. Open Questions

1. **Geographic Distribution:** Which regions should we prioritize for edge nodes?
2. **Blocklist Sources:** Which default blocklists should we include?
3. **Free Tier Limits:** Is 300K queries/month sufficient for trial?
4. **Data Retention:** Should we allow users to customize log retention?

---

## 10. Approval & Sign-off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Product Manager | TBD | - | - |
| Engineering Lead | TBD | - | - |
| Design Lead | TBD | - | - |

---

**Document Control:**
- **Next Review Date:** 2025-11-22
- **Distribution:** Engineering, Product, Design teams
- **Classification:** Internal

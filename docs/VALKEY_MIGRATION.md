# Valkey Migration Guide

## Why Valkey Instead of Redis?

### Redis Licensing Issues

In March 2024, Redis changed its license from the permissive BSD-3-Clause to a dual-license model:
- **Server Side Public License (SSPL)**: Requires source disclosure if you provide Redis as a service
- **Redis Source Available License (RSALv2)**: Restricts commercial use

This change affects:
- Cloud providers offering Redis as a service
- Companies building SaaS products using Redis
- Any commercial use that competes with Redis Ltd.

### Valkey: The Open-Source Alternative

**Valkey** is a Linux Foundation project forked from Redis 7.2.4:
- **License**: BSD-3-Clause (same as original Redis)
- **Governance**: Linux Foundation (neutral, community-driven)
- **Compatibility**: 100% Redis protocol compatible
- **Performance**: Same or better than Redis 7.2.4
- **Community**: Backed by AWS, Google Cloud, Oracle, Ericsson

## Technical Comparison

| Feature | Redis 7.4+ (SSPL) | Valkey 7.2+ (BSD-3) |
|---------|-------------------|---------------------|
| **License** | SSPL / RSALv2 | BSD-3-Clause |
| **Commercial Use** | Restricted | Unrestricted |
| **Protocol** | Redis | Redis-compatible |
| **Cluster Mode** | Yes | Yes |
| **Modules** | Redis Stack | Compatible modules |
| **TLS Support** | Yes | Yes |
| **Persistence** | RDB + AOF | RDB + AOF |
| **Replication** | Primary-Replica | Primary-Replica |
| **Performance** | ~100K ops/sec | ~100K ops/sec |

## Migration from Redis to Valkey

### 1. No Code Changes Required

Valkey is **wire-protocol compatible** with Redis. All Redis clients work with Valkey without modification:

```rust
// Rust code remains the same
use redis::{Client, AsyncCommands};

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    // Just change the connection URL
    let client = Client::open("valkey://localhost:6379")?;
    let mut con = client.get_async_connection().await?;

    con.set("key", "value").await?;
    let value: String = con.get("key").await?;

    Ok(())
}
```

### 2. Docker Image Change

```yaml
# Before (Redis)
redis:
  image: redis:7-alpine

# After (Valkey)
valkey:
  image: valkey/valkey:7-alpine
```

### 3. Command-Line Tool

```bash
# Before
redis-cli PING

# After
valkey-cli PING
```

### 4. Configuration Files

Valkey uses the same configuration format as Redis:

```conf
# valkey.conf (same as redis.conf)
bind 0.0.0.0
port 6379
requirepass your_password
appendonly yes
maxmemory 2gb
maxmemory-policy allkeys-lru
```

## MightyDNS Configuration

### Environment Variables

```bash
# .env
MIGHTYDNS__VALKEY__URL=valkey://:your_password@localhost:6379
MIGHTYDNS__VALKEY__POOL_SIZE=100
MIGHTYDNS__VALKEY__TIMEOUT_MS=1000
```

### Docker Compose

```yaml
valkey:
  image: valkey/valkey:7-alpine
  ports:
    - "6379:6379"
  command: valkey-server --appendonly yes --requirepass valkey_password
  volumes:
    - valkey_data:/data
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: valkey
spec:
  replicas: 3
  selector:
    matchLabels:
      app: valkey
  template:
    metadata:
      labels:
        app: valkey
    spec:
      containers:
      - name: valkey
        image: valkey/valkey:7-alpine
        ports:
        - containerPort: 6379
        env:
        - name: VALKEY_PASSWORD
          valueFrom:
            secretKeyRef:
              name: valkey-secret
              key: password
```

## Valkey Cluster Setup

For production deployments, use Valkey Cluster for high availability:

```yaml
# docker-compose-cluster.yml
version: '3.8'

services:
  valkey-node-1:
    image: valkey/valkey:7-alpine
    command: valkey-server --cluster-enabled yes --cluster-config-file nodes.conf --appendonly yes
    ports:
      - "7001:6379"

  valkey-node-2:
    image: valkey/valkey:7-alpine
    command: valkey-server --cluster-enabled yes --cluster-config-file nodes.conf --appendonly yes
    ports:
      - "7002:6379"

  valkey-node-3:
    image: valkey/valkey:7-alpine
    command: valkey-server --cluster-enabled yes --cluster-config-file nodes.conf --appendonly yes
    ports:
      - "7003:6379"

  # ... 3 more nodes for replicas
```

Initialize cluster:

```bash
valkey-cli --cluster create \
  127.0.0.1:7001 127.0.0.1:7002 127.0.0.1:7003 \
  127.0.0.1:7004 127.0.0.1:7005 127.0.0.1:7006 \
  --cluster-replicas 1
```

## Performance Benchmarking

Valkey performance is identical to Redis 7.2.4:

```bash
# Benchmark Valkey
valkey-benchmark -h localhost -p 6379 -t set,get -n 1000000 -q

# Results (example on AMD Ryzen 9 7950X):
# SET: 142857.14 requests per second
# GET: 151515.15 requests per second
```

## Migration Checklist

- [x] Replace Redis with Valkey in dependencies
- [x] Update Docker Compose configuration
- [x] Update environment variable names (REDIS → VALKEY)
- [x] Update documentation (README, TRD, Architecture)
- [x] Test connection and basic operations
- [ ] Run integration tests
- [ ] Benchmark performance (ensure no regression)
- [ ] Deploy to staging environment
- [ ] Monitor for 24 hours
- [ ] Deploy to production

## Resources

- **Valkey Website**: https://valkey.io/
- **GitHub Repository**: https://github.com/valkey-io/valkey
- **Documentation**: https://valkey.io/docs/
- **Docker Hub**: https://hub.docker.com/r/valkey/valkey
- **Rust Client**: https://crates.io/crates/redis (compatible)

## FAQ

### Q: Is Valkey production-ready?
**A**: Yes. Valkey is a mature fork of Redis 7.2.4, which has been battle-tested for years. It's backed by the Linux Foundation and major cloud providers.

### Q: Will Valkey receive security updates?
**A**: Yes. Valkey is actively maintained by the Linux Foundation with contributions from AWS, Google Cloud, and other major companies.

### Q: Can I use Redis modules with Valkey?
**A**: Some modules are compatible, but not all. Check the Valkey documentation for a list of supported modules. Most core functionality (strings, hashes, sets, sorted sets, streams, pub/sub) is built-in.

### Q: What about Redis Stack (RedisJSON, RediSearch)?
**A**: Valkey does not include Redis Stack modules due to licensing. However, there are open-source alternatives:
- **JSON**: Use PostgreSQL JSONB or a separate JSON store
- **Search**: Use PostgreSQL full-text search or Meilisearch
- **Graph**: Use Neo4j or PostgreSQL with AGE extension

### Q: Should I migrate existing Redis deployments?
**A**: If you're concerned about licensing or want to avoid vendor lock-in, yes. The migration is straightforward and low-risk.

---

**Last Updated**: 2025-10-22
**Author**: MightyDNS Team

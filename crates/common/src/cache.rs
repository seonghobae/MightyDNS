use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use moka::future::Cache as MokaCache;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::config::ValkeyConfig;
use crate::database::Database;
use crate::error::{Error, Result};
use crate::models::{DnsResultType, TenantAccount, TenantConfig};

/// Multi-layer cache for DNS queries and tenant data
#[derive(Clone)]
pub struct Cache {
    /// L1: In-memory LRU cache (fastest, per-process)
    memory_cache: MokaCache<String, CacheValue>,

    /// L2: Valkey cluster (shared across all instances)
    valkey_pool: RedisPool,

    /// L3: PostgreSQL (slowest, authoritative)
    database: Database,
}

/// Cached values
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CacheValue {
    /// Tenant account and config lookup result
    TenantLookup(Option<TenantLookupResult>),

    /// Domain blocklist check result
    BlocklistStatus(bool),

    /// Domain whitelist check result
    WhitelistStatus(bool),

    /// DNS query result type
    FilterResult(DnsResultType),

    /// Generic string value
    String(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantLookupResult {
    pub tenant: TenantAccount,
    pub config: TenantConfig,
}

impl Cache {
    /// Create a new multi-layer cache
    pub async fn new(config: &ValkeyConfig, database: Database) -> Result<Self> {
        // L1: In-memory cache (1M entries, 5-minute TTL)
        let memory_cache = MokaCache::builder()
            .max_capacity(1_000_000)
            .time_to_live(Duration::from_secs(300))
            .build();

        // L2: Valkey connection pool
        let redis_config = RedisConfig::from_url(&config.url);
        let valkey_pool = redis_config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| Error::Internal(format!("Failed to create Valkey pool: {}", e)))?;

        tracing::info!("Cache initialized (memory + Valkey)");

        Ok(Self {
            memory_cache,
            valkey_pool,
            database,
        })
    }

    // ============================================================================
    // Tenant Lookup Methods
    // ============================================================================

    /// Get tenant by identifier (used for DoH/DoT)
    pub async fn get_tenant_by_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<TenantAccount>> {
        let cache_key = format!("tenant:identifier:{}", identifier);

        // L1: Check memory cache
        if let Some(CacheValue::TenantLookup(result)) = self.memory_cache.get(&cache_key).await {
            return Ok(result.map(|r| r.tenant));
        }

        // L2: Check Valkey
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            if let Ok(cached_json) = conn.get::<_, String>(&cache_key).await {
                if let Ok(result) = serde_json::from_str::<Option<TenantLookupResult>>(&cached_json) {
                    // Backfill L1
                    self.memory_cache.insert(
                        cache_key.clone(),
                        CacheValue::TenantLookup(result.clone()),
                    ).await;

                    return Ok(result.map(|r| r.tenant));
                }
            }
        }

        // L3: Query database
        let tenant = self.database.get_tenant_by_identifier(identifier).await?;

        if let Some(ref t) = tenant {
            // Get default config
            let configs = self.database.get_tenant_configs(&t.tenant_id).await?;
            if let Some(config) = configs.first() {
                let result = TenantLookupResult {
                    tenant: t.clone(),
                    config: config.clone(),
                };

                // Backfill caches
                self.set_tenant_cache(&cache_key, Some(result)).await?;
            }
        }

        Ok(tenant)
    }

    /// Get tenant by IP address (used for UDP/53)
    pub async fn get_tenant_by_ip(&self, ip_address: &str) -> Result<Option<(TenantAccount, TenantConfig)>> {
        let cache_key = format!("tenant:ip:{}", ip_address);

        // L1: Check memory cache
        if let Some(CacheValue::TenantLookup(result)) = self.memory_cache.get(&cache_key).await {
            return Ok(result.map(|r| (r.tenant, r.config)));
        }

        // L2: Check Valkey
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            if let Ok(cached_json) = conn.get::<_, String>(&cache_key).await {
                if let Ok(result) = serde_json::from_str::<Option<TenantLookupResult>>(&cached_json) {
                    // Backfill L1
                    self.memory_cache.insert(
                        cache_key.clone(),
                        CacheValue::TenantLookup(result.clone()),
                    ).await;

                    return Ok(result.map(|r| (r.tenant, r.config)));
                }
            }
        }

        // L3: Query database
        let result = self.database.get_tenant_by_ip(ip_address).await?;

        if let Some((ref tenant, ref config)) = result {
            let lookup_result = TenantLookupResult {
                tenant: tenant.clone(),
                config: config.clone(),
            };

            // Backfill caches (60-second TTL for IP bindings)
            self.set_tenant_cache_with_ttl(&cache_key, Some(lookup_result), 60).await?;
        }

        Ok(result)
    }

    // ============================================================================
    // Blocklist/Whitelist Methods
    // ============================================================================

    /// Check if domain is whitelisted for tenant
    pub async fn is_domain_whitelisted(&self, tenant_id: &Uuid, domain: &str) -> Result<bool> {
        let cache_key = format!("whitelist:{}:{}", tenant_id, domain);

        // L1: Check memory cache
        if let Some(CacheValue::WhitelistStatus(is_whitelisted)) = self.memory_cache.get(&cache_key).await {
            return Ok(is_whitelisted);
        }

        // L2: Check Valkey
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            if let Ok(is_whitelisted) = conn.get::<_, bool>(&cache_key).await {
                // Backfill L1
                self.memory_cache.insert(
                    cache_key.clone(),
                    CacheValue::WhitelistStatus(is_whitelisted),
                ).await;

                return Ok(is_whitelisted);
            }
        }

        // L3: Query database
        let is_whitelisted = self.database.is_domain_whitelisted(tenant_id, domain).await?;

        // Backfill caches
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            let _: Result<(), _> = conn.set_ex(&cache_key, is_whitelisted, 300).await;
        }
        self.memory_cache.insert(
            cache_key,
            CacheValue::WhitelistStatus(is_whitelisted),
        ).await;

        Ok(is_whitelisted)
    }

    /// Check if domain is blocked
    pub async fn is_domain_blocked(&self, domain: &str) -> Result<bool> {
        let cache_key = format!("blocklist:{}", domain);

        // L1: Check memory cache
        if let Some(CacheValue::BlocklistStatus(is_blocked)) = self.memory_cache.get(&cache_key).await {
            return Ok(is_blocked);
        }

        // L2: Check Valkey
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            if let Ok(is_blocked) = conn.get::<_, bool>(&cache_key).await {
                // Backfill L1
                self.memory_cache.insert(
                    cache_key.clone(),
                    CacheValue::BlocklistStatus(is_blocked),
                ).await;

                return Ok(is_blocked);
            }
        }

        // L3: Query database
        let entry = self.database.is_domain_blocked(domain).await?;
        let is_blocked = entry.is_some();

        // Backfill caches
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            let _: Result<(), _> = conn.set_ex(&cache_key, is_blocked, 300).await;
        }
        self.memory_cache.insert(
            cache_key,
            CacheValue::BlocklistStatus(is_blocked),
        ).await;

        Ok(is_blocked)
    }

    /// Check domain filter (blocklist + whitelist + config)
    pub async fn check_domain_filter(
        &self,
        tenant_id: &Uuid,
        config_id: &Uuid,
        domain: &str,
    ) -> Result<DnsResultType> {
        let cache_key = format!("filter:{}:{}:{}", tenant_id, config_id, domain);

        // L1: Check memory cache
        if let Some(CacheValue::FilterResult(result)) = self.memory_cache.get(&cache_key).await {
            return Ok(result);
        }

        // L2: Check Valkey
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            if let Ok(cached_json) = conn.get::<_, String>(&cache_key).await {
                if let Ok(result) = serde_json::from_str::<DnsResultType>(&cached_json) {
                    // Backfill L1
                    self.memory_cache.insert(
                        cache_key.clone(),
                        CacheValue::FilterResult(result),
                    ).await;

                    return Ok(result);
                }
            }
        }

        // L3: Query database
        let result = self.database.check_domain_filter(tenant_id, config_id, domain).await?;

        // Backfill caches
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            let json = serde_json::to_string(&result).unwrap_or_default();
            let _: Result<(), _> = conn.set_ex(&cache_key, json, 300).await;
        }
        self.memory_cache.insert(
            cache_key,
            CacheValue::FilterResult(result),
        ).await;

        Ok(result)
    }

    // ============================================================================
    // Cache Management Methods
    // ============================================================================

    /// Set tenant cache with default TTL (300 seconds)
    async fn set_tenant_cache(&self, key: &str, value: Option<TenantLookupResult>) -> Result<()> {
        self.set_tenant_cache_with_ttl(key, value, 300).await
    }

    /// Set tenant cache with custom TTL
    async fn set_tenant_cache_with_ttl(
        &self,
        key: &str,
        value: Option<TenantLookupResult>,
        ttl_secs: u64,
    ) -> Result<()> {
        // Set in Valkey
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            let json = serde_json::to_string(&value).unwrap_or_default();
            let _: Result<(), _> = conn.set_ex(key, json, ttl_secs as usize).await;
        }

        // Set in memory cache
        self.memory_cache.insert(
            key.to_string(),
            CacheValue::TenantLookup(value),
        ).await;

        Ok(())
    }

    /// Invalidate cache for a specific key
    pub async fn invalidate(&self, key: &str) -> Result<()> {
        // Remove from memory cache
        self.memory_cache.invalidate(key).await;

        // Remove from Valkey
        if let Ok(mut conn) = self.valkey_pool.get().await {
            use redis::AsyncCommands;
            let _: Result<(), _> = conn.del(key).await;
        }

        Ok(())
    }

    /// Invalidate all tenant-related caches
    pub async fn invalidate_tenant(&self, tenant_id: &Uuid) -> Result<()> {
        // In production, you'd want to use Redis SCAN or maintain a set of keys
        // For now, we'll invalidate common patterns
        let patterns = vec![
            format!("tenant:identifier:*"),
            format!("tenant:ip:*"),
            format!("filter:{}:*", tenant_id),
            format!("whitelist:{}:*", tenant_id),
        ];

        for pattern in patterns {
            self.invalidate(&pattern).await?;
        }

        Ok(())
    }

    /// Get cache statistics (for monitoring)
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            memory_entry_count: self.memory_cache.entry_count(),
            memory_weighted_size: self.memory_cache.weighted_size(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CacheStats {
    pub memory_entry_count: u64,
    pub memory_weighted_size: u64,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_value_variants() {
        let tenant_lookup = CacheValue::TenantLookup(None);
        let blocklist_status = CacheValue::BlocklistStatus(true);
        let whitelist_status = CacheValue::WhitelistStatus(false);
        let filter_result = CacheValue::FilterResult(DnsResultType::Allowed);
        let string_value = CacheValue::String("test".to_string());

        // All variants should be created successfully
        match tenant_lookup {
            CacheValue::TenantLookup(_) => (),
            _ => panic!("Wrong variant"),
        }
        match blocklist_status {
            CacheValue::BlocklistStatus(true) => (),
            _ => panic!("Wrong variant"),
        }
        match whitelist_status {
            CacheValue::WhitelistStatus(false) => (),
            _ => panic!("Wrong variant"),
        }
        match filter_result {
            CacheValue::FilterResult(_) => (),
            _ => panic!("Wrong variant"),
        }
        match string_value {
            CacheValue::String(_) => (),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_cache_value_serialization() {
        let value = CacheValue::BlocklistStatus(true);
        let json = serde_json::to_string(&value).expect("Serialization failed");
        let deserialized: CacheValue = serde_json::from_str(&json).expect("Deserialization failed");
        
        match deserialized {
            CacheValue::BlocklistStatus(status) => assert!(status),
            _ => panic!("Wrong variant after deserialization"),
        }
    }

    #[test]
    fn test_tenant_lookup_result_structure() {
        let tenant = TenantAccount {
            tenant_id: Uuid::new_v4(),
            email_address: "test@example.com".to_string(),
            tenant_identifier: "test123".to_string(),
            subscription_tier: SubscriptionTier::Free,
            subscription_status: SubscriptionStatus::Active,
            account_created_at: chrono::Utc::now(),
            account_updated_at: chrono::Utc::now(),
            is_account_active: true,
        };

        let config = TenantConfig {
            config_id: Uuid::new_v4(),
            tenant_id: tenant.tenant_id,
            config_name: "Default".to_string(),
            config_description: None,
            is_logging_enabled: true,
            is_dnssec_enabled: false,
            blocked_response_ip: "0.0.0.0".to_string(),
            config_created_at: chrono::Utc::now(),
            config_updated_at: chrono::Utc::now(),
            is_config_active: true,
        };

        let result = TenantLookupResult {
            tenant: tenant.clone(),
            config: config.clone(),
        };

        assert_eq!(result.tenant.email_address, tenant.email_address);
        assert_eq!(result.config.config_name, config.config_name);
    }

    #[test]
    fn test_cache_stats_structure() {
        let stats = CacheStats {
            memory_entry_count: 1000,
            memory_weighted_size: 5000000,
        };

        assert_eq!(stats.memory_entry_count, 1000);
        assert_eq!(stats.memory_weighted_size, 5000000);
    }

    #[test]
    fn test_cache_stats_serialization() {
        let stats = CacheStats {
            memory_entry_count: 500,
            memory_weighted_size: 2500000,
        };

        let json = serde_json::to_string(&stats).expect("Serialization failed");
        assert!(json.contains("memory_entry_count"));
        assert!(json.contains("memory_weighted_size"));
    }

    #[test]
    fn test_cache_value_clone() {
        let value = CacheValue::String("test value".to_string());
        let cloned = value.clone();

        match (value, cloned) {
            (CacheValue::String(s1), CacheValue::String(s2)) => assert_eq!(s1, s2),
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_dns_result_type_in_cache_value() {
        let allowed = CacheValue::FilterResult(DnsResultType::Allowed);
        let blocked = CacheValue::FilterResult(DnsResultType::Blocked);
        let whitelisted = CacheValue::FilterResult(DnsResultType::Whitelisted);
        let error = CacheValue::FilterResult(DnsResultType::Error);

        // All should serialize successfully
        let _ = serde_json::to_string(&allowed).unwrap();
        let _ = serde_json::to_string(&blocked).unwrap();
        let _ = serde_json::to_string(&whitelisted).unwrap();
        let _ = serde_json::to_string(&error).unwrap();
    }

    #[test]
    fn test_tenant_lookup_result_serialization() {
        let tenant = TenantAccount {
            tenant_id: Uuid::new_v4(),
            email_address: "cache@test.com".to_string(),
            tenant_identifier: "cache123".to_string(),
            subscription_tier: SubscriptionTier::Pro,
            subscription_status: SubscriptionStatus::Active,
            account_created_at: chrono::Utc::now(),
            account_updated_at: chrono::Utc::now(),
            is_account_active: true,
        };

        let config = TenantConfig {
            config_id: Uuid::new_v4(),
            tenant_id: tenant.tenant_id,
            config_name: "Test Config".to_string(),
            config_description: Some("Test description".to_string()),
            is_logging_enabled: false,
            is_dnssec_enabled: true,
            blocked_response_ip: "127.0.0.1".to_string(),
            config_created_at: chrono::Utc::now(),
            config_updated_at: chrono::Utc::now(),
            is_config_active: true,
        };

        let lookup_result = TenantLookupResult {
            tenant,
            config,
        };

        let json = serde_json::to_string(&lookup_result).expect("Serialization failed");
        let deserialized: TenantLookupResult = serde_json::from_str(&json)
            .expect("Deserialization failed");

        assert_eq!(lookup_result.tenant.email_address, deserialized.tenant.email_address);
        assert_eq!(lookup_result.config.config_name, deserialized.config.config_name);
    }
}
use serde::{Deserialize, Serialize};
use config::{Config as ConfigBuilder, Environment, File};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub valkey: ValkeyConfig,
    pub nats: NatsConfig,
    pub dns: DnsConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValkeyConfig {
    pub url: String,
    pub pool_size: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsConfig {
    pub url: String,
    pub max_reconnects: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DnsConfig {
    pub upstream_servers: Vec<String>,
    pub timeout_ms: u64,
    pub cache_ttl_secs: u64,
    pub doh_port: u16,
    pub dot_port: u16,
    pub udp_port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub otp_expiry_minutes: i64,
    pub webauthn_rp_id: String,
    pub webauthn_rp_name: String,
    pub webauthn_origin: String,
}

impl Config {
    pub fn from_env() -> crate::Result<Self> {
        let config = ConfigBuilder::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!(
                "config/{}",
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string())
            )).required(false))
            .add_source(Environment::with_prefix("MIGHTYDNS").separator("__"))
            .build()?;

        Ok(config.try_deserialize()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        
        // Server config
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert!(config.server.workers > 0);
        
        // Database config
        assert_eq!(config.database.url, "postgresql://localhost/mightydns");
        assert_eq!(config.database.max_connections, 100);
        assert_eq!(config.database.min_connections, 10);
        assert_eq!(config.database.connect_timeout_secs, 10);
        assert_eq!(config.database.idle_timeout_secs, 600);
        
        // Valkey config
        assert!(config.valkey.url.contains("valkey://"));
        assert_eq!(config.valkey.pool_size, 100);
        assert_eq!(config.valkey.timeout_ms, 1000);
        
        // NATS config
        assert!(config.nats.url.contains("nats://"));
        assert_eq!(config.nats.max_reconnects, 10);
        
        // DNS config
        assert!(!config.dns.upstream_servers.is_empty());
        assert_eq!(config.dns.timeout_ms, 5000);
        assert_eq!(config.dns.cache_ttl_secs, 300);
        assert_eq!(config.dns.doh_port, 8443);
        assert_eq!(config.dns.dot_port, 853);
        assert_eq!(config.dns.udp_port, 53);
        
        // Auth config
        assert!(!config.auth.jwt_secret.is_empty());
        assert_eq!(config.auth.jwt_expiry_hours, 1);
        assert_eq!(config.auth.otp_expiry_minutes, 10);
        assert!(!config.auth.webauthn_rp_id.is_empty());
        assert!(!config.auth.webauthn_rp_name.is_empty());
        assert!(!config.auth.webauthn_origin.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        
        // Should serialize to JSON successfully
        let json = serde_json::to_string(&config).expect("Failed to serialize config");
        assert!(json.contains("server"));
        assert!(json.contains("database"));
        assert!(json.contains("valkey"));
        
        // Should deserialize from JSON successfully
        let deserialized: Config = serde_json::from_str(&json).expect("Failed to deserialize config");
        assert_eq!(deserialized.server.port, config.server.port);
        assert_eq!(deserialized.database.max_connections, config.database.max_connections);
    }

    #[test]
    fn test_server_config_values() {
        let server = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            workers: 4,
        };
        
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 3000);
        assert_eq!(server.workers, 4);
    }

    #[test]
    fn test_database_config_constraints() {
        let db_config = DatabaseConfig {
            url: "postgresql://user:pass@localhost:5432/db".to_string(),
            max_connections: 50,
            min_connections: 5,
            connect_timeout_secs: 30,
            idle_timeout_secs: 300,
        };
        
        assert!(db_config.max_connections >= db_config.min_connections);
        assert!(db_config.connect_timeout_secs > 0);
        assert!(db_config.idle_timeout_secs > db_config.connect_timeout_secs);
    }

    #[test]
    fn test_dns_config_upstream_servers() {
        let dns = DnsConfig {
            upstream_servers: vec![
                "1.1.1.1:53".to_string(),
                "8.8.8.8:53".to_string(),
                "9.9.9.9:53".to_string(),
            ],
            timeout_ms: 3000,
            cache_ttl_secs: 600,
            doh_port: 443,
            dot_port: 853,
            udp_port: 53,
        };
        
        assert_eq!(dns.upstream_servers.len(), 3);
        assert!(dns.upstream_servers.iter().all(|s| s.contains(':')));
        assert!(dns.timeout_ms > 0);
        assert!(dns.cache_ttl_secs > 0);
    }

    #[test]
    fn test_auth_config_security_parameters() {
        let auth = AuthConfig {
            jwt_secret: "super-secret-key-min-32-chars-long".to_string(),
            jwt_expiry_hours: 24,
            otp_expiry_minutes: 5,
            webauthn_rp_id: "example.com".to_string(),
            webauthn_rp_name: "Example App".to_string(),
            webauthn_origin: "https://example.com".to_string(),
        };
        
        // JWT secret should be reasonably long
        assert!(auth.jwt_secret.len() >= 16);
        assert!(auth.jwt_expiry_hours > 0);
        assert!(auth.otp_expiry_minutes > 0 && auth.otp_expiry_minutes <= 30);
        assert!(auth.webauthn_origin.starts_with("https://"));
    }

    #[test]
    fn test_valkey_config_serialization() {
        let valkey = ValkeyConfig {
            url: "valkey://localhost:6379/0".to_string(),
            pool_size: 50,
            timeout_ms: 500,
        };
        
        let json = serde_json::to_string(&valkey).unwrap();
        let deserialized: ValkeyConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.url, valkey.url);
        assert_eq!(deserialized.pool_size, valkey.pool_size);
        assert_eq!(deserialized.timeout_ms, valkey.timeout_ms);
    }

    #[test]
    fn test_nats_config_reconnect_strategy() {
        let nats = NatsConfig {
            url: "nats://nats.example.com:4222".to_string(),
            max_reconnects: 5,
        };
        
        assert!(nats.max_reconnects > 0);
        assert!(nats.url.starts_with("nats://"));
    }

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        
        assert_eq!(config.server.port, cloned.server.port);
        assert_eq!(config.database.url, cloned.database.url);
        assert_eq!(config.dns.upstream_servers, cloned.dns.upstream_servers);
    }
}
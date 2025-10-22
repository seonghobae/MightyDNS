use serde::{Deserialize, Serialize};
use config::{Config as ConfigBuilder, Environment, File};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
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
pub struct RedisConfig {
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

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: num_cpus::get(),
            },
            database: DatabaseConfig {
                url: "postgresql://localhost/mightydns".to_string(),
                max_connections: 100,
                min_connections: 10,
                connect_timeout_secs: 10,
                idle_timeout_secs: 600,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: 100,
                timeout_ms: 1000,
            },
            nats: NatsConfig {
                url: "nats://localhost:4222".to_string(),
                max_reconnects: 10,
            },
            dns: DnsConfig {
                upstream_servers: vec![
                    "1.1.1.1:53".to_string(),
                    "8.8.8.8:53".to_string(),
                ],
                timeout_ms: 5000,
                cache_ttl_secs: 300,
                doh_port: 8443,
                dot_port: 853,
                udp_port: 53,
            },
            auth: AuthConfig {
                jwt_secret: "change-me-in-production".to_string(),
                jwt_expiry_hours: 1,
                otp_expiry_minutes: 10,
                webauthn_rp_id: "mightydns.com".to_string(),
                webauthn_rp_name: "MightyDNS".to_string(),
                webauthn_origin: "https://mightydns.com".to_string(),
            },
        }
    }
}

use async_nats::jetstream::{self, consumer::PullConsumer, stream::Stream};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::config::NatsConfig;
use crate::error::{Error, Result};

/// NATS client wrapper for pub/sub messaging
#[derive(Clone)]
pub struct NatsClient {
    client: async_nats::Client,
    jetstream: jetstream::Context,
}

impl NatsClient {
    /// Create a new NATS client
    pub async fn new(config: &NatsConfig) -> Result<Self> {
        let client = async_nats::connect(&config.url)
            .await
            .map_err(|e| Error::Nats(e.to_string()))?;

        let jetstream = jetstream::new(client.clone());

        tracing::info!("NATS client connected to {}", config.url);

        Ok(Self { client, jetstream })
    }

    /// Get JetStream context for stream management
    pub fn jetstream(&self) -> &jetstream::Context {
        &self.jetstream
    }

    /// Publish a message to a subject
    pub async fn publish<T: Serialize>(&self, subject: &str, payload: &T) -> Result<()> {
        let json = serde_json::to_vec(payload)?;

        self.client
            .publish(subject.to_string(), json.into())
            .await
            .map_err(|e| Error::Nats(e.to_string()))?;

        Ok(())
    }

    /// Publish a message and wait for acknowledgment
    pub async fn publish_with_ack<T: Serialize>(&self, subject: &str, payload: &T) -> Result<()> {
        let json = serde_json::to_vec(payload)?;

        self.jetstream
            .publish(subject.to_string(), json.into())
            .await
            .map_err(|e| Error::Nats(e.to_string()))?
            .await
            .map_err(|e| Error::Nats(e.to_string()))?;

        Ok(())
    }

    /// Create or get a JetStream stream
    pub async fn create_stream(&self, config: jetstream::stream::Config) -> Result<Stream> {
        let stream = self
            .jetstream
            .get_or_create_stream(config)
            .await
            .map_err(|e| Error::Nats(e.to_string()))?;

        Ok(stream)
    }

    /// Create a consumer for a stream
    pub async fn create_consumer(
        &self,
        stream: &Stream,
        config: jetstream::consumer::pull::Config,
    ) -> Result<PullConsumer> {
        let consumer = stream
            .create_consumer(config)
            .await
            .map_err(|e| Error::Nats(e.to_string()))?;

        Ok(consumer)
    }

    /// Subscribe to a subject (core NATS, not JetStream)
    pub async fn subscribe(&self, subject: &str) -> Result<async_nats::Subscriber> {
        let subscriber = self
            .client
            .subscribe(subject.to_string())
            .await
            .map_err(|e| Error::Nats(e.to_string()))?;

        Ok(subscriber)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        self.client
            .flush()
            .await
            .map_err(|e| Error::Nats(e.to_string()))?;

        Ok(())
    }
}

/// NATS message payload types
pub mod messages {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::models::{DnsQueryType, DnsResultType};

    /// DNS query log message (published to dns.query.log)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DnsQueryLogMessage {
        pub tenant_id: Uuid,
        pub config_id: Option<Uuid>,
        pub query_timestamp: chrono::DateTime<chrono::Utc>,
        pub query_domain_name: String,
        pub query_type: DnsQueryType,
        pub query_result_type: DnsResultType,
        pub response_ip_address: Option<String>,
        pub query_latency_ms: Option<i16>,
        pub query_source_protocol: Option<String>,
        pub query_source_ip: Option<String>,
    }

    /// Email sending message (published to email.send)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EmailMessage {
        pub to: String,
        pub subject: String,
        pub template: String,
        pub variables: serde_json::Value,
        pub priority: EmailPriority,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum EmailPriority {
        High,    // OTP codes
        Normal,  // Receipts, notifications
        Low,     // Marketing
    }

    /// Blocklist update message (published to blocklist.update)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlocklistUpdateMessage {
        pub source_id: Uuid,
        pub source_name: String,
        pub source_url: String,
        pub source_format: String,
    }

    /// Analytics aggregation message (published to analytics.aggregate)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AnalyticsAggregateMessage {
        pub tenant_id: Option<Uuid>, // None = aggregate all tenants
        pub start_time: chrono::DateTime<chrono::Utc>,
        pub end_time: chrono::DateTime<chrono::Utc>,
        pub aggregation_type: AggregationType,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum AggregationType {
        Hourly,
        Daily,
        Weekly,
        Monthly,
    }
}

/// Stream configurations for MightyDNS
pub mod streams {
    use async_nats::jetstream;
    use std::time::Duration;

    /// DNS query log stream configuration
    pub fn dns_query_log_stream() -> jetstream::stream::Config {
        jetstream::stream::Config {
            name: "DNS_QUERIES".to_string(),
            description: Some("DNS query logs for analytics".to_string()),
            subjects: vec!["dns.query.log".to_string()],
            max_age: Duration::from_secs(86400), // 24-hour retention (then moved to TimescaleDB)
            max_bytes: 10 * 1024 * 1024 * 1024, // 10 GB max
            storage: jetstream::stream::StorageType::File,
            num_replicas: 1, // Single-node dev default; configure for HA in production
            ..Default::default()
        }
    }

    /// Email sending stream configuration
    pub fn email_stream() -> jetstream::stream::Config {
        jetstream::stream::Config {
            name: "EMAILS".to_string(),
            description: Some("Email sending queue".to_string()),
            subjects: vec!["email.send".to_string()],
            max_age: Duration::from_secs(3600), // 1-hour retention
            max_bytes: 1024 * 1024 * 1024,      // 1 GB max
            storage: jetstream::stream::StorageType::File,
            num_replicas: 1, // Single-node dev default; configure for HA in production
            ..Default::default()
        }
    }

    /// Blocklist update stream configuration
    pub fn blocklist_update_stream() -> jetstream::stream::Config {
        jetstream::stream::Config {
            name: "BLOCKLIST_UPDATES".to_string(),
            description: Some("Blocklist update jobs".to_string()),
            subjects: vec!["blocklist.update".to_string()],
            max_age: Duration::from_secs(7200), // 2-hour retention
            max_bytes: 100 * 1024 * 1024,       // 100 MB max
            storage: jetstream::stream::StorageType::File,
            num_replicas: 1, // Not critical
            ..Default::default()
        }
    }

    /// Analytics aggregation stream configuration
    pub fn analytics_stream() -> jetstream::stream::Config {
        jetstream::stream::Config {
            name: "ANALYTICS".to_string(),
            description: Some("Analytics aggregation jobs".to_string()),
            subjects: vec!["analytics.aggregate".to_string()],
            max_age: Duration::from_secs(3600), // 1-hour retention
            max_bytes: 100 * 1024 * 1024,       // 100 MB max
            storage: jetstream::stream::StorageType::File,
            num_replicas: 1,
            ..Default::default()
        }
    }
}

/// Consumer configurations for workers
pub mod consumers {
    use async_nats::jetstream;

    /// DNS query logger consumer
    pub fn dns_logger_consumer() -> jetstream::consumer::pull::Config {
        jetstream::consumer::pull::Config {
            durable_name: Some("dns-logger".to_string()),
            description: Some("DNS query logger worker".to_string()),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            max_deliver: 3, // Retry up to 3 times
            ..Default::default()
        }
    }

    /// Email sender consumer
    pub fn email_sender_consumer() -> jetstream::consumer::pull::Config {
        jetstream::consumer::pull::Config {
            durable_name: Some("email-sender".to_string()),
            description: Some("Email sending worker".to_string()),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            max_deliver: 5, // Retry emails up to 5 times
            ..Default::default()
        }
    }

    /// Blocklist updater consumer
    pub fn blocklist_updater_consumer() -> jetstream::consumer::pull::Config {
        jetstream::consumer::pull::Config {
            durable_name: Some("blocklist-updater".to_string()),
            description: Some("Blocklist update worker".to_string()),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            max_deliver: 3,
            ..Default::default()
        }
    }

    /// Analytics aggregator consumer
    pub fn analytics_aggregator_consumer() -> jetstream::consumer::pull::Config {
        jetstream::consumer::pull::Config {
            durable_name: Some("analytics-aggregator".to_string()),
            description: Some("Analytics aggregation worker".to_string()),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            max_deliver: 3,
            ..Default::default()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DnsQueryType, DnsResultType};

    #[test]
    fn test_dns_query_log_message_serialization() {
        let message = messages::DnsQueryLogMessage {
            tenant_id: uuid::Uuid::new_v4(),
            config_id: Some(uuid::Uuid::new_v4()),
            query_timestamp: chrono::Utc::now(),
            query_domain_name: "example.com".to_string(),
            query_type: DnsQueryType::A,
            query_result_type: DnsResultType::Allowed,
            response_ip_address: Some("93.184.216.34".to_string()),
            query_latency_ms: Some(42),
            query_source_protocol: Some("DoH".to_string()),
            query_source_ip: Some("192.168.1.100".to_string()),
        };

        let json = serde_json::to_string(&message).expect("Serialization failed");
        let deserialized: messages::DnsQueryLogMessage = 
            serde_json::from_str(&json).expect("Deserialization failed");
        
        assert_eq!(message.query_domain_name, deserialized.query_domain_name);
        assert_eq!(message.query_latency_ms, deserialized.query_latency_ms);
    }

    #[test]
    fn test_email_message_serialization() {
        let message = messages::EmailMessage {
            to: "user@example.com".to_string(),
            subject: "Test Subject".to_string(),
            template: "welcome".to_string(),
            variables: serde_json::json!({"name": "John"}),
            priority: messages::EmailPriority::High,
        };

        let json = serde_json::to_string(&message).expect("Serialization failed");
        let deserialized: messages::EmailMessage = 
            serde_json::from_str(&json).expect("Deserialization failed");
        
        assert_eq!(message.to, deserialized.to);
        assert_eq!(message.subject, deserialized.subject);
        assert_eq!(message.template, deserialized.template);
    }

    #[test]
    fn test_email_priority_variants() {
        let priorities = vec![
            messages::EmailPriority::High,
            messages::EmailPriority::Normal,
            messages::EmailPriority::Low,
        ];

        for priority in priorities {
            let json = serde_json::to_string(&priority).expect("Serialization failed");
            let _: messages::EmailPriority = serde_json::from_str(&json)
                .expect("Deserialization failed");
        }
    }

    #[test]
    fn test_blocklist_update_message() {
        let message = messages::BlocklistUpdateMessage {
            source_id: uuid::Uuid::new_v4(),
            source_name: "EasyList".to_string(),
            source_url: "https://easylist.to/easylist/easylist.txt".to_string(),
            source_format: "hostfile".to_string(),
        };

        assert!(!message.source_name.is_empty());
        assert!(message.source_url.starts_with("https://"));
        
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("source_name"));
    }

    #[test]
    fn test_analytics_aggregate_message() {
        let message = messages::AnalyticsAggregateMessage {
            tenant_id: Some(uuid::Uuid::new_v4()),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now() + chrono::Duration::hours(1),
            aggregation_type: messages::AggregationType::Hourly,
        };

        assert!(message.tenant_id.is_some());
        assert!(message.end_time > message.start_time);
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: messages::AnalyticsAggregateMessage = 
            serde_json::from_str(&json).unwrap();
        assert_eq!(message.tenant_id, deserialized.tenant_id);
    }

    #[test]
    fn test_aggregation_type_variants() {
        let types = vec![
            messages::AggregationType::Hourly,
            messages::AggregationType::Daily,
            messages::AggregationType::Weekly,
            messages::AggregationType::Monthly,
        ];

        for agg_type in types {
            let json = serde_json::to_string(&agg_type).unwrap();
            let _: messages::AggregationType = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_dns_query_log_with_optional_fields() {
        let message = messages::DnsQueryLogMessage {
            tenant_id: uuid::Uuid::new_v4(),
            config_id: None,
            query_timestamp: chrono::Utc::now(),
            query_domain_name: "blocked.com".to_string(),
            query_type: DnsQueryType::A,
            query_result_type: DnsResultType::Blocked,
            response_ip_address: None,
            query_latency_ms: None,
            query_source_protocol: None,
            query_source_ip: None,
        };

        assert!(message.config_id.is_none());
        assert!(message.response_ip_address.is_none());
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: messages::DnsQueryLogMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message.query_domain_name, deserialized.query_domain_name);
    }

    #[test]
    fn test_email_message_with_variables() {
        let vars = serde_json::json!({
            "otp_code": "123456",
            "expires_in": 600,
            "user_email": "user@example.com"
        });

        let message = messages::EmailMessage {
            to: "user@example.com".to_string(),
            subject: "Your OTP Code".to_string(),
            template: "otp_email".to_string(),
            variables: vars.clone(),
            priority: messages::EmailPriority::High,
        };

        assert_eq!(message.variables["otp_code"], "123456");
        assert_eq!(message.variables["expires_in"], 600);
    }

    #[test]
    fn test_stream_configs_have_valid_names() {
        let dns_stream = streams::dns_query_log_stream();
        assert_eq!(dns_stream.name, "DNS_QUERIES");
        
        let email_stream = streams::email_stream();
        assert_eq!(email_stream.name, "EMAILS");
        
        let blocklist_stream = streams::blocklist_update_stream();
        assert_eq!(blocklist_stream.name, "BLOCKLIST_UPDATES");
        
        let analytics_stream = streams::analytics_stream();
        assert_eq!(analytics_stream.name, "ANALYTICS");
    }

    #[test]
    fn test_stream_configs_have_subjects() {
        let dns_stream = streams::dns_query_log_stream();
        assert!(!dns_stream.subjects.is_empty());
        assert!(dns_stream.subjects.contains(&"dns.query.log".to_string()));
        
        let email_stream = streams::email_stream();
        assert!(email_stream.subjects.contains(&"email.send".to_string()));
    }

    #[test]
    fn test_consumer_configs_have_durable_names() {
        let dns_consumer = consumers::dns_logger_consumer();
        assert_eq!(dns_consumer.durable_name, Some("dns-logger".to_string()));
        
        let email_consumer = consumers::email_sender_consumer();
        assert_eq!(email_consumer.durable_name, Some("email-sender".to_string()));
    }
}
use anyhow::Result;
use async_nats::jetstream::consumer::PullConsumer;
use common::nats_client::messages::DnsQueryLogMessage;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::WorkerState;

const BATCH_SIZE: usize = 100;
const BATCH_TIMEOUT_SECS: u64 = 5;
const MAX_RETRIES: u32 = 3;

/// DNS query logger worker
/// Consumes messages from NATS dns.query.log and batch-inserts into PostgreSQL
pub async fn run(state: Arc<WorkerState>) -> Result<()> {
    info!("DNS Logger worker starting...");

    // Get or create consumer
    let stream = state
        .nats
        .jetstream()
        .get_stream("DNS_QUERIES")
        .await?;

    let consumer: PullConsumer = stream
        .get_or_create_consumer(
            "dns-logger",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("dns-logger".to_string()),
                description: Some("DNS query logger worker".to_string()),
                ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                max_deliver: MAX_RETRIES as i64,
                ..Default::default()
            },
        )
        .await?;

    info!("DNS Logger consumer created, starting message processing...");

    let mut messages = consumer
        .stream()
        .max_messages_per_batch(BATCH_SIZE)
        .messages()
        .await?;

    let mut batch: Vec<DnsQueryLogMessage> = Vec::with_capacity(BATCH_SIZE);
    let mut last_flush = tokio::time::Instant::now();

    // Process messages in batches
    loop {
        tokio::select! {
            // Receive message
            Some(message) = messages.next() => {
                match message {
                    Ok(msg) => {
                        // Parse message
                        match serde_json::from_slice::<DnsQueryLogMessage>(&msg.payload) {
                            Ok(log_msg) => {
                                debug!("Received DNS query log: {} for tenant {}",
                                    log_msg.query_domain_name, log_msg.tenant_id);
                                batch.push(log_msg);

                                // Flush batch if full
                                if batch.len() >= BATCH_SIZE {
                                    flush_batch(&state, &mut batch, &mut last_flush).await;
                                }

                                // Acknowledge message
                                if let Err(e) = msg.ack().await {
                                    error!("Failed to acknowledge message: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse DNS query log message: {}", e);
                                // Acknowledge to avoid reprocessing bad messages
                                let _ = msg.ack().await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            // Timeout - flush partial batch
            _ = tokio::time::sleep(Duration::from_secs(BATCH_TIMEOUT_SECS)) => {
                if !batch.is_empty() && last_flush.elapsed().as_secs() >= BATCH_TIMEOUT_SECS {
                    flush_batch(&state, &mut batch, &mut last_flush).await;
                }
            }
        }
    }
}

/// Flush a batch of DNS query logs to PostgreSQL
async fn flush_batch(
    state: &WorkerState,
    batch: &mut Vec<DnsQueryLogMessage>,
    last_flush: &mut tokio::time::Instant,
) {
    if batch.is_empty() {
        return;
    }

    let batch_size = batch.len();
    info!("Flushing batch of {} DNS query logs to database", batch_size);

    let start = tokio::time::Instant::now();

    // Insert batch into database
    match insert_batch(&state.database, batch).await {
        Ok(()) => {
            let duration = start.elapsed();
            info!(
                "Successfully inserted {} DNS query logs in {:?} ({:.2} logs/sec)",
                batch_size,
                duration,
                batch_size as f64 / duration.as_secs_f64()
            );

            // Record metrics
            common::metrics::BACKGROUND_JOB_DURATION
                .with_label_values(&["dns_logger"])
                .observe(duration.as_secs_f64());
        }
        Err(e) => {
            error!("Failed to insert DNS query log batch: {}", e);
            warn!("Discarding batch of {} logs due to persistent errors", batch_size);
        }
    }

    // Clear batch and update timestamp
    batch.clear();
    *last_flush = tokio::time::Instant::now();
}

/// Insert a batch of DNS query logs into PostgreSQL
async fn insert_batch(database: &common::database::Database, batch: &[DnsQueryLogMessage]) -> Result<()> {
    // Build bulk insert query
    let mut query_builder = sqlx::QueryBuilder::new(
        r#"
        INSERT INTO dns_query_log (
            tenant_id,
            config_id,
            query_domain_name,
            query_type,
            query_result_type,
            response_ip_address,
            query_latency_ms,
            query_source_protocol,
            query_source_ip,
            query_timestamp
        )
        "#
    );

    query_builder.push_values(batch.iter(), |mut b, log| {
        b.push_bind(log.tenant_id)
            .push_bind(log.config_id)
            .push_bind(&log.query_domain_name)
            .push_bind(log.query_type)
            .push_bind(log.query_result_type)
            .push_bind(log.response_ip_address.as_deref())
            .push_bind(log.query_latency_ms)
            .push_bind(log.query_source_protocol.as_deref())
            .push_bind(log.query_source_ip.as_deref())
            .push_bind(log.query_timestamp);
    });

    let query = query_builder.build();

    // Execute bulk insert
    query.execute(database.pool()).await?;

    Ok(())
}

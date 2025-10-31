use anyhow::Result;
use async_nats::jetstream::consumer::PullConsumer;
use common::nats_client::messages::EmailMessage;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::WorkerState;

const MAX_RETRIES: u32 = 5;
const RETRY_DELAY_SECS: u64 = 2;

/// Email sender worker
/// Consumes messages from NATS email.send and sends emails via SMTP/API
pub async fn run(state: Arc<WorkerState>) -> Result<()> {
    info!("Email Sender worker starting...");

    // Get or create consumer
    let stream = state
        .nats
        .jetstream()
        .get_stream("EMAILS")
        .await?;

    let consumer: PullConsumer = stream
        .get_or_create_consumer(
            "email-sender",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("email-sender".to_string()),
                description: Some("Email sending worker".to_string()),
                ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                max_deliver: MAX_RETRIES as i64,
                ..Default::default()
            },
        )
        .await?;

    info!("Email Sender consumer created, starting message processing...");

    let mut messages = consumer
        .stream()
        .max_messages_per_batch(10)
        .messages()
        .await?;

    // Process messages one at a time (emails are not batched)
    while let Some(message) = messages.next().await {
        match message {
            Ok(msg) => {
                // Parse message
                match serde_json::from_slice::<EmailMessage>(&msg.payload) {
                    Ok(email_msg) => {
                        debug!("Received email request to: {}", email_msg.to);

                        // Send email with retry logic
                        let result = send_email_with_retry(&email_msg, MAX_RETRIES).await;

                        match result {
                            Ok(()) => {
                                info!("Email sent successfully to: {}", email_msg.to);
                                // Acknowledge message
                                if let Err(e) = msg.ack().await {
                                    error!("Failed to acknowledge message: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to send email to {}: {}", email_msg.to, e);
                                // Nack message for retry
                                if let Err(e) = msg.ack_with(async_nats::jetstream::AckKind::Nak(Some(
                                    Duration::from_secs(RETRY_DELAY_SECS)
                                ))).await {
                                    error!("Failed to nack message: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse email message: {}", e);
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

    Ok(())
}

/// Send email with retry logic
async fn send_email_with_retry(email: &EmailMessage, max_retries: u32) -> Result<()> {
    for attempt in 1..=max_retries {
        match send_email(email).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt < max_retries {
                    warn!(
                        "Email send attempt {}/{} failed: {}. Retrying in {}s...",
                        attempt, max_retries, e, RETRY_DELAY_SECS
                    );
                    tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
                } else {
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
}

/// Send email via SMTP or API
async fn send_email(email: &EmailMessage) -> Result<()> {
    // TODO: Implement actual email sending
    // Options:
    // 1. AWS SES via aws-sdk-ses
    // 2. Resend API via reqwest
    // 3. SendGrid API via reqwest
    // 4. SMTP via lettre crate

    info!(
        "Sending email (STUB): to={}, subject={}, template={}",
        email.to, email.subject, email.template
    );

    // For now, just log the email
    debug!("Email variables: {:?}", email.variables);

    // Simulate email sending delay
    tokio::time::sleep(Duration::from_millis(100)).await;

    // TODO: Replace with actual email sending implementation
    // Example with Resend:
    // let client = reqwest::Client::new();
    // let response = client
    //     .post("https://api.resend.com/emails")
    //     .header("Authorization", format!("Bearer {}", api_key))
    //     .json(&json!({
    //         "from": "MightyDNS <noreply@mightydns.com>",
    //         "to": [email.to],
    //         "subject": email.subject,
    //         "html": render_template(&email.template, &email.variables)?
    //     }))
    //     .send()
    //     .await?;

    Ok(())
}

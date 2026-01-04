//! Webhook delivery service with HMAC-SHA256 signing and exponential backoff retry.

use std::time::Duration;

use chrono::{DateTime, Utc};
use db::models::{
    webhook::{Webhook, WebhookEvent},
    webhook_delivery::{CreateWebhookDelivery, WebhookDelivery},
};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::SqlitePool;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Maximum number of delivery attempts before marking as permanently failed.
const MAX_ATTEMPTS: i64 = 7;

/// Retry delays in seconds: 1s, 5s, 30s, 5m, 30m, 2h, 8h
const RETRY_DELAYS_SECS: [u64; 7] = [
    1,           // Attempt 1: 1 second
    5,           // Attempt 2: 5 seconds
    30,          // Attempt 3: 30 seconds
    5 * 60,      // Attempt 4: 5 minutes
    30 * 60,     // Attempt 5: 30 minutes
    2 * 60 * 60, // Attempt 6: 2 hours
    8 * 60 * 60, // Attempt 7: 8 hours
];

/// HTTP request timeout for webhook delivery.
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Errors that can occur during webhook delivery.
#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("network error: {0}")]
    Network(String),

    #[error("timeout")]
    Timeout,

    #[error("http error {status}: {body}")]
    Http { status: u16, body: String },

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("webhook not found: {0}")]
    NotFound(Uuid),
}

impl WebhookError {
    /// Returns true if this error is transient and should be retried.
    pub fn should_retry(&self) -> bool {
        match self {
            Self::Network(_) | Self::Timeout => true,
            // Retry on 5xx server errors
            Self::Http { status, .. } => (500..=599).contains(status),
            // Don't retry on database errors, serialization errors, or not found
            _ => false,
        }
    }
}

/// Webhook payload structure sent to the endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Event type (e.g., "task_created")
    pub event: String,
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    /// Unique delivery ID for deduplication
    pub delivery_id: Uuid,
    /// Event-specific data
    pub data: serde_json::Value,
}

/// Result of a delivery attempt.
#[derive(Debug)]
pub struct DeliveryResult {
    /// Whether the delivery succeeded
    pub success: bool,
    /// HTTP status code if applicable
    pub status_code: Option<u16>,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of attempts made
    pub attempts: i64,
}

/// Service for delivering webhooks with retry logic.
pub struct WebhookService {
    pool: SqlitePool,
    client: Client,
}

impl WebhookService {
    /// Creates a new WebhookService with the given database pool.
    pub fn new(pool: SqlitePool) -> Self {
        let client = Client::builder()
            .timeout(DELIVERY_TIMEOUT)
            .user_agent(concat!("vibe-kanban-webhook/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");

        Self { pool, client }
    }

    /// Sign a payload with HMAC-SHA256 and return the hex-encoded signature.
    ///
    /// Returns the signature in the format "sha256=<hex>"
    pub fn sign_payload(secret: &str, payload: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let result = mac.finalize();
        format!("sha256={}", hex::encode(result.into_bytes()))
    }

    /// Calculate the next retry delay based on attempt count (0-indexed).
    ///
    /// Returns None if max attempts have been reached.
    pub fn next_retry_delay(attempts: i64) -> Option<Duration> {
        if attempts < 0 || attempts >= MAX_ATTEMPTS {
            return None;
        }
        Some(Duration::from_secs(
            RETRY_DELAYS_SECS[attempts as usize],
        ))
    }

    /// Queue a new webhook delivery for the given event.
    ///
    /// Creates a WebhookDelivery record in pending status.
    pub async fn queue_delivery(
        &self,
        webhook_id: Uuid,
        event: &WebhookEvent,
        data: serde_json::Value,
    ) -> Result<WebhookDelivery, WebhookError> {
        let delivery_id = Uuid::new_v4();
        let payload = WebhookPayload {
            event: event.as_str().to_string(),
            timestamp: Utc::now(),
            delivery_id,
            data,
        };

        let payload_json = serde_json::to_string(&payload)?;

        let create_data = CreateWebhookDelivery {
            webhook_id,
            event_type: event.as_str().to_string(),
            payload: payload_json,
        };

        let delivery = WebhookDelivery::create(&self.pool, &create_data).await?;

        info!(
            webhook_id = %webhook_id,
            delivery_id = %delivery.id,
            event = %event.as_str(),
            "Queued webhook delivery"
        );

        Ok(delivery)
    }

    /// Deliver a single webhook (HTTP POST).
    ///
    /// This makes a single delivery attempt and returns the result.
    /// It does NOT handle retries or status updates - use `process_delivery` for that.
    pub async fn deliver(
        &self,
        webhook: &Webhook,
        delivery: &WebhookDelivery,
    ) -> Result<(), WebhookError> {
        debug!(
            webhook_id = %webhook.id,
            delivery_id = %delivery.id,
            url = %webhook.url,
            "Attempting webhook delivery"
        );

        let signature = Self::sign_payload(&webhook.secret, &delivery.payload);

        let response = self
            .client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-Event", &delivery.event_type)
            .header("X-Webhook-Delivery", delivery.id.to_string())
            .body(delivery.payload.clone())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    WebhookError::Timeout
                } else {
                    WebhookError::Network(e.to_string())
                }
            })?;

        let status = response.status();
        if status.is_success() {
            debug!(
                webhook_id = %webhook.id,
                delivery_id = %delivery.id,
                status = %status.as_u16(),
                "Webhook delivery succeeded"
            );
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(WebhookError::Http {
                status: status.as_u16(),
                body,
            })
        }
    }

    /// Process a single delivery with retry logic.
    ///
    /// This attempts delivery and updates the database status accordingly:
    /// - On success: marks as Success
    /// - On retriable failure with attempts remaining: marks as Retrying with next_retry_at
    /// - On non-retriable failure or max attempts reached: marks as Failed
    pub async fn process_delivery(
        &self,
        webhook: &Webhook,
        delivery: &WebhookDelivery,
    ) -> Result<DeliveryResult, WebhookError> {
        let result = self.deliver(webhook, delivery).await;

        match result {
            Ok(()) => {
                // Mark as successful
                WebhookDelivery::mark_success(&self.pool, delivery.id).await?;
                info!(
                    webhook_id = %webhook.id,
                    delivery_id = %delivery.id,
                    attempts = delivery.attempts + 1,
                    "Webhook delivery succeeded"
                );
                Ok(DeliveryResult {
                    success: true,
                    status_code: Some(200),
                    error: None,
                    attempts: delivery.attempts + 1,
                })
            }
            Err(err) => {
                let error_msg = err.to_string();
                let status_code = match &err {
                    WebhookError::Http { status, .. } => Some(*status),
                    _ => None,
                };

                // Check if we should retry
                let next_attempt = delivery.attempts + 1;
                if err.should_retry() && next_attempt < MAX_ATTEMPTS {
                    // Calculate next retry time
                    let delay = Self::next_retry_delay(next_attempt)
                        .expect("delay should exist for attempts < MAX_ATTEMPTS");
                    let next_retry_at = Utc::now() + chrono::Duration::from_std(delay).unwrap();

                    WebhookDelivery::mark_retrying(&self.pool, delivery.id, &error_msg, next_retry_at)
                        .await?;

                    warn!(
                        webhook_id = %webhook.id,
                        delivery_id = %delivery.id,
                        attempts = next_attempt,
                        next_retry_at = %next_retry_at,
                        error = %error_msg,
                        "Webhook delivery failed, will retry"
                    );

                    Ok(DeliveryResult {
                        success: false,
                        status_code,
                        error: Some(error_msg),
                        attempts: next_attempt,
                    })
                } else {
                    // Mark as permanently failed
                    WebhookDelivery::mark_failed(&self.pool, delivery.id, &error_msg).await?;

                    error!(
                        webhook_id = %webhook.id,
                        delivery_id = %delivery.id,
                        attempts = next_attempt,
                        error = %error_msg,
                        "Webhook delivery permanently failed"
                    );

                    Ok(DeliveryResult {
                        success: false,
                        status_code,
                        error: Some(error_msg),
                        attempts: next_attempt,
                    })
                }
            }
        }
    }

    /// Process all pending deliveries that are ready to be sent.
    ///
    /// This includes:
    /// - Deliveries in Pending status
    /// - Deliveries in Retrying status where next_retry_at <= now
    pub async fn process_pending_deliveries(&self) -> Result<Vec<DeliveryResult>, WebhookError> {
        let deliveries = WebhookDelivery::find_pending_deliveries(&self.pool).await?;

        if deliveries.is_empty() {
            debug!("No pending webhook deliveries to process");
            return Ok(vec![]);
        }

        info!(count = deliveries.len(), "Processing pending webhook deliveries");

        let mut results = Vec::with_capacity(deliveries.len());

        for delivery in deliveries {
            // Fetch the webhook for this delivery
            let webhook = match Webhook::find_by_id(&self.pool, delivery.webhook_id).await? {
                Some(w) if w.is_active => w,
                Some(_) => {
                    // Webhook exists but is inactive - mark delivery as failed
                    WebhookDelivery::mark_failed(
                        &self.pool,
                        delivery.id,
                        "Webhook is inactive",
                    )
                    .await?;
                    warn!(
                        webhook_id = %delivery.webhook_id,
                        delivery_id = %delivery.id,
                        "Skipping delivery for inactive webhook"
                    );
                    continue;
                }
                None => {
                    // Webhook was deleted - mark delivery as failed
                    WebhookDelivery::mark_failed(
                        &self.pool,
                        delivery.id,
                        "Webhook not found",
                    )
                    .await?;
                    warn!(
                        webhook_id = %delivery.webhook_id,
                        delivery_id = %delivery.id,
                        "Skipping delivery for deleted webhook"
                    );
                    continue;
                }
            };

            match self.process_delivery(&webhook, &delivery).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!(
                        delivery_id = %delivery.id,
                        error = %e,
                        "Error processing delivery"
                    );
                }
            }
        }

        Ok(results)
    }

    /// Queue deliveries for all active webhooks subscribed to an event.
    ///
    /// This is a convenience method that:
    /// 1. Finds all active webhooks for the project subscribed to the event
    /// 2. Creates a delivery for each webhook
    pub async fn trigger_event(
        &self,
        project_id: Uuid,
        event: &WebhookEvent,
        data: serde_json::Value,
    ) -> Result<Vec<WebhookDelivery>, WebhookError> {
        let webhooks = Webhook::find_by_project_and_event(&self.pool, project_id, event).await?;

        if webhooks.is_empty() {
            debug!(
                project_id = %project_id,
                event = %event.as_str(),
                "No webhooks subscribed to event"
            );
            return Ok(vec![]);
        }

        info!(
            project_id = %project_id,
            event = %event.as_str(),
            webhook_count = webhooks.len(),
            "Triggering webhook event"
        );

        let mut deliveries = Vec::with_capacity(webhooks.len());
        for webhook in webhooks {
            let delivery = self.queue_delivery(webhook.id, event, data.clone()).await?;
            deliveries.push(delivery);
        }

        Ok(deliveries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_payload() {
        let secret = "test-secret";
        let payload = r#"{"event":"task_created","timestamp":"2026-01-04T12:00:00Z","delivery_id":"550e8400-e29b-41d4-a716-446655440000","data":{"task":{}}}"#;

        let signature = WebhookService::sign_payload(secret, payload);

        // Verify format
        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), 7 + 64); // "sha256=" + 64 hex chars

        // Verify deterministic
        let signature2 = WebhookService::sign_payload(secret, payload);
        assert_eq!(signature, signature2);

        // Verify different secrets produce different signatures
        let signature3 = WebhookService::sign_payload("different-secret", payload);
        assert_ne!(signature, signature3);
    }

    #[test]
    fn test_next_retry_delay() {
        // Test all valid attempts
        assert_eq!(
            WebhookService::next_retry_delay(0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            WebhookService::next_retry_delay(1),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            WebhookService::next_retry_delay(2),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            WebhookService::next_retry_delay(3),
            Some(Duration::from_secs(5 * 60))
        );
        assert_eq!(
            WebhookService::next_retry_delay(4),
            Some(Duration::from_secs(30 * 60))
        );
        assert_eq!(
            WebhookService::next_retry_delay(5),
            Some(Duration::from_secs(2 * 60 * 60))
        );
        assert_eq!(
            WebhookService::next_retry_delay(6),
            Some(Duration::from_secs(8 * 60 * 60))
        );

        // Test out of bounds
        assert_eq!(WebhookService::next_retry_delay(-1), None);
        assert_eq!(WebhookService::next_retry_delay(7), None);
        assert_eq!(WebhookService::next_retry_delay(100), None);
    }

    #[test]
    fn test_webhook_error_should_retry() {
        // Network errors should retry
        assert!(WebhookError::Network("connection refused".to_string()).should_retry());

        // Timeout should retry
        assert!(WebhookError::Timeout.should_retry());

        // 5xx errors should retry
        assert!(WebhookError::Http {
            status: 500,
            body: "Internal Server Error".to_string()
        }
        .should_retry());
        assert!(WebhookError::Http {
            status: 502,
            body: "Bad Gateway".to_string()
        }
        .should_retry());
        assert!(WebhookError::Http {
            status: 503,
            body: "Service Unavailable".to_string()
        }
        .should_retry());
        assert!(WebhookError::Http {
            status: 599,
            body: "".to_string()
        }
        .should_retry());

        // 4xx errors should NOT retry
        assert!(!WebhookError::Http {
            status: 400,
            body: "Bad Request".to_string()
        }
        .should_retry());
        assert!(!WebhookError::Http {
            status: 401,
            body: "Unauthorized".to_string()
        }
        .should_retry());
        assert!(!WebhookError::Http {
            status: 404,
            body: "Not Found".to_string()
        }
        .should_retry());

        // NotFound should NOT retry
        assert!(!WebhookError::NotFound(Uuid::new_v4()).should_retry());
    }

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            event: "task_created".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2026-01-04T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            delivery_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            data: serde_json::json!({
                "task": {
                    "id": "123",
                    "title": "Test Task"
                }
            }),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let parsed: WebhookPayload = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.event, "task_created");
        assert_eq!(
            parsed.delivery_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }
}

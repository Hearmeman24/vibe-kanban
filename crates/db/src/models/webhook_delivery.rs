use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

/// Status of a webhook delivery attempt
#[derive(
    Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS, EnumString, Display, Default,
)]
#[sqlx(type_name = "delivery_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum DeliveryStatus {
    #[default]
    Pending,
    Success,
    Failed,
    Retrying,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub payload: String,
    pub status: DeliveryStatus,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateWebhookDelivery {
    pub webhook_id: Uuid,
    pub event_type: String,
    pub payload: String,
}

impl WebhookDelivery {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"SELECT id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>"
               FROM webhook_deliveries
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_webhook_id(
        pool: &SqlitePool,
        webhook_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"SELECT id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>"
               FROM webhook_deliveries
               WHERE webhook_id = $1
               ORDER BY created_at DESC"#,
            webhook_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all pending or retrying deliveries that are ready to be processed
    /// (either pending, or retrying with next_retry_at <= now)
    pub async fn find_pending_deliveries(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"SELECT id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>"
               FROM webhook_deliveries
               WHERE status = 'pending'
                  OR (status = 'retrying' AND (next_retry_at IS NULL OR next_retry_at <= datetime('now', 'subsec')))
               ORDER BY created_at ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find all retrying deliveries (for monitoring/debugging)
    pub async fn find_retrying_deliveries(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"SELECT id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>"
               FROM webhook_deliveries
               WHERE status = 'retrying'
               ORDER BY next_retry_at ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find deliveries by status
    pub async fn find_by_status(
        pool: &SqlitePool,
        status: DeliveryStatus,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"SELECT id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>"
               FROM webhook_deliveries
               WHERE status = $1
               ORDER BY created_at DESC"#,
            status
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateWebhookDelivery,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            WebhookDelivery,
            r#"INSERT INTO webhook_deliveries (id, webhook_id, event_type, payload)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>""#,
            id,
            data.webhook_id,
            data.event_type,
            data.payload
        )
        .fetch_one(pool)
        .await
    }

    /// Mark a delivery as successfully delivered
    pub async fn mark_success(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"UPDATE webhook_deliveries
               SET status = 'success', delivered_at = datetime('now', 'subsec'), attempts = attempts + 1
               WHERE id = $1
               RETURNING id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>""#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Mark a delivery as failed (no more retries)
    pub async fn mark_failed(
        pool: &SqlitePool,
        id: Uuid,
        error: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"UPDATE webhook_deliveries
               SET status = 'failed', last_error = $2, attempts = attempts + 1
               WHERE id = $1
               RETURNING id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>""#,
            id,
            error
        )
        .fetch_optional(pool)
        .await
    }

    /// Mark a delivery for retry with exponential backoff
    pub async fn mark_retrying(
        pool: &SqlitePool,
        id: Uuid,
        error: &str,
        next_retry_at: DateTime<Utc>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"UPDATE webhook_deliveries
               SET status = 'retrying', last_error = $2, next_retry_at = $3, attempts = attempts + 1
               WHERE id = $1
               RETURNING id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>""#,
            id,
            error,
            next_retry_at
        )
        .fetch_optional(pool)
        .await
    }

    /// Update the delivery status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: DeliveryStatus,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            WebhookDelivery,
            r#"UPDATE webhook_deliveries
               SET status = $2
               WHERE id = $1
               RETURNING id as "id!: Uuid", webhook_id as "webhook_id!: Uuid", event_type, payload, status as "status!: DeliveryStatus", attempts as "attempts!: i64", last_error, next_retry_at as "next_retry_at: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>", delivered_at as "delivered_at: DateTime<Utc>""#,
            id,
            status
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM webhook_deliveries WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_by_webhook_id(
        pool: &SqlitePool,
        webhook_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM webhook_deliveries WHERE webhook_id = $1",
            webhook_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Delete old successful/failed deliveries for cleanup
    /// Keeps deliveries newer than the specified number of days
    pub async fn cleanup_old_deliveries(
        pool: &SqlitePool,
        days_to_keep: i64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"DELETE FROM webhook_deliveries
               WHERE status IN ('success', 'failed')
                 AND created_at < datetime('now', '-' || $1 || ' days')"#,
            days_to_keep
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

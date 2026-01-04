use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

/// Webhook event types that can trigger deliveries
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS, EnumString, Display)]
#[sqlx(type_name = "webhook_event", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WebhookEvent {
    TaskCreated,
    TaskUpdated,
    TaskCompleted,
    WorkspaceStarted,
}

impl WebhookEvent {
    /// Convert event to its string representation for JSON storage
    pub fn as_str(&self) -> &'static str {
        match self {
            WebhookEvent::TaskCreated => "task_created",
            WebhookEvent::TaskUpdated => "task_updated",
            WebhookEvent::TaskCompleted => "task_completed",
            WebhookEvent::WorkspaceStarted => "workspace_started",
        }
    }

    /// Parse event from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "task_created" => Some(WebhookEvent::TaskCreated),
            "task_updated" => Some(WebhookEvent::TaskUpdated),
            "task_completed" => Some(WebhookEvent::TaskCompleted),
            "workspace_started" => Some(WebhookEvent::WorkspaceStarted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Webhook {
    pub id: Uuid,
    pub project_id: Uuid,
    pub url: String,
    pub secret: String,
    /// JSON array of event types, e.g., ["task_created", "task_updated"]
    pub events: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateWebhook {
    pub project_id: Uuid,
    pub url: String,
    pub secret: String,
    /// List of event types to subscribe to
    pub events: Vec<WebhookEvent>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct UpdateWebhook {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<WebhookEvent>>,
    pub is_active: Option<bool>,
}

impl Webhook {
    /// Parse the events JSON array into a Vec of WebhookEvents
    pub fn get_events(&self) -> Vec<WebhookEvent> {
        serde_json::from_str::<Vec<String>>(&self.events)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| WebhookEvent::from_str(&s))
            .collect()
    }

    /// Check if this webhook is subscribed to a specific event
    pub fn is_subscribed_to(&self, event: &WebhookEvent) -> bool {
        self.get_events().contains(event)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Webhook,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", url, secret, events, is_active as "is_active!: bool", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM webhooks
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Webhook,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", url, secret, events, is_active as "is_active!: bool", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM webhooks
               WHERE project_id = $1
               ORDER BY created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all active webhooks for a project that are subscribed to a specific event type
    pub async fn find_by_project_and_event(
        pool: &SqlitePool,
        project_id: Uuid,
        event: &WebhookEvent,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let event_str = event.as_str();
        // Use LIKE to search for the event in the JSON array
        let pattern = format!("%\"{}%", event_str);
        sqlx::query_as!(
            Webhook,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", url, secret, events, is_active as "is_active!: bool", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM webhooks
               WHERE project_id = $1
                 AND is_active = 1
                 AND events LIKE $2
               ORDER BY created_at DESC"#,
            project_id,
            pattern
        )
        .fetch_all(pool)
        .await
    }

    /// Find all active webhooks across all projects subscribed to a specific event type
    pub async fn find_all_by_event(
        pool: &SqlitePool,
        event: &WebhookEvent,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let event_str = event.as_str();
        let pattern = format!("%\"{}%", event_str);
        sqlx::query_as!(
            Webhook,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", url, secret, events, is_active as "is_active!: bool", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM webhooks
               WHERE is_active = 1
                 AND events LIKE $1
               ORDER BY created_at DESC"#,
            pattern
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateWebhook) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let events_json = serde_json::to_string(
            &data
                .events
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize events: {}", e)))?;

        sqlx::query_as!(
            Webhook,
            r#"INSERT INTO webhooks (id, project_id, url, secret, events)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", url, secret, events, is_active as "is_active!: bool", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.project_id,
            data.url,
            data.secret,
            events_json
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateWebhook,
    ) -> Result<Option<Self>, sqlx::Error> {
        // First fetch the existing webhook
        let existing = Self::find_by_id(pool, id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let url = data.url.as_ref().unwrap_or(&existing.url);
        let secret = data.secret.as_ref().unwrap_or(&existing.secret);
        let is_active = data.is_active.unwrap_or(existing.is_active);

        let events_json = if let Some(ref events) = data.events {
            serde_json::to_string(&events.iter().map(|e| e.as_str()).collect::<Vec<_>>())
                .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize events: {}", e)))?
        } else {
            existing.events.clone()
        };

        sqlx::query_as!(
            Webhook,
            r#"UPDATE webhooks
               SET url = $2, secret = $3, events = $4, is_active = $5, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", url, secret, events, is_active as "is_active!: bool", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            url,
            secret,
            events_json,
            is_active
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM webhooks WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM webhooks WHERE project_id = $1", project_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Set the active status of a webhook
    pub async fn set_active(
        pool: &SqlitePool,
        id: Uuid,
        is_active: bool,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Webhook,
            r#"UPDATE webhooks
               SET is_active = $2, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", url, secret, events, is_active as "is_active!: bool", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            is_active
        )
        .fetch_optional(pool)
        .await
    }
}

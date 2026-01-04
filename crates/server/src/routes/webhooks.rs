use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::{
    webhook::{CreateWebhook, UpdateWebhook, Webhook, WebhookEvent},
    webhook_delivery::WebhookDelivery,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_webhook_middleware};

/// Request body for creating a new webhook subscription
#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateWebhookRequest {
    /// The URL to send webhook payloads to
    pub url: String,
    /// List of event types to subscribe to
    pub events: Vec<WebhookEvent>,
    /// Optional secret for signing payloads. Auto-generated if not provided.
    pub secret: Option<String>,
}

/// Request body for updating a webhook
#[derive(Debug, Deserialize, Serialize, TS)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<WebhookEvent>>,
    pub is_active: Option<bool>,
}

/// Response containing webhook data with parsed events
#[derive(Debug, Serialize, TS)]
pub struct WebhookResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: Vec<WebhookEvent>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Webhook> for WebhookResponse {
    fn from(webhook: Webhook) -> Self {
        WebhookResponse {
            id: webhook.id,
            project_id: webhook.project_id,
            url: webhook.url.clone(),
            secret: webhook.secret.clone(),
            events: webhook.get_events(),
            is_active: webhook.is_active,
            created_at: webhook.created_at,
            updated_at: webhook.updated_at,
        }
    }
}

/// Query parameters for listing webhooks by project
#[derive(Debug, Deserialize)]
pub struct WebhookProjectQuery {
    pub project_id: Uuid,
}

/// Query parameters for listing deliveries with optional pagination
#[derive(Debug, Deserialize)]
pub struct DeliveryListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Response for the test webhook endpoint
#[derive(Debug, Serialize, TS)]
pub struct TestWebhookResponse {
    pub message: String,
}

/// Validates a URL format for webhook subscriptions
fn validate_webhook_url(url: &str) -> Result<(), ApiError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(ApiError::BadRequest("Webhook URL cannot be empty".to_string()));
    }

    // Parse the URL to validate format
    match url::Url::parse(url) {
        Ok(parsed) => {
            // Only allow http and https schemes
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                return Err(ApiError::BadRequest(
                    "Webhook URL must use http or https scheme".to_string(),
                ));
            }
            Ok(())
        }
        Err(_) => Err(ApiError::BadRequest(
            "Invalid webhook URL format".to_string(),
        )),
    }
}

/// Generates a random secret for webhook signing
fn generate_webhook_secret() -> String {
    // Generate a UUID and convert to hex string for the secret
    Uuid::new_v4().to_string().replace("-", "")
}

/// Create a new webhook subscription for a project
/// POST /api/projects/{project_id}/webhooks
pub async fn create_webhook(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CreateWebhookRequest>,
) -> Result<ResponseJson<ApiResponse<WebhookResponse>>, ApiError> {
    // Validate URL format
    validate_webhook_url(&payload.url)?;

    // Validate non-empty events
    if payload.events.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one event type must be specified".to_string(),
        ));
    }

    // Generate secret if not provided
    let secret = payload.secret.unwrap_or_else(generate_webhook_secret);

    let create_data = CreateWebhook {
        project_id,
        url: payload.url.trim().to_string(),
        secret,
        events: payload.events,
    };

    let webhook = Webhook::create(&deployment.db().pool, &create_data).await?;

    tracing::info!(
        "Created webhook {} for project {} with URL: {}",
        webhook.id,
        project_id,
        webhook.url
    );

    Ok(ResponseJson(ApiResponse::success(WebhookResponse::from(webhook))))
}

/// List all webhooks for a project
/// GET /api/projects/{project_id}/webhooks
pub async fn list_webhooks(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<WebhookResponse>>>, ApiError> {
    let webhooks = Webhook::find_by_project_id(&deployment.db().pool, project_id).await?;

    let response: Vec<WebhookResponse> = webhooks.into_iter().map(WebhookResponse::from).collect();

    Ok(ResponseJson(ApiResponse::success(response)))
}

/// Get webhook details by ID
/// GET /api/webhooks/{webhook_id}
pub async fn get_webhook(
    Extension(webhook): Extension<Webhook>,
) -> Result<ResponseJson<ApiResponse<WebhookResponse>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(WebhookResponse::from(webhook))))
}

/// Update a webhook subscription
/// PUT /api/webhooks/{webhook_id}
pub async fn update_webhook(
    Extension(existing_webhook): Extension<Webhook>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateWebhookRequest>,
) -> Result<ResponseJson<ApiResponse<WebhookResponse>>, ApiError> {
    // Validate URL if provided
    if let Some(ref url) = payload.url {
        validate_webhook_url(url)?;
    }

    // Validate events if provided (must not be empty)
    if let Some(ref events) = payload.events {
        if events.is_empty() {
            return Err(ApiError::BadRequest(
                "Events list cannot be empty".to_string(),
            ));
        }
    }

    let update_data = UpdateWebhook {
        url: payload.url.map(|u| u.trim().to_string()),
        secret: payload.secret,
        events: payload.events,
        is_active: payload.is_active,
    };

    let updated_webhook = Webhook::update(&deployment.db().pool, existing_webhook.id, &update_data)
        .await?
        .ok_or_else(|| ApiError::Database(sqlx::Error::RowNotFound))?;

    tracing::info!("Updated webhook {}", existing_webhook.id);

    Ok(ResponseJson(ApiResponse::success(WebhookResponse::from(updated_webhook))))
}

/// Delete a webhook subscription
/// DELETE /api/webhooks/{webhook_id}
pub async fn delete_webhook(
    Extension(webhook): Extension<Webhook>,
    State(deployment): State<DeploymentImpl>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<()>>), ApiError> {
    // Delete associated deliveries first
    let deliveries_deleted = WebhookDelivery::delete_by_webhook_id(&deployment.db().pool, webhook.id).await?;

    // Delete the webhook
    let rows_affected = Webhook::delete(&deployment.db().pool, webhook.id).await?;

    if rows_affected == 0 {
        return Err(ApiError::Database(sqlx::Error::RowNotFound));
    }

    tracing::info!(
        "Deleted webhook {} and {} associated deliveries",
        webhook.id,
        deliveries_deleted
    );

    Ok((StatusCode::OK, ResponseJson(ApiResponse::success(()))))
}

/// List delivery history for a webhook
/// GET /api/webhooks/{webhook_id}/deliveries
pub async fn list_webhook_deliveries(
    Extension(webhook): Extension<Webhook>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<DeliveryListQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<WebhookDelivery>>>, ApiError> {
    let mut deliveries = WebhookDelivery::find_by_webhook_id(&deployment.db().pool, webhook.id).await?;

    // Apply offset
    let offset = query.offset.unwrap_or(0) as usize;
    if offset > 0 && offset < deliveries.len() {
        deliveries = deliveries.into_iter().skip(offset).collect();
    } else if offset >= deliveries.len() {
        deliveries = Vec::new();
    }

    // Apply limit
    if let Some(limit) = query.limit {
        deliveries.truncate(limit as usize);
    }

    Ok(ResponseJson(ApiResponse::success(deliveries)))
}

/// Send a test webhook (placeholder implementation)
/// POST /api/webhooks/{webhook_id}/test
pub async fn test_webhook(
    Extension(webhook): Extension<Webhook>,
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TestWebhookResponse>>, ApiError> {
    // This is a placeholder implementation
    // Full implementation will be done in P3-3 (webhook delivery engine)

    if !webhook.is_active {
        return Err(ApiError::BadRequest(
            "Cannot test an inactive webhook. Please activate it first.".to_string(),
        ));
    }

    tracing::info!(
        "Test webhook triggered for webhook {} (URL: {}). Full implementation pending.",
        webhook.id,
        webhook.url
    );

    Ok(ResponseJson(ApiResponse::success(TestWebhookResponse {
        message: "Test webhook queued. Full delivery implementation pending.".to_string(),
    })))
}

/// Build the webhook router
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Routes that operate on a specific webhook (with middleware to load webhook)
    let webhook_id_router = Router::new()
        .route("/", get(get_webhook).put(update_webhook).delete(delete_webhook))
        .route("/deliveries", get(list_webhook_deliveries))
        .route("/test", post(test_webhook))
        .layer(from_fn_with_state(deployment.clone(), load_webhook_middleware));

    // Top-level webhook routes (for accessing webhooks by ID)
    let webhooks_router = Router::new()
        .nest("/{webhook_id}", webhook_id_router);

    Router::new().nest("/webhooks", webhooks_router)
}

/// Build the project-scoped webhook router (nested under /projects/{project_id})
pub fn project_webhooks_router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/webhooks", get(list_webhooks).post(create_webhook))
}

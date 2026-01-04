use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::task_history::TaskHistory;
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn get_task_history(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskHistory>>>, ApiError> {
    let history = TaskHistory::find_by_task_id(&deployment.db().pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(history)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/tasks/{task_id}/history",
        Router::new().route("/", get(get_task_history)),
    )
}

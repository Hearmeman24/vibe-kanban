use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::task_comment::{CreateTaskComment, TaskComment};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn get_task_comments(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskComment>>>, ApiError> {
    let comments = TaskComment::find_by_task_id(&deployment.db().pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(comments)))
}

pub async fn create_task_comment(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
    Json(mut payload): Json<CreateTaskComment>,
) -> Result<ResponseJson<ApiResponse<TaskComment>>, ApiError> {
    // Ensure task_id in path matches payload
    payload.task_id = task_id;

    let comment = TaskComment::create(&deployment.db().pool, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(comment)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/tasks/{task_id}/comments",
        Router::new().route("/", get(get_task_comments).post(create_task_comment)),
    )
}

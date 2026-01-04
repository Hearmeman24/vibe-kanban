use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskHistory {
    pub id: Uuid,
    pub task_id: Uuid,
    pub field_changed: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateTaskHistory {
    pub task_id: Uuid,
    pub field_changed: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: String,
}

impl TaskHistory {
    pub async fn find_by_task_id(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskHistory,
            r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", field_changed, old_value, new_value, changed_by, changed_at as "changed_at!: DateTime<Utc>"
               FROM task_history
               WHERE task_id = $1
               ORDER BY changed_at ASC"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskHistory,
            r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", field_changed, old_value, new_value, changed_by, changed_at as "changed_at!: DateTime<Utc>"
               FROM task_history
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateTaskHistory) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TaskHistory,
            r#"INSERT INTO task_history (id, task_id, field_changed, old_value, new_value, changed_by)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid", task_id as "task_id!: Uuid", field_changed, old_value, new_value, changed_by, changed_at as "changed_at!: DateTime<Utc>""#,
            id,
            data.task_id,
            data.field_changed,
            data.old_value,
            data.new_value,
            data.changed_by
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete_by_task_id(pool: &SqlitePool, task_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_history WHERE task_id = $1", task_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

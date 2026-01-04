use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskComment {
    pub id: Uuid,
    pub task_id: Uuid,
    pub content: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateTaskComment {
    pub task_id: Uuid,
    pub content: String,
    pub author: String,
}

impl TaskComment {
    pub async fn find_by_task_id(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskComment,
            r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", content, author, created_at as "created_at!: DateTime<Utc>"
               FROM task_comments
               WHERE task_id = $1
               ORDER BY created_at ASC"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskComment,
            r#"SELECT id as "id!: Uuid", task_id as "task_id!: Uuid", content, author, created_at as "created_at!: DateTime<Utc>"
               FROM task_comments
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateTaskComment) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TaskComment,
            r#"INSERT INTO task_comments (id, task_id, content, author)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid", task_id as "task_id!: Uuid", content, author, created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.task_id,
            data.content,
            data.author
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_comments WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_by_task_id(pool: &SqlitePool, task_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_comments WHERE task_id = $1", task_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

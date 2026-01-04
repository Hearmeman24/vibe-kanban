use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

use super::{project::Project, workspace::Workspace};

/// Represents a single entry in the agent metadata history for a task.
/// Tracks which agent performed what action on the task and when.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentMetadataEntry {
    /// The name of the agent (e.g., "Ferris", "Miley", "Bree")
    pub agent_name: String,
    /// The action performed: "started", "completed", "updated", "commented"
    pub action: String,
    /// ISO 8601 timestamp when the action occurred
    pub timestamp: String,
    /// Optional summary of what the agent did
    pub summary: Option<String>,
}

impl AgentMetadataEntry {
    /// Create a new AgentMetadataEntry with the current timestamp
    pub fn new(agent_name: String, action: String, summary: Option<String>) -> Self {
        Self {
            agent_name,
            action,
            timestamp: Utc::now().to_rfc3339(),
            summary,
        }
    }
}

#[derive(
    Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display, Default,
)]
#[sqlx(type_name = "task_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    InReview,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid, // Foreign key to Project
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub parent_workspace_id: Option<Uuid>, // Foreign key to parent Workspace
    pub shared_task_id: Option<Uuid>,
    pub assignee: Option<String>,
    /// JSON-serialized array of AgentMetadataEntry for tracking agent activity
    pub agent_metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskWithAttemptStatus {
    #[serde(flatten)]
    #[ts(flatten)]
    pub task: Task,
    pub has_in_progress_attempt: bool,
    pub last_attempt_failed: bool,
    pub executor: String,
}

impl std::ops::Deref for TaskWithAttemptStatus {
    type Target = Task;
    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl std::ops::DerefMut for TaskWithAttemptStatus {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.task
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskRelationships {
    pub parent_task: Option<Task>, // The task that owns the parent workspace
    pub current_workspace: Workspace, // The workspace we're viewing
    pub children: Vec<Task>,       // Tasks created from this workspace
}

/// Simplified task relationships without requiring a workspace reference.
/// Used by MCP tools to query relationships by task_id directly.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskRelationshipsSimple {
    pub current_task: Task,        // The task we're querying relationships for
    pub parent_task: Option<Task>, // The task that spawned this task (if any)
    pub children: Vec<Task>,       // Tasks spawned by this task's workspaces
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateTask {
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub parent_workspace_id: Option<Uuid>,
    pub image_ids: Option<Vec<Uuid>>,
    pub shared_task_id: Option<Uuid>,
}

impl CreateTask {
    pub fn from_title_description(
        project_id: Uuid,
        title: String,
        description: Option<String>,
    ) -> Self {
        Self {
            project_id,
            title,
            description,
            status: Some(TaskStatus::Todo),
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
        }
    }

    pub fn from_shared_task(
        project_id: Uuid,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        shared_task_id: Uuid,
    ) -> Self {
        Self {
            project_id,
            title,
            description,
            status: Some(status),
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: Some(shared_task_id),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub parent_workspace_id: Option<Uuid>,
    pub image_ids: Option<Vec<Uuid>>,
    pub assignee: Option<String>,
}

impl Task {
    pub fn to_prompt(&self) -> String {
        if let Some(description) = self.description.as_ref().filter(|d| !d.trim().is_empty()) {
            format!("{}\n\n{}", &self.title, description)
        } else {
            self.title.clone()
        }
    }

    pub async fn parent_project(&self, pool: &SqlitePool) -> Result<Option<Project>, sqlx::Error> {
        Project::find_by_id(pool, self.project_id).await
    }

    pub async fn find_by_project_id_with_attempt_status(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<TaskWithAttemptStatus>, sqlx::Error> {
        let records = sqlx::query!(
            r#"SELECT
  t.id                            AS "id!: Uuid",
  t.project_id                    AS "project_id!: Uuid",
  t.title,
  t.description,
  t.status                        AS "status!: TaskStatus",
  t.parent_workspace_id           AS "parent_workspace_id: Uuid",
  t.shared_task_id                AS "shared_task_id: Uuid",
  t.assignee,
  t.created_at                    AS "created_at!: DateTime<Utc>",
  t.updated_at                    AS "updated_at!: DateTime<Utc>",

  CASE WHEN EXISTS (
    SELECT 1
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      JOIN execution_processes ep ON ep.session_id = s.id
     WHERE w.task_id       = t.id
       AND ep.status        = 'running'
       AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     LIMIT 1
  ) THEN 1 ELSE 0 END            AS "has_in_progress_attempt!: i64",

  CASE WHEN (
    SELECT ep.status
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      JOIN execution_processes ep ON ep.session_id = s.id
     WHERE w.task_id       = t.id
     AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     ORDER BY ep.created_at DESC
     LIMIT 1
  ) IN ('failed','killed') THEN 1 ELSE 0 END
                                 AS "last_attempt_failed!: i64",

  ( SELECT s.executor
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      WHERE w.task_id = t.id
     ORDER BY s.created_at DESC
      LIMIT 1
    )                               AS "executor!: String"

FROM tasks t
WHERE t.project_id = $1
ORDER BY t.created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        let tasks = records
            .into_iter()
            .map(|rec| TaskWithAttemptStatus {
                task: Task {
                    id: rec.id,
                    project_id: rec.project_id,
                    title: rec.title,
                    description: rec.description,
                    status: rec.status,
                    parent_workspace_id: rec.parent_workspace_id,
                    shared_task_id: rec.shared_task_id,
                    assignee: rec.assignee,
                    created_at: rec.created_at,
                    updated_at: rec.updated_at,
                },
                has_in_progress_attempt: rec.has_in_progress_attempt != 0,
                last_attempt_failed: rec.last_attempt_failed != 0,
                executor: rec.executor,
            })
            .collect();

        Ok(tasks)
    }

    pub async fn find_by_project_id_advanced(
        pool: &SqlitePool,
        project_id: Uuid,
        statuses: Option<Vec<TaskStatus>>,
        created_after: Option<DateTime<Utc>>,
        created_before: Option<DateTime<Utc>>,
        updated_after: Option<DateTime<Utc>>,
        updated_before: Option<DateTime<Utc>>,
        sort_by: &str,
        sort_order: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<TaskWithAttemptStatus>, sqlx::Error> {
        use sqlx::{QueryBuilder, Row};

        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            r#"SELECT
  t.id,
  t.project_id,
  t.title,
  t.description,
  t.status,
  t.parent_workspace_id,
  t.shared_task_id,
  t.assignee,
  t.created_at,
  t.updated_at,

  CASE WHEN EXISTS (
    SELECT 1
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      JOIN execution_processes ep ON ep.session_id = s.id
     WHERE w.task_id       = t.id
       AND ep.status        = 'running'
       AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     LIMIT 1
  ) THEN 1 ELSE 0 END AS has_in_progress_attempt,

  CASE WHEN (
    SELECT ep.status
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      JOIN execution_processes ep ON ep.session_id = s.id
     WHERE w.task_id       = t.id
     AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     ORDER BY ep.created_at DESC
     LIMIT 1
  ) IN ('failed','killed') THEN 1 ELSE 0 END AS last_attempt_failed,

  COALESCE((
    SELECT s.executor
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      WHERE w.task_id = t.id
     ORDER BY s.created_at DESC
      LIMIT 1
    ), '') AS executor

FROM tasks t
WHERE t.project_id = "#,
        );

        query_builder.push_bind(project_id);

        // Add status filters
        if let Some(ref status_list) = statuses {
            if !status_list.is_empty() {
                query_builder.push(" AND t.status IN (");
                let mut separated = query_builder.separated(", ");
                for status in status_list {
                    separated.push_bind(status);
                }
                separated.push_unseparated(")");
            }
        }

        // Add date filters
        if let Some(created_after) = created_after {
            query_builder.push(" AND t.created_at >= ");
            query_builder.push_bind(created_after);
        }
        if let Some(created_before) = created_before {
            query_builder.push(" AND t.created_at <= ");
            query_builder.push_bind(created_before);
        }
        if let Some(updated_after) = updated_after {
            query_builder.push(" AND t.updated_at >= ");
            query_builder.push_bind(updated_after);
        }
        if let Some(updated_before) = updated_before {
            query_builder.push(" AND t.updated_at <= ");
            query_builder.push_bind(updated_before);
        }

        // Add sorting
        query_builder.push(" ORDER BY t.");
        match sort_by {
            "created_at" => query_builder.push("created_at"),
            "updated_at" => query_builder.push("updated_at"),
            "title" => query_builder.push("title"),
            _ => query_builder.push("created_at"),
        };
        query_builder.push(" ");
        match sort_order {
            "asc" => query_builder.push("ASC"),
            _ => query_builder.push("DESC"),
        };

        // Add pagination
        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit as i64);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset as i64);

        let query = query_builder.build();

        let records = query.fetch_all(pool).await?;

        let tasks = records
            .into_iter()
            .map(|row| {
                let id: Uuid = row.try_get("id").unwrap_or_default();
                let project_id: Uuid = row.try_get("project_id").unwrap_or_default();
                let title: String = row.try_get("title").unwrap_or_default();
                let description: Option<String> = row.try_get("description").ok();
                let status: TaskStatus = row.try_get("status").unwrap_or_default();
                let parent_workspace_id: Option<Uuid> = row.try_get("parent_workspace_id").ok();
                let shared_task_id: Option<Uuid> = row.try_get("shared_task_id").ok();
                let assignee: Option<String> = row.try_get("assignee").ok().flatten();
                let created_at: DateTime<Utc> = row.try_get("created_at").unwrap_or_default();
                let updated_at: DateTime<Utc> = row.try_get("updated_at").unwrap_or_default();
                let has_in_progress_attempt: i64 =
                    row.try_get("has_in_progress_attempt").unwrap_or(0);
                let last_attempt_failed: i64 = row.try_get("last_attempt_failed").unwrap_or(0);
                let executor: String = row.try_get("executor").unwrap_or_default();

                TaskWithAttemptStatus {
                    task: Task {
                        id,
                        project_id,
                        title,
                        description,
                        status,
                        parent_workspace_id,
                        shared_task_id,
                        assignee,
                        created_at,
                        updated_at,
                    },
                    has_in_progress_attempt: has_in_progress_attempt != 0,
                    last_attempt_failed: last_attempt_failed != 0,
                    executor,
                }
            })
            .collect();

        Ok(tasks)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, agent_metadata, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_rowid(pool: &SqlitePool, rowid: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, agent_metadata, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE rowid = $1"#,
            rowid
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_shared_task_id<'e, E>(
        executor: E,
        shared_task_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, agent_metadata, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE shared_task_id = $1
               LIMIT 1"#,
            shared_task_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_all_shared(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, agent_metadata, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE shared_task_id IS NOT NULL"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateTask,
        task_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let status = data.status.clone().unwrap_or_default();
        sqlx::query_as!(
            Task,
            r#"INSERT INTO tasks (id, project_id, title, description, status, parent_workspace_id, shared_task_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, agent_metadata, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            task_id,
            data.project_id,
            data.title,
            data.description,
            status,
            data.parent_workspace_id,
            data.shared_task_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        project_id: Uuid,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        parent_workspace_id: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"UPDATE tasks
               SET title = $3, description = $4, status = $5, parent_workspace_id = $6
               WHERE id = $1 AND project_id = $2
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, agent_metadata, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            title,
            description,
            status,
            parent_workspace_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: TaskStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE tasks SET status = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
            id,
            status
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update the status of multiple tasks at once.
    /// Returns the updated tasks.
    pub async fn bulk_update_status(
        pool: &SqlitePool,
        task_ids: &[Uuid],
        status: TaskStatus,
    ) -> Result<Vec<Task>, sqlx::Error> {
        if task_ids.is_empty() {
            return Ok(Vec::new());
        }

        use sqlx::QueryBuilder;

        // First update the tasks
        let mut update_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "UPDATE tasks SET status = ",
        );
        update_builder.push_bind(&status);
        update_builder.push(", updated_at = CURRENT_TIMESTAMP WHERE id IN (");

        let mut separated = update_builder.separated(", ");
        for id in task_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        update_builder.build().execute(pool).await?;

        // Then fetch and return the updated tasks
        let mut select_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            r#"SELECT id, project_id, title, description, status, parent_workspace_id, shared_task_id, assignee, agent_metadata, created_at, updated_at
               FROM tasks WHERE id IN ("#,
        );

        let mut separated = select_builder.separated(", ");
        for id in task_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        let rows = select_builder.build().fetch_all(pool).await?;

        use sqlx::Row;
        let tasks = rows
            .into_iter()
            .map(|row| {
                Task {
                    id: row.try_get("id").unwrap_or_default(),
                    project_id: row.try_get("project_id").unwrap_or_default(),
                    title: row.try_get("title").unwrap_or_default(),
                    description: row.try_get("description").ok().flatten(),
                    status: row.try_get("status").unwrap_or_default(),
                    parent_workspace_id: row.try_get("parent_workspace_id").ok().flatten(),
                    shared_task_id: row.try_get("shared_task_id").ok().flatten(),
                    assignee: row.try_get("assignee").ok().flatten(),
                    agent_metadata: row.try_get("agent_metadata").ok().flatten(),
                    created_at: row.try_get("created_at").unwrap_or_default(),
                    updated_at: row.try_get("updated_at").unwrap_or_default(),
                }
            })
            .collect();

        Ok(tasks)
    }

    /// Update the parent_workspace_id field for a task
    pub async fn update_parent_workspace_id(
        pool: &SqlitePool,
        task_id: Uuid,
        parent_workspace_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE tasks SET parent_workspace_id = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
            task_id,
            parent_workspace_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update the assignee field for a task
    pub async fn update_assignee(
        pool: &SqlitePool,
        task_id: Uuid,
        assignee: Option<String>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"UPDATE tasks SET assignee = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, agent_metadata, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            task_id,
            assignee
        )
        .fetch_one(pool)
        .await
    }

    /// Nullify parent_workspace_id for all tasks that reference the given workspace ID
    /// This breaks parent-child relationships before deleting a parent task
    pub async fn nullify_children_by_workspace_id<'e, E>(
        executor: E,
        workspace_id: Uuid,
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!(
            "UPDATE tasks SET parent_workspace_id = NULL WHERE parent_workspace_id = $1",
            workspace_id
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected())
    }

    /// Clear shared_task_id for all tasks that reference shared tasks belonging to a remote project
    /// This breaks the link between local tasks and shared tasks when a project is unlinked
    pub async fn clear_shared_task_ids_for_remote_project<'e, E>(
        executor: E,
        remote_project_id: Uuid,
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!(
            r#"UPDATE tasks
               SET shared_task_id = NULL
               WHERE project_id IN (
                   SELECT id FROM projects WHERE remote_project_id = $1
               )"#,
            remote_project_id
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn set_shared_task_id<'e, E>(
        executor: E,
        id: Uuid,
        shared_task_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query!(
            "UPDATE tasks SET shared_task_id = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
            id,
            shared_task_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn batch_unlink_shared_tasks<'e, E>(
        executor: E,
        shared_task_ids: &[Uuid],
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        if shared_task_ids.is_empty() {
            return Ok(0);
        }

        let mut query_builder = sqlx::QueryBuilder::new(
            "UPDATE tasks SET shared_task_id = NULL, updated_at = CURRENT_TIMESTAMP WHERE shared_task_id IN (",
        );

        let mut separated = query_builder.separated(", ");
        for id in shared_task_ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        let result = query_builder.build().execute(executor).await?;
        Ok(result.rows_affected())
    }

    pub async fn find_children_by_workspace_id(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // Find only child tasks that have this workspace as their parent
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE parent_workspace_id = $1
               ORDER BY created_at DESC"#,
            workspace_id,
        )
        .fetch_all(pool)
        .await
    }

    /// Search tasks by text in title and description using LIKE with wildcards.
    /// Returns tasks matching the query, ordered by relevance (title matches first, then by updated_at).
    pub async fn search_by_query(
        pool: &SqlitePool,
        project_id: Uuid,
        query: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Task>, sqlx::Error> {
        let search_pattern = format!("%{}%", query);
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", shared_task_id as "shared_task_id: Uuid", assignee, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE project_id = $1
                 AND (title LIKE $2 OR description LIKE $2)
               ORDER BY
                 CASE WHEN title LIKE $2 THEN 0 ELSE 1 END,
                 updated_at DESC
               LIMIT $3 OFFSET $4"#,
            project_id,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_relationships_for_workspace(
        pool: &SqlitePool,
        workspace: &Workspace,
    ) -> Result<TaskRelationships, sqlx::Error> {
        // 1. Get the current task (task that owns this workspace)
        let current_task = Self::find_by_id(pool, workspace.task_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // 2. Get parent task (if current task was created by another workspace)
        let parent_task = if let Some(parent_workspace_id) = current_task.parent_workspace_id {
            // Find the workspace that created the current task
            if let Ok(Some(parent_workspace)) =
                Workspace::find_by_id(pool, parent_workspace_id).await
            {
                // Find the task that owns that parent workspace - THAT's the real parent
                Self::find_by_id(pool, parent_workspace.task_id).await?
            } else {
                None
            }
        } else {
            None
        };

        // 3. Get children tasks (created from this workspace)
        let children = Self::find_children_by_workspace_id(pool, workspace.id).await?;

        Ok(TaskRelationships {
            parent_task,
            current_workspace: workspace.clone(),
            children,
        })
    }

    /// Find task relationships given a task_id.
    /// Returns parent task (if any) and child tasks created from this task's workspaces.
    pub async fn find_relationships_for_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<TaskRelationshipsSimple, sqlx::Error> {
        // 1. Get the current task
        let current_task = Self::find_by_id(pool, task_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // 2. Get parent task (if current task was created by another workspace)
        let parent_task = if let Some(parent_workspace_id) = current_task.parent_workspace_id {
            // Find the workspace that created the current task
            if let Ok(Some(parent_workspace)) =
                Workspace::find_by_id(pool, parent_workspace_id).await
            {
                // Find the task that owns that parent workspace - THAT's the real parent
                Self::find_by_id(pool, parent_workspace.task_id).await?
            } else {
                None
            }
        } else {
            None
        };

        // 3. Get all workspaces for this task
        let workspaces = Workspace::fetch_all(pool, Some(task_id))
            .await
            .map_err(|e| match e {
                super::workspace::WorkspaceError::Database(db_err) => db_err,
                _ => sqlx::Error::RowNotFound,
            })?;

        // 4. Collect all children from all workspaces
        let mut children = Vec::new();
        for workspace in &workspaces {
            let workspace_children = Self::find_children_by_workspace_id(pool, workspace.id).await?;
            children.extend(workspace_children);
        }

        // Remove duplicates (in case a child somehow has multiple parent workspace refs)
        children.sort_by(|a, b| a.id.cmp(&b.id));
        children.dedup_by(|a, b| a.id == b.id);

        Ok(TaskRelationshipsSimple {
            current_task,
            parent_task,
            children,
        })
    }
}

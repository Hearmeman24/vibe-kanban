use std::{future::Future, str::FromStr};

use db::models::{
    project::Project,
    repo::Repo,
    tag::Tag,
    task::{CreateTask, Task, TaskStatus, TaskWithAttemptStatus, UpdateTask},
    workspace::{Workspace, WorkspaceContext},
};
use executors::{executors::BaseCodingAgent, profile::ExecutorProfileId};
use regex::Regex;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json;
use uuid::Uuid;

use crate::routes::{
    containers::ContainerQuery,
    task_attempts::WorkspaceRepoInput,
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTaskRequest {
    #[schemars(description = "The ID of the project to create the task in. This is required!")]
    pub project_id: Uuid,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProjectSummary {
    #[schemars(description = "The unique identifier of the project")]
    pub id: String,
    #[schemars(description = "The name of the project")]
    pub name: String,
    #[schemars(description = "When the project was created")]
    pub created_at: String,
    #[schemars(description = "When the project was last updated")]
    pub updated_at: String,
}

impl ProjectSummary {
    fn from_project(project: Project) -> Self {
        Self {
            id: project.id.to_string(),
            name: project.name,
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct McpRepoSummary {
    #[schemars(description = "The unique identifier of the repository")]
    pub id: String,
    #[schemars(description = "The name of the repository")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListReposRequest {
    #[schemars(description = "The ID of the project to list repositories from")]
    pub project_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListReposResponse {
    pub repos: Vec<McpRepoSummary>,
    pub count: usize,
    pub project_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListProjectsResponse {
    pub projects: Vec<ProjectSummary>,
    pub count: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTasksRequest {
    #[schemars(description = "The ID of the project to list tasks from")]
    pub project_id: Uuid,
    #[schemars(
        description = "Optional status filter: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'"
    )]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of tasks to return (default: 50)")]
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTasksAdvancedRequest {
    #[schemars(description = "The ID of the project to list tasks from. This is required!")]
    pub project_id: Uuid,
    #[schemars(
        description = "Filter by multiple statuses: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'"
    )]
    pub statuses: Option<Vec<String>>,
    #[schemars(description = "Filter by assignee name (exact match)")]
    pub assignee: Option<String>,
    #[schemars(description = "Filter tasks created after this timestamp (RFC3339 format)")]
    pub created_after: Option<String>,
    #[schemars(description = "Filter tasks created before this timestamp (RFC3339 format)")]
    pub created_before: Option<String>,
    #[schemars(description = "Filter tasks updated after this timestamp (RFC3339 format)")]
    pub updated_after: Option<String>,
    #[schemars(description = "Filter tasks updated before this timestamp (RFC3339 format)")]
    pub updated_before: Option<String>,
    #[schemars(description = "Maximum number of tasks to return (default: 50)")]
    pub limit: Option<u32>,
    #[schemars(description = "Number of tasks to skip for pagination (default: 0)")]
    pub offset: Option<u32>,
    #[schemars(description = "Field to sort by: 'created_at', 'updated_at', 'title' (default: 'created_at')")]
    pub sort_by: Option<String>,
    #[schemars(description = "Sort order: 'asc' or 'desc' (default: 'desc')")]
    pub sort_order: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskSummary {
    #[schemars(description = "The unique identifier of the task")]
    pub id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Current status of the task")]
    pub status: String,
    #[schemars(description = "When the task was created")]
    pub created_at: String,
    #[schemars(description = "When the task was last updated")]
    pub updated_at: String,
    #[schemars(description = "Whether the task has an in-progress execution attempt")]
    pub has_in_progress_attempt: Option<bool>,
    #[schemars(description = "Whether the last execution attempt failed")]
    pub last_attempt_failed: Option<bool>,
}

impl TaskSummary {
    fn from_task_with_status(task: TaskWithAttemptStatus) -> Self {
        Self {
            id: task.id.to_string(),
            title: task.title.to_string(),
            status: task.status.to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            has_in_progress_attempt: Some(task.has_in_progress_attempt),
            last_attempt_failed: Some(task.last_attempt_failed),
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskDetails {
    #[schemars(description = "The unique identifier of the task")]
    pub id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
    #[schemars(description = "Current status of the task")]
    pub status: String,
    #[schemars(description = "The assignee of the task (agent or user name)")]
    pub assignee: Option<String>,
    #[schemars(description = "When the task was created")]
    pub created_at: String,
    #[schemars(description = "When the task was last updated")]
    pub updated_at: String,
    #[schemars(description = "Whether the task has an in-progress execution attempt")]
    pub has_in_progress_attempt: Option<bool>,
    #[schemars(description = "Whether the last execution attempt failed")]
    pub last_attempt_failed: Option<bool>,
}

impl TaskDetails {
    fn from_task(task: Task) -> Self {
        Self {
            id: task.id.to_string(),
            title: task.title,
            description: task.description,
            status: task.status.to_string(),
            assignee: task.assignee,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            has_in_progress_attempt: None,
            last_attempt_failed: None,
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskSummary>,
    pub count: usize,
    pub project_id: String,
    pub applied_filters: ListTasksFilters,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksFilters {
    pub status: Option<String>,
    pub limit: i32,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksAdvancedResponse {
    pub tasks: Vec<TaskSummary>,
    pub count: usize,
    pub project_id: String,
    pub applied_filters: ListTasksAdvancedFilters,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksAdvancedFilters {
    pub statuses: Option<Vec<String>>,
    pub assignee: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub updated_after: Option<String>,
    pub updated_before: Option<String>,
    pub limit: u32,
    pub offset: u32,
    pub sort_by: String,
    pub sort_order: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateTaskRequest {
    #[schemars(description = "The ID of the task to update")]
    pub task_id: Uuid,
    #[schemars(description = "New title for the task")]
    pub title: Option<String>,
    #[schemars(description = "New description for the task")]
    pub description: Option<String>,
    #[schemars(description = "New status: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct UpdateTaskResponse {
    pub task: TaskDetails,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteTaskRequest {
    #[schemars(description = "The ID of the task to delete")]
    pub task_id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct McpWorkspaceRepoInput {
    #[schemars(description = "The repository ID")]
    pub repo_id: Uuid,
    #[schemars(description = "The base branch for this repository")]
    pub base_branch: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartWorkspaceSessionRequest {
    #[schemars(description = "The ID of the task to start")]
    pub task_id: Uuid,
    #[schemars(
        description = "The executor type. Only 'ORCHESTRATOR_MANAGED' is supported - orchestrator dispatches sub-agents that manage their own processes."
    )]
    pub executor: String,
    #[schemars(description = "Optional executor variant, if needed")]
    pub variant: Option<String>,
    #[schemars(description = "Base branch for each repository in the project")]
    pub repos: Vec<McpWorkspaceRepoInput>,
    #[schemars(description = "Optional name of the agent starting the session (e.g., 'Ferris', 'Miley'). When provided, metadata is logged to track agent activity.")]
    pub agent_name: Option<String>,
    #[schemars(description = "Workspace mode: only 'branch' mode is supported (creates git branch and database records without worktree/container)")]
    pub mode: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StartWorkspaceSessionResponse {
    pub task_id: String,
    pub workspace_id: String,
    #[schemars(description = "The workspace mode used: 'worktree' or 'branch'")]
    pub mode: String,
    #[schemars(description = "The executor type used")]
    pub executor: String,
    #[schemars(description = "Information about each repository in the workspace")]
    pub repos: Vec<WorkspaceRepoInfo>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WorkspaceRepoInfo {
    #[schemars(description = "The repository ID")]
    pub repo_id: String,
    #[schemars(description = "The branch name created for this workspace")]
    pub branch_name: String,
    #[schemars(description = "The base branch this workspace branch is based on")]
    pub base_branch: String,
    #[schemars(description = "The working directory path for this repository")]
    pub working_directory: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DeleteTaskResponse {
    pub deleted_task_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskRequest {
    #[schemars(description = "The ID of the task to retrieve")]
    pub task_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTaskResponse {
    pub task: TaskDetails,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddTaskCommentRequest {
    #[schemars(description = "The ID of the task to add a comment to")]
    pub task_id: Uuid,
    #[schemars(description = "The content of the comment")]
    pub content: String,
    #[schemars(description = "The author of the comment (e.g., agent name like 'Ferris', 'Bree', etc.)")]
    pub author: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CommentSummary {
    #[schemars(description = "The unique identifier of the comment")]
    pub id: String,
    #[schemars(description = "The ID of the task this comment belongs to")]
    pub task_id: String,
    #[schemars(description = "The content of the comment")]
    pub content: String,
    #[schemars(description = "The author of the comment")]
    pub author: String,
    #[schemars(description = "When the comment was created")]
    pub created_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AddTaskCommentResponse {
    pub comment: CommentSummary,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskCommentsRequest {
    #[schemars(description = "The ID of the task to get comments for")]
    pub task_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTaskCommentsResponse {
    pub comments: Vec<CommentSummary>,
    pub count: usize,
    pub task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskHistoryRequest {
    #[schemars(description = "The ID of the task to get change history for")]
    pub task_id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssignTaskRequest {
    #[schemars(description = "The ID of the task to assign. This is required!")]
    pub task_id: Uuid,
    #[schemars(description = "The name/identifier of the assignee. Pass null/None to unassign the task.")]
    pub assignee: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchTasksRequest {
    #[schemars(description = "The ID of the project to search tasks in. This is required!")]
    pub project_id: Uuid,
    #[schemars(description = "The search query string to match against task titles and descriptions. This is required!")]
    pub query: String,
    #[schemars(description = "Maximum number of tasks to return (default: 50, max: 500)")]
    pub limit: Option<u32>,
    #[schemars(description = "Number of tasks to skip for pagination (default: 0)")]
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SearchTasksResponse {
    pub tasks: Vec<TaskDetails>,
    pub count: usize,
    pub project_id: String,
    pub query: String,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AssignTaskResponse {
    pub task: TaskDetails,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskRelationshipsRequest {
    #[schemars(description = "The ID of the task to get relationships for. This is required!")]
    pub task_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskRelationshipsSummary {
    #[schemars(description = "The task we're querying relationships for")]
    pub current_task: TaskDetails,
    #[schemars(description = "The parent task that spawned this task (if any)")]
    pub parent_task: Option<TaskDetails>,
    #[schemars(description = "Child tasks spawned by this task's workspaces")]
    pub children: Vec<TaskDetails>,
    #[schemars(description = "Number of child tasks")]
    pub children_count: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTaskRelationshipsResponse {
    pub relationships: TaskRelationshipsSummary,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulkUpdateTasksRequest {
    #[schemars(description = "Array of task IDs to update. This is required!")]
    pub task_ids: Vec<Uuid>,
    #[schemars(
        description = "New status for all tasks: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'. This is required!"
    )]
    pub status: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BulkUpdateTasksResponse {
    pub updated_tasks: Vec<TaskDetails>,
    pub count: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskHistorySummary {
    #[schemars(description = "The unique identifier of the history entry")]
    pub id: String,
    #[schemars(description = "The ID of the task this history entry belongs to")]
    pub task_id: String,
    #[schemars(description = "The field that was changed")]
    pub field_changed: String,
    #[schemars(description = "The previous value of the field (null for creates)")]
    pub old_value: Option<String>,
    #[schemars(description = "The new value of the field (null for deletes)")]
    pub new_value: Option<String>,
    #[schemars(description = "Who or what made the change")]
    pub changed_by: String,
    #[schemars(description = "When the change was made")]
    pub changed_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTaskHistoryResponse {
    pub history: Vec<TaskHistorySummary>,
    pub count: usize,
    pub task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddAgentMetadataRequest {
    #[schemars(description = "The ID of the task to add agent metadata to. This is required!")]
    pub task_id: Uuid,
    #[schemars(description = "The name of the agent (e.g., 'Ferris', 'Miley', 'Bree'). This is required!")]
    pub agent_name: String,
    #[schemars(description = "The action performed: 'started', 'completed', 'updated', 'commented', etc. This is required!")]
    pub action: String,
    #[schemars(description = "Optional summary of what the agent did")]
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AgentMetadataSummary {
    #[schemars(description = "The name of the agent")]
    pub agent_name: String,
    #[schemars(description = "The action performed")]
    pub action: String,
    #[schemars(description = "ISO 8601 timestamp when the action occurred")]
    pub timestamp: String,
    #[schemars(description = "Optional summary of what the agent did")]
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AddAgentMetadataResponse {
    pub task_id: String,
    pub entry: AgentMetadataSummary,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetAgentMetadataRequest {
    #[schemars(description = "The ID of the task to get agent metadata for. This is required!")]
    pub task_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetAgentMetadataResponse {
    pub task_id: String,
    pub metadata: Vec<AgentMetadataSummary>,
    pub count: usize,
}

// ============================================================================
// Push/PR MCP Tool Request/Response Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PushWorkspaceBranchRequest {
    #[schemars(description = "The workspace ID to push. This is required!")]
    pub workspace_id: Uuid,
    #[schemars(description = "The repository ID to push. This is required!")]
    pub repo_id: Uuid,
    #[schemars(description = "Whether to force push (overwrite remote). Default: false")]
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PushWorkspaceBranchResponse {
    #[schemars(description = "Whether the push was successful")]
    pub success: bool,
    #[schemars(description = "The branch name that was pushed")]
    pub branch_name: String,
    #[schemars(description = "The remote URL the branch was pushed to")]
    pub remote_url: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateWorkspacePrRequest {
    #[schemars(description = "The workspace ID to create a PR for. This is required!")]
    pub workspace_id: Uuid,
    #[schemars(description = "The repository ID to create a PR for. This is required!")]
    pub repo_id: Uuid,
    #[schemars(description = "The title of the pull request. This is required!")]
    pub title: String,
    #[schemars(description = "Optional body/description for the pull request")]
    pub body: Option<String>,
    #[schemars(description = "Target branch for the PR. Defaults to the workspace's target branch")]
    pub target_branch: Option<String>,
    #[schemars(description = "Whether to create the PR as a draft. Default: false")]
    pub draft: Option<bool>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateWorkspacePrResponse {
    #[schemars(description = "The PR number")]
    pub pr_number: i64,
    #[schemars(description = "The URL of the created PR")]
    pub pr_url: String,
    #[schemars(description = "The status of the PR (e.g., 'open')")]
    pub status: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetWorkspacePrStatusRequest {
    #[schemars(description = "The workspace ID to check PR status for. This is required!")]
    pub workspace_id: Uuid,
    #[schemars(description = "The repository ID to check PR status for. This is required!")]
    pub repo_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetWorkspacePrStatusResponse {
    #[schemars(description = "Whether a PR exists for this workspace/repo")]
    pub has_pr: bool,
    #[schemars(description = "The PR number (if exists)")]
    pub pr_number: Option<i64>,
    #[schemars(description = "The PR URL (if exists)")]
    pub pr_url: Option<String>,
    #[schemars(description = "The PR status: 'open', 'merged', 'closed', or 'unknown'")]
    pub status: Option<String>,
    #[schemars(description = "When the PR was merged (if merged)")]
    pub merged_at: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RefreshWorkspacePrStatusRequest {
    #[schemars(description = "The workspace ID to refresh PR status for. This is required!")]
    pub workspace_id: Uuid,
    #[schemars(description = "The repository ID to refresh PR status for. This is required!")]
    pub repo_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct RefreshWorkspacePrStatusResponse {
    #[schemars(description = "The PR number")]
    pub pr_number: i64,
    #[schemars(description = "The PR status before refresh")]
    pub previous_status: String,
    #[schemars(description = "The PR status after refresh")]
    pub current_status: String,
    #[schemars(description = "Whether the status changed")]
    pub status_changed: bool,
    #[schemars(description = "Whether the task was updated (moved to 'done' if PR merged and task was 'inreview')")]
    pub task_updated: bool,
}

#[derive(Debug, Clone)]
pub struct TaskServer {
    client: reqwest::Client,
    base_url: String,
    tool_router: ToolRouter<TaskServer>,
    context: Option<McpContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct McpRepoContext {
    #[schemars(description = "The unique identifier of the repository")]
    pub repo_id: Uuid,
    #[schemars(description = "The name of the repository")]
    pub repo_name: String,
    #[schemars(description = "The target branch for this repository in this workspace")]
    pub target_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct McpContext {
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub task_title: String,
    pub workspace_id: Uuid,
    pub workspace_branch: String,
    #[schemars(
        description = "Repository info and target branches for each repo in this workspace"
    )]
    pub workspace_repos: Vec<McpRepoContext>,
}

impl TaskServer {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            tool_router: Self::tool_router(),
            context: None,
        }
    }

    pub async fn init(mut self) -> Self {
        let context = self.fetch_context_at_startup().await;

        if context.is_none() {
            self.tool_router.map.remove("get_context");
            tracing::debug!("VK context not available, get_context tool will not be registered");
        } else {
            tracing::info!("VK context loaded, get_context tool available");
        }

        self.context = context;
        self
    }

    async fn fetch_context_at_startup(&self) -> Option<McpContext> {
        let current_dir = std::env::current_dir().ok()?;
        let canonical_path = current_dir.canonicalize().unwrap_or(current_dir);
        let normalized_path = utils::path::normalize_macos_private_alias(&canonical_path);

        let url = self.url("/api/containers/attempt-context");
        let query = ContainerQuery {
            container_ref: normalized_path.to_string_lossy().to_string(),
        };

        let response = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            self.client.get(&url).query(&query).send(),
        )
        .await
        .ok()?
        .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let api_response: ApiResponseEnvelope<WorkspaceContext> = response.json().await.ok()?;

        if !api_response.success {
            return None;
        }

        let ctx = api_response.data?;

        // Map RepoWithTargetBranch to McpRepoContext
        let workspace_repos: Vec<McpRepoContext> = ctx
            .workspace_repos
            .into_iter()
            .map(|rwb| McpRepoContext {
                repo_id: rwb.repo.id,
                repo_name: rwb.repo.name,
                target_branch: rwb.target_branch,
            })
            .collect();

        Some(McpContext {
            project_id: ctx.project.id,
            task_id: ctx.task.id,
            task_title: ctx.task.title,
            workspace_id: ctx.workspace.id,
            workspace_branch: ctx.workspace.branch,
            workspace_repos,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponseEnvelope<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

impl TaskServer {
    fn success<T: Serialize>(data: &T) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(data)
                .unwrap_or_else(|_| "Failed to serialize response".to_string()),
        )]))
    }

    fn err_value(v: serde_json::Value) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::error(vec![Content::text(
            serde_json::to_string_pretty(&v)
                .unwrap_or_else(|_| "Failed to serialize error".to_string()),
        )]))
    }

    fn err<S: Into<String>>(msg: S, details: Option<S>) -> Result<CallToolResult, ErrorData> {
        let mut v = serde_json::json!({"success": false, "error": msg.into()});
        if let Some(d) = details {
            v["details"] = serde_json::json!(d.into());
        };
        Self::err_value(v)
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        rb: reqwest::RequestBuilder,
    ) -> Result<T, CallToolResult> {
        let resp = rb
            .send()
            .await
            .map_err(|e| Self::err("Failed to connect to VK API", Some(&e.to_string())).unwrap())?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(
                Self::err(format!("VK API returned error status: {}", status), None).unwrap(),
            );
        }

        let api_response = resp.json::<ApiResponseEnvelope<T>>().await.map_err(|e| {
            Self::err("Failed to parse VK API response", Some(&e.to_string())).unwrap()
        })?;

        if !api_response.success {
            let msg = api_response.message.as_deref().unwrap_or("Unknown error");
            return Err(Self::err("VK API returned error", Some(msg)).unwrap());
        }

        api_response
            .data
            .ok_or_else(|| Self::err("VK API response missing data field", None).unwrap())
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// Expands @tagname references in text by replacing them with tag content.
    /// Returns the original text if expansion fails (e.g., network error).
    /// Unknown tags are left as-is (not expanded, not an error).
    async fn expand_tags(&self, text: &str) -> String {
        // Pattern matches @tagname where tagname is non-whitespace, non-@ characters
        let tag_pattern = match Regex::new(r"@([^\s@]+)") {
            Ok(re) => re,
            Err(_) => return text.to_string(),
        };

        // Find all unique tag names referenced in the text
        let tag_names: Vec<String> = tag_pattern
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if tag_names.is_empty() {
            return text.to_string();
        }

        // Fetch all tags from the API
        let url = self.url("/api/tags");
        let tags: Vec<Tag> = match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<ApiResponseEnvelope<Vec<Tag>>>().await {
                    Ok(envelope) if envelope.success => envelope.data.unwrap_or_default(),
                    _ => return text.to_string(),
                }
            }
            _ => return text.to_string(),
        };

        // Build a map of tag_name -> content for quick lookup
        let tag_map: std::collections::HashMap<&str, &str> = tags
            .iter()
            .map(|t| (t.tag_name.as_str(), t.content.as_str()))
            .collect();

        // Replace each @tagname with its content (if found)
        let result = tag_pattern.replace_all(text, |caps: &regex::Captures| {
            let tag_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            match tag_map.get(tag_name) {
                Some(content) => (*content).to_string(),
                None => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
            }
        });

        result.into_owned()
    }
}

#[tool_router]
impl TaskServer {
    #[tool(
        description = "Return project, task, and workspace metadata for the current workspace session context."
    )]
    async fn get_context(&self) -> Result<CallToolResult, ErrorData> {
        // Context was fetched at startup and cached
        // This tool is only registered if context exists, so unwrap is safe
        let context = self.context.as_ref().expect("VK context should exist");
        TaskServer::success(context)
    }

    #[tool(
        description = "Create a new task/ticket in a project. Always pass the `project_id` of the project you want to create the task in - it is required!"
    )]
    async fn create_task(
        &self,
        Parameters(CreateTaskRequest {
            project_id,
            title,
            description,
        }): Parameters<CreateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Expand @tagname references in description
        let expanded_description = match description {
            Some(desc) => Some(self.expand_tags(&desc).await),
            None => None,
        };

        let url = self.url("/api/tasks");

        let task: Task = match self
            .send_json(
                self.client
                    .post(&url)
                    .json(&CreateTask::from_title_description(
                        project_id,
                        title,
                        expanded_description,
                    )),
            )
            .await
        {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&CreateTaskResponse {
            task_id: task.id.to_string(),
        })
    }

    #[tool(description = "List all the available projects")]
    async fn list_projects(&self) -> Result<CallToolResult, ErrorData> {
        let url = self.url("/api/projects");
        let projects: Vec<Project> = match self.send_json(self.client.get(&url)).await {
            Ok(ps) => ps,
            Err(e) => return Ok(e),
        };

        let project_summaries: Vec<ProjectSummary> = projects
            .into_iter()
            .map(ProjectSummary::from_project)
            .collect();

        let response = ListProjectsResponse {
            count: project_summaries.len(),
            projects: project_summaries,
        };

        TaskServer::success(&response)
    }

    #[tool(description = "List all repositories for a project. `project_id` is required!")]
    async fn list_repos(
        &self,
        Parameters(ListReposRequest { project_id }): Parameters<ListReposRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/projects/{}/repositories", project_id));
        let repos: Vec<Repo> = match self.send_json(self.client.get(&url)).await {
            Ok(rs) => rs,
            Err(e) => return Ok(e),
        };

        let repo_summaries: Vec<McpRepoSummary> = repos
            .into_iter()
            .map(|r| McpRepoSummary {
                id: r.id.to_string(),
                name: r.name,
            })
            .collect();

        let response = ListReposResponse {
            count: repo_summaries.len(),
            repos: repo_summaries,
            project_id: project_id.to_string(),
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "List all the task/tickets in a project with optional filtering and execution status. `project_id` is required!"
    )]
    async fn list_tasks(
        &self,
        Parameters(ListTasksRequest {
            project_id,
            status,
            limit,
        }): Parameters<ListTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let status_filter = if let Some(ref status_str) = status {
            match TaskStatus::from_str(status_str) {
                Ok(s) => Some(s),
                Err(_) => {
                    return Self::err(
                        "Invalid status filter. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'".to_string(),
                        Some(status_str.to_string()),
                    );
                }
            }
        } else {
            None
        };

        let url = self.url(&format!("/api/tasks?project_id={}", project_id));
        let all_tasks: Vec<TaskWithAttemptStatus> =
            match self.send_json(self.client.get(&url)).await {
                Ok(t) => t,
                Err(e) => return Ok(e),
            };

        let task_limit = limit.unwrap_or(50).max(0) as usize;
        let filtered = all_tasks.into_iter().filter(|t| {
            if let Some(ref want) = status_filter {
                &t.status == want
            } else {
                true
            }
        });
        let limited: Vec<TaskWithAttemptStatus> = filtered.take(task_limit).collect();

        let task_summaries: Vec<TaskSummary> = limited
            .into_iter()
            .map(TaskSummary::from_task_with_status)
            .collect();

        let response = ListTasksResponse {
            count: task_summaries.len(),
            tasks: task_summaries,
            project_id: project_id.to_string(),
            applied_filters: ListTasksFilters {
                status: status.clone(),
                limit: task_limit as i32,
            },
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Advanced task listing with multiple filters, date ranges, sorting, and pagination. Use this for complex queries. `project_id` is required!"
    )]
    async fn list_tasks_advanced(
        &self,
        Parameters(ListTasksAdvancedRequest {
            project_id,
            statuses,
            assignee,
            created_after,
            created_before,
            updated_after,
            updated_before,
            limit,
            offset,
            sort_by,
            sort_order,
        }): Parameters<ListTasksAdvancedRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        use chrono::DateTime;

        // Validate statuses
        if let Some(ref status_strs) = statuses {
            for status_str in status_strs {
                if TaskStatus::from_str(status_str).is_err() {
                    return Self::err(
                        "Invalid status value. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'".to_string(),
                        Some(status_str.to_string()),
                    );
                }
            }
        }

        // Validate date filters
        if let Some(ref ts) = created_after {
            if DateTime::parse_from_rfc3339(ts).is_err() {
                return Self::err(
                    "Invalid created_after timestamp. Use RFC3339 format".to_string(),
                    Some(ts.to_string()),
                );
            }
        }

        if let Some(ref ts) = created_before {
            if DateTime::parse_from_rfc3339(ts).is_err() {
                return Self::err(
                    "Invalid created_before timestamp. Use RFC3339 format".to_string(),
                    Some(ts.to_string()),
                );
            }
        }

        if let Some(ref ts) = updated_after {
            if DateTime::parse_from_rfc3339(ts).is_err() {
                return Self::err(
                    "Invalid updated_after timestamp. Use RFC3339 format".to_string(),
                    Some(ts.to_string()),
                );
            }
        }

        if let Some(ref ts) = updated_before {
            if DateTime::parse_from_rfc3339(ts).is_err() {
                return Self::err(
                    "Invalid updated_before timestamp. Use RFC3339 format".to_string(),
                    Some(ts.to_string()),
                );
            }
        }

        // Validate and set defaults for pagination and sorting
        let task_limit = limit.unwrap_or(50).max(1).min(500);
        let task_offset = offset.unwrap_or(0);
        let task_sort_by = sort_by.as_deref().unwrap_or("created_at");
        let task_sort_order = sort_order.as_deref().unwrap_or("desc");

        // Validate sort_by
        if !matches!(task_sort_by, "created_at" | "updated_at" | "title") {
            return Self::err(
                "Invalid sort_by value. Valid values: 'created_at', 'updated_at', 'title'".to_string(),
                Some(task_sort_by.to_string()),
            );
        }

        // Validate sort_order
        if !matches!(task_sort_order, "asc" | "desc") {
            return Self::err(
                "Invalid sort_order value. Valid values: 'asc', 'desc'".to_string(),
                Some(task_sort_order.to_string()),
            );
        }

        // Build query parameters
        let mut query_params = vec![("project_id", project_id.to_string())];

        if let Some(ref status_list) = statuses {
            for status in status_list {
                query_params.push(("statuses", status.clone()));
            }
        }

        if let Some(ref assignee_name) = assignee {
            query_params.push(("assignee", assignee_name.clone()));
        }

        if let Some(ref ts) = created_after {
            query_params.push(("created_after", ts.clone()));
        }
        if let Some(ref ts) = created_before {
            query_params.push(("created_before", ts.clone()));
        }
        if let Some(ref ts) = updated_after {
            query_params.push(("updated_after", ts.clone()));
        }
        if let Some(ref ts) = updated_before {
            query_params.push(("updated_before", ts.clone()));
        }

        query_params.push(("limit", task_limit.to_string()));
        query_params.push(("offset", task_offset.to_string()));
        query_params.push(("sort_by", task_sort_by.to_string()));
        query_params.push(("sort_order", task_sort_order.to_string()));

        let url = self.url("/api/tasks/advanced");
        let filtered_tasks: Vec<TaskWithAttemptStatus> =
            match self.send_json(self.client.get(&url).query(&query_params)).await {
                Ok(t) => t,
                Err(e) => return Ok(e),
            };

        let task_summaries: Vec<TaskSummary> = filtered_tasks
            .into_iter()
            .map(TaskSummary::from_task_with_status)
            .collect();

        let response = ListTasksAdvancedResponse {
            count: task_summaries.len(),
            tasks: task_summaries,
            project_id: project_id.to_string(),
            applied_filters: ListTasksAdvancedFilters {
                statuses: statuses.clone(),
                assignee: assignee.clone(),
                created_after: created_after.clone(),
                created_before: created_before.clone(),
                updated_after: updated_after.clone(),
                updated_before: updated_before.clone(),
                limit: task_limit,
                offset: task_offset,
                sort_by: task_sort_by.to_string(),
                sort_order: task_sort_order.to_string(),
            },
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Start working on a task by creating a workspace session with branch-only mode. Only ORCHESTRATOR_MANAGED executor is supported - the orchestrator dispatches sub-agents that manage their own processes."
    )]
    async fn start_workspace_session(
        &self,
        Parameters(StartWorkspaceSessionRequest {
            task_id,
            executor,
            variant,
            repos,
            agent_name,
            mode,
        }): Parameters<StartWorkspaceSessionRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        if repos.is_empty() {
            return Self::err(
                "At least one repository must be specified.".to_string(),
                None::<String>,
            );
        }

        let executor_trimmed = executor.trim();
        if executor_trimmed.is_empty() {
            return Self::err("Executor must not be empty.".to_string(), None::<String>);
        }

        let normalized_executor = executor_trimmed.replace('-', "_").to_ascii_uppercase();

        // ONLY ORCHESTRATOR_MANAGED is supported - reject all other executor types
        if normalized_executor != "ORCHESTRATOR_MANAGED" {
            return Self::err(
                format!(
                    "Invalid executor '{}'. Only 'ORCHESTRATOR_MANAGED' is supported. \
                    ORCHESTRATOR_MANAGED is used when the orchestrator dispatches sub-agents \
                    that manage their own processes.",
                    executor_trimmed
                ),
                None::<String>,
            );
        }

        // ORCHESTRATOR_MANAGED always uses branch mode (no worktree/container)
        // Validate that mode is either not specified or explicitly "branch"
        if let Some(ref m) = mode {
            let m_lower = m.trim().to_lowercase();
            if m_lower != "branch" {
                return Self::err(
                    format!(
                        "Invalid mode '{}'. ORCHESTRATOR_MANAGED only supports mode='branch'. \
                        Worktree mode is not available.",
                        m
                    ),
                    None::<String>,
                );
            }
        }
        let mode_str = "branch".to_string();

        // For ORCHESTRATOR_MANAGED, we use CLAUDE_CODE as placeholder for DB records
        // (no process is spawned due to branch mode)
        let placeholder_executor = BaseCodingAgent::ClaudeCode;
        let executor_profile_id = ExecutorProfileId {
            executor: placeholder_executor,
            variant: variant.and_then(|v| {
                let trimmed = v.trim();
                if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
            }),
        };
        let executor_for_response = "ORCHESTRATOR_MANAGED".to_string();

        // NOTE: The following code for other executor types is commented out but preserved
        // in case we need to re-enable support for other executors in the future.
        //
        // ```rust
        // let is_orchestrator_managed = normalized_executor == "ORCHESTRATOR_MANAGED";
        // let mode_str = if is_orchestrator_managed {
        //     if let Some(ref m) = mode {
        //         let m_lower = m.trim().to_lowercase();
        //         if m_lower != "branch" {
        //             return Self::err(
        //                 "ORCHESTRATOR_MANAGED executor requires mode='branch'.".to_string(),
        //                 None::<String>,
        //             );
        //         }
        //     }
        //     "branch".to_string()
        // } else {
        //     mode.as_deref().unwrap_or("worktree").trim().to_lowercase()
        // };
        //
        // let (executor_profile_id, executor_for_response) = if is_orchestrator_managed {
        //     let placeholder_executor = BaseCodingAgent::ClaudeCode;
        //     (
        //         ExecutorProfileId {
        //             executor: placeholder_executor,
        //             variant: variant.and_then(|v| {
        //                 let trimmed = v.trim();
        //                 if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        //             }),
        //         },
        //         "ORCHESTRATOR_MANAGED".to_string(),
        //     )
        // } else {
        //     let base_executor = match BaseCodingAgent::from_str(&normalized_executor) {
        //         Ok(exec) => exec,
        //         Err(_) => {
        //             return Self::err(
        //                 format!("Unknown executor '{executor_trimmed}'."),
        //                 None::<String>,
        //             );
        //         }
        //     };
        //     let variant = variant.and_then(|v| {
        //         let trimmed = v.trim();
        //         if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        //     });
        //     (
        //         ExecutorProfileId {
        //             executor: base_executor,
        //             variant,
        //         },
        //         normalized_executor,
        //     )
        // };
        // ```

        // Clone repos for response building later
        let repos_input: Vec<_> = repos.iter().map(|r| (r.repo_id, r.base_branch.clone())).collect();

        let workspace_repos: Vec<WorkspaceRepoInput> = repos
            .into_iter()
            .map(|r| WorkspaceRepoInput {
                repo_id: r.repo_id,
                target_branch: r.base_branch,
            })
            .collect();

        // If agent_name is provided, log agent metadata for the task
        if let Some(ref name) = agent_name {
            let trimmed_name = name.trim();
            if !trimmed_name.is_empty() {
                let metadata_url = self.url(&format!("/api/tasks/{}/agent-metadata", task_id));
                let metadata_payload = serde_json::json!({
                    "agent_name": trimmed_name,
                    "action": "started",
                    "summary": format!("Started workspace session with executor {} (mode: {})", executor_for_response, mode_str)
                });
                // Fire and forget - don't block on metadata logging
                let _ = self.client.post(&metadata_url).json(&metadata_payload).send().await;
            }
        }

        // ORCHESTRATOR_MANAGED always passes the executor name override
        let executor_name_override = Some("ORCHESTRATOR_MANAGED".to_string());

        let payload = serde_json::json!({
            "task_id": task_id,
            "executor_profile_id": executor_profile_id,
            "repos": workspace_repos,
            "mode": mode_str,
            "executor_name": executor_name_override,
        });

        let url = self.url("/api/task-attempts");
        let workspace: Workspace = match self.send_json(self.client.post(&url).json(&payload)).await
        {
            Ok(workspace) => workspace,
            Err(e) => return Ok(e),
        };

        // Auto-assign task to agent if agent_name is provided
        if let Some(ref name) = agent_name {
            let trimmed_name = name.trim();
            if !trimmed_name.is_empty() {
                let assign_payload = UpdateTask {
                    title: None,
                    description: None,
                    status: None,
                    parent_workspace_id: None,
                    image_ids: None,
                    assignee: Some(trimmed_name.to_string()),
                };
                let assign_url = self.url(&format!("/api/tasks/{}", task_id));
                // Fire and forget - don't block on assignment (best effort)
                let _ = self.send_json::<Task>(self.client.put(&assign_url).json(&assign_payload)).await;
            }
        }

        // Build repo info for response
        // For branch mode (only mode supported), working_directory is the project root (repo path)
        let mut repo_infos = Vec::new();
        for (repo_id, base_branch) in repos_input {
            // Get repo path from the repos API
            let repo_url = self.url(&format!("/api/repos/{}", repo_id));
            let working_directory = match self.send_json::<serde_json::Value>(self.client.get(&repo_url)).await {
                Ok(repo_data) => repo_data.get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string(),
                Err(_) => String::new(),
            };

            repo_infos.push(WorkspaceRepoInfo {
                repo_id: repo_id.to_string(),
                branch_name: workspace.branch.clone(),
                base_branch,
                working_directory,
            });
        }

        let response = StartWorkspaceSessionResponse {
            task_id: workspace.task_id.to_string(),
            workspace_id: workspace.id.to_string(),
            mode: mode_str,
            executor: executor_for_response,
            repos: repo_infos,
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Update an existing task/ticket's title, description, or status. `project_id` and `task_id` are required! `title`, `description`, and `status` are optional."
    )]
    async fn update_task(
        &self,
        Parameters(UpdateTaskRequest {
            task_id,
            title,
            description,
            status,
        }): Parameters<UpdateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let status = if let Some(ref status_str) = status {
            match TaskStatus::from_str(status_str) {
                Ok(s) => Some(s),
                Err(_) => {
                    return Self::err(
                        "Invalid status filter. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'".to_string(),
                        Some(status_str.to_string()),
                    );
                }
            }
        } else {
            None
        };

        // Expand @tagname references in description
        let expanded_description = match description {
            Some(desc) => Some(self.expand_tags(&desc).await),
            None => None,
        };

        let payload = UpdateTask {
            title,
            description: expanded_description,
            status,
            parent_workspace_id: None,
            image_ids: None,
            assignee: None,
        };
        let url = self.url(&format!("/api/tasks/{}", task_id));
        let updated_task: Task = match self.send_json(self.client.put(&url).json(&payload)).await {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let details = TaskDetails::from_task(updated_task);
        let response = UpdateTaskResponse { task: details };
        TaskServer::success(&response)
    }

    #[tool(
        description = "Delete a task/ticket from a project. `project_id` and `task_id` are required!"
    )]
    async fn delete_task(
        &self,
        Parameters(DeleteTaskRequest { task_id }): Parameters<DeleteTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}", task_id));
        if let Err(e) = self
            .send_json::<serde_json::Value>(self.client.delete(&url))
            .await
        {
            return Ok(e);
        }

        let repsonse = DeleteTaskResponse {
            deleted_task_id: Some(task_id.to_string()),
        };

        TaskServer::success(&repsonse)
    }

    #[tool(
        description = "Get detailed information (like task description) about a specific task/ticket. You can use `list_tasks` to find the `task_ids` of all tasks in a project. `project_id` and `task_id` are required!"
    )]
    async fn get_task(
        &self,
        Parameters(GetTaskRequest { task_id }): Parameters<GetTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}", task_id));
        let task: Task = match self.send_json(self.client.get(&url)).await {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let details = TaskDetails::from_task(task);
        let response = GetTaskResponse { task: details };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Add a comment to a task. Use this to leave notes, progress updates, or other information on a task. `task_id`, `content`, and `author` are required!"
    )]
    async fn add_task_comment(
        &self,
        Parameters(AddTaskCommentRequest {
            task_id,
            content,
            author,
        }): Parameters<AddTaskCommentRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Validate inputs
        if content.trim().is_empty() {
            return Self::err("Comment content cannot be empty".to_string(), None::<String>);
        }
        if author.trim().is_empty() {
            return Self::err("Author cannot be empty".to_string(), None::<String>);
        }

        let url = self.url(&format!("/api/tasks/{}/comments", task_id));
        let payload = serde_json::json!({
            "task_id": task_id,
            "content": content,
            "author": author
        });

        #[derive(Debug, Deserialize)]
        struct ApiComment {
            id: Uuid,
            task_id: Uuid,
            content: String,
            author: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let comment: ApiComment = match self.send_json(self.client.post(&url).json(&payload)).await
        {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };

        let response = AddTaskCommentResponse {
            comment: CommentSummary {
                id: comment.id.to_string(),
                task_id: comment.task_id.to_string(),
                content: comment.content,
                author: comment.author,
                created_at: comment.created_at.to_rfc3339(),
            },
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Get all comments for a task. Returns comments in chronological order (oldest first). `task_id` is required!"
    )]
    async fn get_task_comments(
        &self,
        Parameters(GetTaskCommentsRequest { task_id }): Parameters<GetTaskCommentsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}/comments", task_id));

        #[derive(Debug, Deserialize)]
        struct ApiComment {
            id: Uuid,
            task_id: Uuid,
            content: String,
            author: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let comments: Vec<ApiComment> = match self.send_json(self.client.get(&url)).await {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };

        let comment_summaries: Vec<CommentSummary> = comments
            .into_iter()
            .map(|c| CommentSummary {
                id: c.id.to_string(),
                task_id: c.task_id.to_string(),
                content: c.content,
                author: c.author,
                created_at: c.created_at.to_rfc3339(),
            })
            .collect();

        let response = GetTaskCommentsResponse {
            count: comment_summaries.len(),
            comments: comment_summaries,
            task_id: task_id.to_string(),
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Get the change history for a task. Returns all modifications made to the task including field changes, who made them, and when. `task_id` is required!"
    )]
    async fn get_task_history(
        &self,
        Parameters(GetTaskHistoryRequest { task_id }): Parameters<GetTaskHistoryRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}/history", task_id));

        #[derive(Debug, Deserialize)]
        struct ApiHistory {
            id: Uuid,
            task_id: Uuid,
            field_changed: String,
            old_value: Option<String>,
            new_value: Option<String>,
            changed_by: String,
            changed_at: chrono::DateTime<chrono::Utc>,
        }

        let history: Vec<ApiHistory> = match self.send_json(self.client.get(&url)).await {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };

        let history_summaries: Vec<TaskHistorySummary> = history
            .into_iter()
            .map(|h| TaskHistorySummary {
                id: h.id.to_string(),
                task_id: h.task_id.to_string(),
                field_changed: h.field_changed,
                old_value: h.old_value,
                new_value: h.new_value,
                changed_by: h.changed_by,
                changed_at: h.changed_at.to_rfc3339(),
            })
            .collect();

        let response = GetTaskHistoryResponse {
            count: history_summaries.len(),
            history: history_summaries,
            task_id: task_id.to_string(),
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Assign a task to an agent or user. Pass assignee as the name/identifier. Pass null/None to unassign. `task_id` is required!"
    )]
    async fn assign_task(
        &self,
        Parameters(AssignTaskRequest { task_id, assignee }): Parameters<AssignTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Validate assignee: if provided, must not be empty/whitespace-only
        let assignee = match assignee {
            Some(s) if s.trim().is_empty() => None, // Empty string = unassign
            Some(s) => Some(s),                     // Non-empty string = assign
            None => None,                           // Null = unassign
        };

        let payload = UpdateTask {
            title: None,
            description: None,
            status: None,
            parent_workspace_id: None,
            image_ids: None,
            assignee: assignee.clone(),
        };

        let url = self.url(&format!("/api/tasks/{}", task_id));
        let updated_task: Task = match self.send_json(self.client.put(&url).json(&payload)).await {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let details = TaskDetails::from_task(updated_task);
        let response = AssignTaskResponse { task: details };
        TaskServer::success(&response)
    }

    #[tool(
        description = "Search tasks by text in title and description. Returns matching tasks with details. `project_id` and `query` are required!"
    )]
    async fn search_tasks(
        &self,
        Parameters(SearchTasksRequest {
            project_id,
            query,
            limit,
            offset,
        }): Parameters<SearchTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let search_query = query.trim();
        if search_query.is_empty() {
            return Self::err(
                "Search query cannot be empty".to_string(),
                None::<String>,
            );
        }

        let task_limit = limit.unwrap_or(50).max(1).min(500);
        let task_offset = offset.unwrap_or(0);

        let url = self.url("/api/tasks/search");
        let query_params = vec![
            ("project_id", project_id.to_string()),
            ("q", search_query.to_string()),
            ("limit", task_limit.to_string()),
            ("offset", task_offset.to_string()),
        ];

        let tasks: Vec<Task> = match self
            .send_json(self.client.get(&url).query(&query_params))
            .await
        {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let task_details: Vec<TaskDetails> = tasks
            .into_iter()
            .map(TaskDetails::from_task)
            .collect();

        let response = SearchTasksResponse {
            count: task_details.len(),
            tasks: task_details,
            project_id: project_id.to_string(),
            query: search_query.to_string(),
            limit: task_limit,
            offset: task_offset,
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Get parent and child tasks for a given task. Returns the task's relationships in the hierarchy - useful for understanding task dependencies and subtasks. `task_id` is required!"
    )]
    async fn get_task_relationships(
        &self,
        Parameters(GetTaskRelationshipsRequest { task_id }): Parameters<GetTaskRelationshipsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}/relationships", task_id));

        #[derive(Debug, Deserialize)]
        struct ApiTaskRelationships {
            current_task: Task,
            parent_task: Option<Task>,
            children: Vec<Task>,
        }

        let relationships: ApiTaskRelationships =
            match self.send_json(self.client.get(&url)).await {
                Ok(r) => r,
                Err(e) => return Ok(e),
            };

        let children_details: Vec<TaskDetails> = relationships
            .children
            .into_iter()
            .map(TaskDetails::from_task)
            .collect();

        let response = GetTaskRelationshipsResponse {
            relationships: TaskRelationshipsSummary {
                current_task: TaskDetails::from_task(relationships.current_task),
                parent_task: relationships.parent_task.map(TaskDetails::from_task),
                children_count: children_details.len(),
                children: children_details,
            },
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Update the status of multiple tasks at once. `task_ids` (array) and `status` are required!"
    )]
    async fn bulk_update_tasks(
        &self,
        Parameters(BulkUpdateTasksRequest { task_ids, status }): Parameters<BulkUpdateTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        if task_ids.is_empty() {
            return Self::err(
                "task_ids array cannot be empty".to_string(),
                None::<String>,
            );
        }

        // Validate status
        let status_trimmed = status.trim();
        if TaskStatus::from_str(status_trimmed).is_err() {
            return Self::err(
                "Invalid status. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'"
                    .to_string(),
                Some(status.clone()),
            );
        }

        let url = self.url("/api/tasks/bulk-update");
        let payload = serde_json::json!({
            "task_ids": task_ids,
            "status": status_trimmed
        });

        #[derive(Debug, Deserialize)]
        struct ApiBulkUpdateResponse {
            updated_tasks: Vec<Task>,
            #[allow(dead_code)]
            count: usize,
        }

        let api_response: ApiBulkUpdateResponse =
            match self.send_json(self.client.post(&url).json(&payload)).await {
                Ok(r) => r,
                Err(e) => return Ok(e),
            };

        let task_details: Vec<TaskDetails> = api_response
            .updated_tasks
            .into_iter()
            .map(TaskDetails::from_task)
            .collect();

        let response = BulkUpdateTasksResponse {
            count: task_details.len(),
            updated_tasks: task_details,
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Add agent metadata to a task. Use this to track which agents have worked on a task and what actions they performed. `task_id`, `agent_name`, and `action` are required!"
    )]
    async fn add_agent_metadata(
        &self,
        Parameters(AddAgentMetadataRequest {
            task_id,
            agent_name,
            action,
            summary,
        }): Parameters<AddAgentMetadataRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Validate inputs
        let agent_name_trimmed = agent_name.trim();
        if agent_name_trimmed.is_empty() {
            return Self::err("agent_name cannot be empty".to_string(), None::<String>);
        }

        let action_trimmed = action.trim();
        if action_trimmed.is_empty() {
            return Self::err("action cannot be empty".to_string(), None::<String>);
        }

        let url = self.url(&format!("/api/tasks/{}/agent-metadata", task_id));
        let payload = serde_json::json!({
            "agent_name": agent_name_trimmed,
            "action": action_trimmed,
            "summary": summary
        });

        let _updated_task: Task = match self.send_json(self.client.post(&url).json(&payload)).await {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        // Return the entry that was added
        let response = AddAgentMetadataResponse {
            task_id: task_id.to_string(),
            entry: AgentMetadataSummary {
                agent_name: agent_name_trimmed.to_string(),
                action: action_trimmed.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                summary,
            },
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Get all agent metadata entries for a task. Returns the history of which agents worked on the task and what actions they performed. `task_id` is required!"
    )]
    async fn get_agent_metadata(
        &self,
        Parameters(GetAgentMetadataRequest { task_id }): Parameters<GetAgentMetadataRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}/agent-metadata", task_id));

        #[derive(Debug, Deserialize)]
        struct ApiAgentMetadataEntry {
            agent_name: String,
            action: String,
            timestamp: String,
            summary: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct ApiGetAgentMetadataResponse {
            #[allow(dead_code)]
            task_id: Uuid,
            metadata: Vec<ApiAgentMetadataEntry>,
            #[allow(dead_code)]
            count: usize,
        }

        let api_response: ApiGetAgentMetadataResponse =
            match self.send_json(self.client.get(&url)).await {
                Ok(r) => r,
                Err(e) => return Ok(e),
            };

        let metadata_summaries: Vec<AgentMetadataSummary> = api_response
            .metadata
            .into_iter()
            .map(|entry| AgentMetadataSummary {
                agent_name: entry.agent_name,
                action: entry.action,
                timestamp: entry.timestamp,
                summary: entry.summary,
            })
            .collect();

        let response = GetAgentMetadataResponse {
            task_id: task_id.to_string(),
            count: metadata_summaries.len(),
            metadata: metadata_summaries,
        };

        TaskServer::success(&response)
    }

    // ========================================================================
    // Git/PR MCP Tools
    // ========================================================================

    #[tool(
        description = "Push a workspace branch to GitHub. This delegates to the existing push functionality. `workspace_id` and `repo_id` are required!"
    )]
    async fn push_workspace_branch(
        &self,
        Parameters(PushWorkspaceBranchRequest {
            workspace_id,
            repo_id,
            force,
        }): Parameters<PushWorkspaceBranchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let force = force.unwrap_or(false);

        // Build the push URL - using force endpoint if force=true
        let url = if force {
            self.url(&format!("/api/task-attempts/{}/push/force", workspace_id))
        } else {
            self.url(&format!("/api/task-attempts/{}/push", workspace_id))
        };

        let payload = serde_json::json!({
            "repo_id": repo_id
        });

        // First get workspace info to include in response
        let workspace_url = self.url(&format!("/api/task-attempts/{}", workspace_id));
        let workspace: Workspace = match self.send_json(self.client.get(&workspace_url)).await {
            Ok(w) => w,
            Err(e) => return Ok(e),
        };

        // Get repo info for remote URL
        #[derive(Debug, Deserialize)]
        struct RepoWithTargetBranch {
            id: Uuid,
            #[allow(dead_code)]
            name: String,
            path: String,
            #[allow(dead_code)]
            target_branch: String,
        }

        let repos_url = self.url(&format!("/api/task-attempts/{}/repos", workspace_id));
        let repos: Vec<RepoWithTargetBranch> =
            match self.send_json(self.client.get(&repos_url)).await {
                Ok(r) => r,
                Err(e) => return Ok(e),
            };

        let repo_info = repos.iter().find(|r| r.id == repo_id);
        let remote_url = repo_info.map(|r| r.path.clone());

        // Execute the push
        let _: serde_json::Value = match self.send_json(self.client.post(&url).json(&payload)).await
        {
            Ok(v) => v,
            Err(e) => return Ok(e),
        };

        let response = PushWorkspaceBranchResponse {
            success: true,
            branch_name: workspace.branch,
            remote_url,
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Create a GitHub Pull Request for a workspace. This pushes the branch and creates the PR. `workspace_id`, `repo_id`, and `title` are required!"
    )]
    async fn create_workspace_pr(
        &self,
        Parameters(CreateWorkspacePrRequest {
            workspace_id,
            repo_id,
            title,
            body,
            target_branch,
            draft,
        }): Parameters<CreateWorkspacePrRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Validate title
        if title.trim().is_empty() {
            return Self::err("PR title cannot be empty".to_string(), None::<String>);
        }

        let url = self.url(&format!("/api/task-attempts/{}/pr", workspace_id));
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "target_branch": target_branch,
            "draft": draft.unwrap_or(false),
            "repo_id": repo_id,
            "auto_generate_description": false
        });

        // The PR endpoint returns the PR URL as a string on success
        let pr_url: String = match self.send_json(self.client.post(&url).json(&payload)).await {
            Ok(u) => u,
            Err(e) => return Ok(e),
        };

        // Extract PR number from URL (format: https://github.com/owner/repo/pull/123)
        let pr_number = pr_url
            .rsplit('/')
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let response = CreateWorkspacePrResponse {
            pr_number,
            pr_url,
            status: "open".to_string(),
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Get PR status for a workspace from the database (not live from GitHub). `workspace_id` and `repo_id` are required!"
    )]
    async fn get_workspace_pr_status(
        &self,
        Parameters(GetWorkspacePrStatusRequest {
            workspace_id,
            repo_id,
        }): Parameters<GetWorkspacePrStatusRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Get branch status which includes merge info
        let url = self.url(&format!("/api/task-attempts/{}/branch-status", workspace_id));

        #[derive(Debug, Deserialize)]
        struct ApiMerge {
            #[serde(rename = "type")]
            merge_type: String,
            #[serde(default)]
            pr_info: Option<ApiPrInfo>,
        }

        #[derive(Debug, Deserialize)]
        struct ApiPrInfo {
            number: i64,
            url: String,
            status: String,
            merged_at: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct ApiBranchStatus {
            repo_id: Uuid,
            #[allow(dead_code)]
            repo_name: String,
            merges: Vec<ApiMerge>,
        }

        let statuses: Vec<ApiBranchStatus> = match self.send_json(self.client.get(&url)).await {
            Ok(s) => s,
            Err(e) => return Ok(e),
        };

        // Find the status for the requested repo
        let repo_status = statuses.iter().find(|s| s.repo_id == repo_id);

        let response = if let Some(status) = repo_status {
            // Find a PR merge in the merges list
            let pr_merge = status
                .merges
                .iter()
                .find(|m| m.merge_type == "pr" && m.pr_info.is_some());

            if let Some(merge) = pr_merge {
                let pr_info = merge.pr_info.as_ref().unwrap();
                GetWorkspacePrStatusResponse {
                    has_pr: true,
                    pr_number: Some(pr_info.number),
                    pr_url: Some(pr_info.url.clone()),
                    status: Some(pr_info.status.clone()),
                    merged_at: pr_info.merged_at.clone(),
                }
            } else {
                GetWorkspacePrStatusResponse {
                    has_pr: false,
                    pr_number: None,
                    pr_url: None,
                    status: None,
                    merged_at: None,
                }
            }
        } else {
            GetWorkspacePrStatusResponse {
                has_pr: false,
                pr_number: None,
                pr_url: None,
                status: None,
                merged_at: None,
            }
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Refresh PR status from GitHub API and update the database. If PR is merged and task is 'inreview', moves task to 'done'. `workspace_id` and `repo_id` are required!"
    )]
    async fn refresh_workspace_pr_status(
        &self,
        Parameters(RefreshWorkspacePrStatusRequest {
            workspace_id,
            repo_id,
        }): Parameters<RefreshWorkspacePrStatusRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // First, get current PR status from database
        let status_url = self.url(&format!("/api/task-attempts/{}/branch-status", workspace_id));

        #[derive(Debug, Deserialize)]
        struct ApiMerge {
            #[serde(rename = "type")]
            merge_type: String,
            #[serde(default)]
            pr_info: Option<ApiPrInfo>,
        }

        #[derive(Debug, Deserialize)]
        struct ApiPrInfo {
            number: i64,
            #[allow(dead_code)]
            url: String,
            status: String,
            #[allow(dead_code)]
            merged_at: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct ApiBranchStatus {
            repo_id: Uuid,
            #[allow(dead_code)]
            repo_name: String,
            merges: Vec<ApiMerge>,
        }

        let statuses: Vec<ApiBranchStatus> =
            match self.send_json(self.client.get(&status_url)).await {
                Ok(s) => s,
                Err(e) => return Ok(e),
            };

        // Find the status for the requested repo
        let repo_status = statuses.iter().find(|s| s.repo_id == repo_id);

        let (pr_number, previous_status) = if let Some(status) = repo_status {
            let pr_merge = status
                .merges
                .iter()
                .find(|m| m.merge_type == "pr" && m.pr_info.is_some());

            if let Some(merge) = pr_merge {
                let pr_info = merge.pr_info.as_ref().unwrap();
                (pr_info.number, pr_info.status.clone())
            } else {
                return Self::err(
                    "No PR found for this workspace/repo combination".to_string(),
                    None::<String>,
                );
            }
        } else {
            return Self::err(
                "Repo not found in workspace".to_string(),
                None::<String>,
            );
        };

        // Now attach/refresh the PR using the attach endpoint
        // This will fetch fresh status from GitHub
        let attach_url = self.url(&format!("/api/task-attempts/{}/pr/attach", workspace_id));
        let attach_payload = serde_json::json!({
            "repo_id": repo_id
        });

        #[derive(Debug, Deserialize)]
        struct AttachPrResponse {
            #[allow(dead_code)]
            pr_attached: bool,
            #[allow(dead_code)]
            pr_url: Option<String>,
            #[allow(dead_code)]
            pr_number: Option<i64>,
            pr_status: Option<String>,
        }

        let attach_response: AttachPrResponse =
            match self.send_json(self.client.post(&attach_url).json(&attach_payload)).await {
                Ok(r) => r,
                Err(e) => return Ok(e),
            };

        let current_status = attach_response
            .pr_status
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let status_changed = previous_status != current_status;

        // If PR is now merged and status changed, check if we should update the task
        let mut task_updated = false;
        if status_changed && current_status == "merged" {
            // Get workspace to find the task
            let workspace_url = self.url(&format!("/api/task-attempts/{}", workspace_id));

            #[derive(Debug, Deserialize)]
            struct WorkspaceInfo {
                task_id: Uuid,
            }

            if let Ok(workspace_info) =
                self.send_json::<WorkspaceInfo>(self.client.get(&workspace_url)).await
            {
                // Get task to check its status
                let task_url = self.url(&format!("/api/tasks/{}", workspace_info.task_id));
                if let Ok(task) = self.send_json::<Task>(self.client.get(&task_url)).await {
                    // If task is "inreview", move it to "done"
                    if task.status == TaskStatus::InReview {
                        let update_payload = serde_json::json!({
                            "status": "done"
                        });
                        if self
                            .send_json::<Task>(
                                self.client.put(&task_url).json(&update_payload),
                            )
                            .await
                            .is_ok()
                        {
                            task_updated = true;
                        }
                    }
                }
            }
        }

        let response = RefreshWorkspacePrStatusResponse {
            pr_number,
            previous_status,
            current_status,
            status_changed,
            task_updated,
        };

        TaskServer::success(&response)
    }
}

#[tool_handler]
impl ServerHandler for TaskServer {
    fn get_info(&self) -> ServerInfo {
        let mut instruction = "A task and project management server. If you need to create or update tickets or tasks then use these tools. Most of them absolutely require that you pass the `project_id` of the project that you are currently working on. You can get project ids by using `list projects`. Call `list_tasks` to fetch the `task_ids` of all the tasks in a project. For advanced filtering, sorting, and pagination, use `list_tasks_advanced`. Use `search_tasks` to find tasks by keyword in title or description. Use `get_task_relationships` to see parent/child task hierarchies. Use `add_agent_metadata` and `get_agent_metadata` to track which agents worked on a task. For Git/PR operations: use `push_workspace_branch` to push a workspace branch to GitHub, `create_workspace_pr` to create a pull request, `get_workspace_pr_status` to check PR status from the database, and `refresh_workspace_pr_status` to refresh PR status from GitHub (auto-updates task to 'done' when PR is merged and task was 'inreview'). TOOLS: 'list_projects', 'list_tasks', 'list_tasks_advanced', 'search_tasks', 'create_task', 'start_workspace_session', 'get_task', 'get_task_relationships', 'update_task', 'bulk_update_tasks', 'delete_task', 'list_repos', 'add_task_comment', 'get_task_comments', 'get_task_history', 'assign_task', 'add_agent_metadata', 'get_agent_metadata', 'push_workspace_branch', 'create_workspace_pr', 'get_workspace_pr_status', 'refresh_workspace_pr_status'. Make sure to pass `project_id` or `task_id` where required. You can use list tools to get the available ids.".to_string();
        if self.context.is_some() {
            let context_instruction = "Use 'get_context' to fetch project/task/workspace metadata for the active Vibe Kanban workspace session when available.";
            instruction = format!("{} {}", context_instruction, instruction);
        }

        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "vibe-kanban".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some(instruction),
        }
    }
}

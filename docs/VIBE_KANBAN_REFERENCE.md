# Vibe Kanban Reference Document

**Version:** 1.0
**Last Updated:** 2026-01-05

## Table of Contents

1. [Overview](#1-overview)
2. [Database Entities](#2-database-entities)
3. [MCP Tools Reference](#3-mcp-tools-reference)
4. [Workflows](#4-workflows)
5. [Integration FAQ](#5-integration-faq)
6. [Recommended Patterns](#6-recommended-patterns)

---

## 1. Overview

### What is Vibe Kanban?

Vibe Kanban is a **multi-agent task management system** designed to orchestrate AI-powered development workflows. It provides:

- **Task Management**: Create, track, and manage development tasks with status tracking
- **Workspace Isolation**: Each task execution runs in an isolated workspace with its own git branch
- **MCP Integration**: Model Context Protocol (MCP) tools for AI agent orchestration
- **Multi-Repository Support**: Work across multiple repositories in a single workspace
- **PR Workflow**: Automated GitHub Pull Request creation and status tracking
- **Agent Metadata**: Track which AI agents have worked on tasks

### Architecture Summary

```
┌─────────────────────────────────────────────────────────────┐
│  MCP Server (task_server.rs)                                │
│  ├─ Project Management Tools                                │
│  ├─ Task CRUD Tools                                         │
│  ├─ Workspace/Session Management                            │
│  └─ Git/PR Tools                                            │
└─────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  HTTP API (Axum Routes)                                     │
│  ├─ /api/tasks                                              │
│  ├─ /api/task-attempts                                      │
│  ├─ /api/projects                                           │
│  └─ /api/containers                                         │
└─────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  Database Layer (SQLite)                                    │
│  ├─ projects, tasks, workspaces                             │
│  ├─ sessions, execution_processes                           │
│  ├─ merges (PRs), workspace_repos                           │
│  ├─ task_comments, task_history                             │
│  └─ repos, project_repos                                    │
└─────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  Container Service                                          │
│  ├─ Docker container management                             │
│  ├─ Git worktree creation                                   │
│  └─ Agent executor launching                                │
└─────────────────────────────────────────────────────────────┘
```

**Stack:**
- **Backend**: Rust (Axum, SQLx, SQLite)
- **Frontend**: React (TypeScript, TanStack Query, Radix UI, shadcn)
- **Container**: Docker (optional)
- **Git**: Git worktrees for workspace isolation

---

## 2. Database Entities

### Project

A project represents a collection of repositories and tasks.

**Fields:**
- `id` (UUID) - Unique identifier
- `name` (String) - Project name
- `dev_script` (Optional String) - Development server script
- `dev_script_working_dir` (Optional String) - Working directory for dev script
- `default_agent_working_dir` (Optional String) - Default working directory for agents
- `remote_project_id` (Optional UUID) - Link to remote project (for syncing)
- `created_at` (DateTime<Utc>) - Creation timestamp
- `updated_at` (DateTime<Utc>) - Last update timestamp

**Relationships:**
- Has many `Task`
- Has many `Repo` (through `ProjectRepo` join table)

**Lifecycle:**
- Created when setting up a new project
- Deleted manually (cascades to tasks, project_repos)

---

### Task

A task represents a unit of work (ticket/issue) within a project.

**Fields:**
- `id` (UUID) - Unique identifier
- `project_id` (UUID) - Foreign key to Project
- `title` (String) - Task title
- `description` (Optional String) - Task description (supports @tagname expansion)
- `status` (TaskStatus) - Current status: `todo`, `inprogress`, `inreview`, `done`, `cancelled`
- `parent_workspace_id` (Optional UUID) - Foreign key to Workspace (if spawned by a workspace)
- `shared_task_id` (Optional UUID) - Link to shared task (for remote sync)
- `assignee` (Optional String) - Name of assigned agent/user
- `agent_metadata` (Optional String) - JSON array of `AgentMetadataEntry`
- `created_at` (DateTime<Utc>) - Creation timestamp
- `updated_at` (DateTime<Utc>) - Last update timestamp

**Status Flow:**
```
todo → inprogress → inreview → done
   ↘      ↓            ↓         ↓
     cancelled ←──────┴─────────┘
```

**Relationships:**
- Belongs to one `Project`
- Has many `Workspace`
- Has many `TaskComment`
- Has many `TaskHistory` entries
- May have one parent `Workspace` (if spawned from another workspace)

**Agent Metadata Structure:**
```typescript
{
  agent_name: string;      // e.g., "Ferris", "Miley"
  action: string;          // e.g., "started", "completed", "updated"
  timestamp: string;       // ISO 8601 format
  summary?: string;        // Optional description
}
```

---

### Workspace

A workspace represents an execution attempt for a task. Each workspace gets its own git branch and optional Docker container.

**Fields:**
- `id` (UUID) - Unique identifier
- `task_id` (UUID) - Foreign key to Task
- `container_ref` (Optional String) - Reference to Docker container (if containerized)
- `branch` (String) - Git branch name for this workspace
- `agent_working_dir` (Optional String) - Working directory for the agent
- `setup_completed_at` (Optional DateTime<Utc>) - When setup completed
- `created_at` (DateTime<Utc>) - Creation timestamp
- `updated_at` (DateTime<Utc>) - Last update timestamp

**Relationships:**
- Belongs to one `Task`
- Has many `WorkspaceRepo` (repositories used in this workspace)
- Has many `Session` (execution sessions)
- Has many `Merge` (direct merges or PRs)

**Lifecycle:**
1. **Created** - Workspace created with branch name
2. **Container Started** - Optional Docker container launched
3. **Worktree Created** - Git worktree created for the branch
4. **Executor Running** - Coding agent executor starts
5. **Work Complete** - Agent completes work
6. **Pushed** - Branch pushed to remote
7. **PR Created** - Pull request created (optional)
8. **Merged** - Changes merged
9. **Cleanup** - Container stopped, worktree removed (after 72 hours)

**Branch Naming:**
Format: `vk-{workspace_id}-{sanitized_task_title}`

---

### WorkspaceRepo

Join table linking a workspace to repositories, storing the target branch for each repo.

**Fields:**
- `id` (UUID) - Unique identifier
- `workspace_id` (UUID) - Foreign key to Workspace
- `repo_id` (UUID) - Foreign key to Repo
- `target_branch` (String) - Base branch to target (e.g., "main", "develop")
- `created_at` (DateTime<Utc>) - Creation timestamp
- `updated_at` (DateTime<Utc>) - Last update timestamp

**Purpose:**
- Associates repositories with a workspace
- Stores which branch to use as the base/target for PRs
- Allows different workspaces to target different branches

---

### Session

A session represents an execution of an agent within a workspace.

**Fields:**
- `id` (UUID) - Unique identifier
- `workspace_id` (UUID) - Foreign key to Workspace
- `executor` (Optional String) - Executor type (e.g., "CLAUDE_CODE", "CURSOR_AGENT")
- `created_at` (DateTime<Utc>) - Creation timestamp
- `updated_at` (DateTime<Utc>) - Last update timestamp

**Relationships:**
- Belongs to one `Workspace`
- Has many `ExecutionProcess` (individual process runs)

---

### Merge

A merge represents either a direct merge to a target branch or a Pull Request.

**Types:**

#### DirectMerge
- `id` (UUID)
- `workspace_id` (UUID)
- `repo_id` (UUID)
- `merge_commit` (String) - Commit SHA
- `target_branch_name` (String) - Branch merged into
- `created_at` (DateTime<Utc>)

#### PrMerge
- `id` (UUID)
- `workspace_id` (UUID)
- `repo_id` (UUID)
- `target_branch_name` (String)
- `pr_info` (PullRequestInfo):
  - `number` (i64) - PR number
  - `url` (String) - GitHub PR URL
  - `status` (MergeStatus) - `open`, `merged`, `closed`, `unknown`
  - `merged_at` (Optional DateTime<Utc>) - When PR was merged
  - `merge_commit_sha` (Optional String) - Merge commit SHA

**Relationships:**
- Belongs to one `Workspace`
- Belongs to one `Repo`

---

### TaskComment

User/agent comments on tasks.

**Fields:**
- `id` (UUID) - Unique identifier
- `task_id` (UUID) - Foreign key to Task
- `content` (String) - Comment text
- `author` (String) - Author name (agent or user)
- `created_at` (DateTime<Utc>) - Creation timestamp

**Usage:**
- Log progress updates
- Leave notes for other agents
- Document decisions

---

### TaskHistory

Audit trail of task changes.

**Fields:**
- `id` (UUID) - Unique identifier
- `task_id` (UUID) - Foreign key to Task
- `field_changed` (String) - Which field was changed
- `old_value` (Optional String) - Previous value
- `new_value` (Optional String) - New value
- `changed_by` (String) - Who/what made the change
- `changed_at` (DateTime<Utc>) - When the change occurred

**Tracked Fields:**
- `status` - Status changes
- `title` - Title changes
- `description` - Description changes
- `assignee` - Assignment changes

---

## 3. MCP Tools Reference

All MCP tools are implemented in `/crates/server/src/mcp/task_server.rs`.

### Project Tools

#### `list_projects`

List all available projects.

**Parameters:** None

**Returns:**
```typescript
{
  projects: Array<{
    id: string;
    name: string;
    created_at: string;
    updated_at: string;
  }>;
  count: number;
}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "list_projects", {});
// { projects: [...], count: 5 }
```

---

#### `list_repos`

List all repositories for a project.

**Parameters:**
- `project_id` (UUID, **required**) - Project ID

**Returns:**
```typescript
{
  repos: Array<{
    id: string;
    name: string;
  }>;
  count: number;
  project_id: string;
}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "list_repos", {
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f"
});
```

---

### Task Tools

#### `create_task`

Create a new task in a project.

**Parameters:**
- `project_id` (UUID, **required**) - Project ID
- `title` (String, **required**) - Task title
- `description` (Optional String) - Task description (supports @tagname expansion)

**Returns:**
```typescript
{
  task_id: string;
}
```

**What it creates:**
- New `Task` record with status `todo`
- `TaskHistory` entry for creation

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "create_task", {
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  title: "Fix authentication bug",
  description: "Users cannot log in with email addresses containing special characters."
});
// { task_id: "abc123..." }
```

**Tag Expansion:**
If description contains `@tagname`, it's replaced with tag content from the tags database.

---

#### `list_tasks`

List tasks in a project with basic filtering.

**Parameters:**
- `project_id` (UUID, **required**) - Project ID
- `status` (Optional String) - Filter by status: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'
- `limit` (Optional i32) - Max results (default: 50)

**Returns:**
```typescript
{
  tasks: Array<{
    id: string;
    title: string;
    status: string;
    created_at: string;
    updated_at: string;
    has_in_progress_attempt?: boolean;
    last_attempt_failed?: boolean;
  }>;
  count: number;
  project_id: string;
  applied_filters: {
    status?: string;
    limit: number;
  };
}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "list_tasks", {
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  status: "todo",
  limit: 10
});
```

---

#### `list_tasks_advanced`

Advanced task listing with multiple filters, sorting, and pagination.

**Parameters:**
- `project_id` (UUID, **required**)
- `statuses` (Optional Array<String>) - Multiple statuses
- `assignee` (Optional String) - Exact match on assignee name
- `created_after` (Optional String) - RFC3339 timestamp
- `created_before` (Optional String) - RFC3339 timestamp
- `updated_after` (Optional String) - RFC3339 timestamp
- `updated_before` (Optional String) - RFC3339 timestamp
- `limit` (Optional u32) - Default: 50, max: 500
- `offset` (Optional u32) - Default: 0
- `sort_by` (Optional String) - 'created_at', 'updated_at', 'title' (default: 'created_at')
- `sort_order` (Optional String) - 'asc' or 'desc' (default: 'desc')

**Returns:**
```typescript
{
  tasks: Array<TaskSummary>;
  count: number;
  project_id: string;
  applied_filters: {
    statuses?: string[];
    assignee?: string;
    created_after?: string;
    created_before?: string;
    updated_after?: string;
    updated_before?: string;
    limit: number;
    offset: number;
    sort_by: string;
    sort_order: string;
  };
}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "list_tasks_advanced", {
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  statuses: ["inprogress", "inreview"],
  assignee: "Ferris",
  sort_by: "updated_at",
  sort_order: "desc",
  limit: 20
});
```

---

#### `get_task`

Get detailed information about a specific task.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  task: {
    id: string;
    title: string;
    description?: string;
    status: string;
    assignee?: string;
    created_at: string;
    updated_at: string;
    has_in_progress_attempt?: boolean;
    last_attempt_failed?: boolean;
  };
}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "get_task", {
  task_id: "abc123..."
});
```

---

#### `update_task`

Update a task's title, description, or status.

**Parameters:**
- `task_id` (UUID, **required**)
- `title` (Optional String)
- `description` (Optional String) - Supports @tagname expansion
- `status` (Optional String) - 'todo', 'inprogress', 'inreview', 'done', 'cancelled'

**Returns:**
```typescript
{
  task: TaskDetails;
}
```

**What it modifies:**
- Updates specified fields on `Task`
- Creates `TaskHistory` entries for each changed field

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "update_task", {
  task_id: "abc123...",
  status: "done"
});
```

---

#### `delete_task`

Delete a task from a project.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  deleted_task_id?: string;
}
```

**What it deletes:**
- Task record
- Associated task_comments (cascaded)
- Associated task_history (cascaded)

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "delete_task", {
  task_id: "abc123..."
});
```

---

#### `assign_task`

Assign a task to an agent or user, or unassign.

**Parameters:**
- `task_id` (UUID, **required**)
- `assignee` (Optional String) - Name/identifier of assignee. Pass null/empty to unassign.

**Returns:**
```typescript
{
  task: TaskDetails;
}
```

**What it modifies:**
- Updates `assignee` field on `Task`
- Creates `TaskHistory` entry

**Example:**
```typescript
// Assign
const result = await mcp.call("vibe_kanban", "assign_task", {
  task_id: "abc123...",
  assignee: "Ferris"
});

// Unassign
const result = await mcp.call("vibe_kanban", "assign_task", {
  task_id: "abc123...",
  assignee: null
});
```

---

#### `bulk_update_tasks`

Update the status of multiple tasks at once.

**Parameters:**
- `task_ids` (Array<UUID>, **required**) - Array of task IDs
- `status` (String, **required**) - New status for all tasks

**Returns:**
```typescript
{
  updated_tasks: Array<TaskDetails>;
  count: number;
}
```

**What it modifies:**
- Updates `status` on all specified tasks
- Creates `TaskHistory` entries for each task

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "bulk_update_tasks", {
  task_ids: ["abc123...", "def456..."],
  status: "done"
});
```

---

#### `search_tasks`

Search tasks by text in title and description.

**Parameters:**
- `project_id` (UUID, **required**)
- `query` (String, **required**) - Search query
- `limit` (Optional u32) - Default: 50, max: 500
- `offset` (Optional u32) - Default: 0

**Returns:**
```typescript
{
  tasks: Array<TaskDetails>;
  count: number;
  project_id: string;
  query: string;
  limit: number;
  offset: number;
}
```

**Search Logic:**
- Uses SQL LIKE with wildcards: `%query%`
- Searches in both `title` and `description`
- Prioritizes title matches, then sorts by `updated_at DESC`

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "search_tasks", {
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  query: "authentication",
  limit: 10
});
```

---

#### `get_task_relationships`

Get parent and child tasks for a given task.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  relationships: {
    current_task: TaskDetails;
    parent_task?: TaskDetails;     // Task that spawned this task (if any)
    children: Array<TaskDetails>;  // Tasks spawned by this task's workspaces
    children_count: number;
  };
}
```

**Relationship Logic:**
- **Parent Task**: If current task has `parent_workspace_id`, finds the task that owns that workspace
- **Children**: Finds all tasks where `parent_workspace_id` references any workspace belonging to the current task

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "get_task_relationships", {
  task_id: "abc123..."
});
// Shows task hierarchy
```

---

#### `get_task_history`

Get the change history for a task.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  history: Array<{
    id: string;
    task_id: string;
    field_changed: string;
    old_value?: string;
    new_value?: string;
    changed_by: string;
    changed_at: string;
  }>;
  count: number;
  task_id: string;
}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "get_task_history", {
  task_id: "abc123..."
});
// Shows all modifications to the task
```

---

### Comment Tools

#### `add_task_comment`

Add a comment to a task.

**Parameters:**
- `task_id` (UUID, **required**)
- `content` (String, **required**) - Comment text
- `author` (String, **required**) - Author name (e.g., "Ferris", "Bree")

**Returns:**
```typescript
{
  comment: {
    id: string;
    task_id: string;
    content: string;
    author: string;
    created_at: string;
  };
}
```

**What it creates:**
- New `TaskComment` record

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "add_task_comment", {
  task_id: "abc123...",
  content: "Fixed the authentication bug by updating the regex validation.",
  author: "Ferris"
});
```

---

#### `get_task_comments`

Get all comments for a task.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  comments: Array<CommentSummary>;
  count: number;
  task_id: string;
}
```

**Ordering:** Chronological (oldest first)

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "get_task_comments", {
  task_id: "abc123..."
});
```

---

### Agent Metadata Tools

#### `add_agent_metadata`

Add agent metadata to a task to track which agents worked on it.

**Parameters:**
- `task_id` (UUID, **required**)
- `agent_name` (String, **required**) - Agent name (e.g., "Ferris", "Miley")
- `action` (String, **required**) - Action performed (e.g., "started", "completed", "updated")
- `summary` (Optional String) - Description of what was done

**Returns:**
```typescript
{
  task_id: string;
  entry: {
    agent_name: string;
    action: string;
    timestamp: string;
    summary?: string;
  };
}
```

**What it modifies:**
- Appends to `agent_metadata` JSON array on `Task`

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "add_agent_metadata", {
  task_id: "abc123...",
  agent_name: "Ferris",
  action: "completed",
  summary: "Fixed authentication bug and added tests"
});
```

---

#### `get_agent_metadata`

Get all agent metadata entries for a task.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  task_id: string;
  metadata: Array<{
    agent_name: string;
    action: string;
    timestamp: string;
    summary?: string;
  }>;
  count: number;
}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "get_agent_metadata", {
  task_id: "abc123..."
});
// Shows history of agent activity
```

---

### Workspace Tools

#### `start_workspace_session`

Start working on a task by creating a new workspace session.

**Parameters:**
- `task_id` (UUID, **required**)
- `executor` (String, **required**) - Executor type: 'CLAUDE_CODE', 'CODEX', 'GEMINI', 'CURSOR_AGENT', 'OPENCODE'
- `variant` (Optional String) - Executor variant if needed
- `repos` (Array<McpWorkspaceRepoInput>, **required**) - At least one repo required
  - `repo_id` (UUID) - Repository ID
  - `base_branch` (String) - Base/target branch (e.g., "main")
- `agent_name` (Optional String) - Agent name for metadata logging

**Returns:**
```typescript
{
  task_id: string;
  workspace_id: string;
}
```

**What it creates:**
1. New `Workspace` record with generated branch name
2. `WorkspaceRepo` records for each repo
3. `Session` record
4. `AgentMetadataEntry` if `agent_name` provided
5. Triggers container/worktree creation (async)

**Branch Name Format:**
```
vk-{workspace_id}-{sanitized_task_title}
```

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "start_workspace_session", {
  task_id: "abc123...",
  executor: "CLAUDE_CODE",
  repos: [
    {
      repo_id: "repo123...",
      base_branch: "main"
    }
  ],
  agent_name: "Ferris"
});
// { task_id: "abc123...", workspace_id: "workspace456..." }
```

**Post-creation Actions:**
- Git worktree created for the workspace branch
- Optional Docker container started
- Executor environment prepared

---

### Git/PR Tools

#### `push_workspace_branch`

Push a workspace branch to GitHub.

**Parameters:**
- `workspace_id` (UUID, **required**)
- `repo_id` (UUID, **required**)
- `force` (Optional Boolean) - Whether to force push (default: false)

**Returns:**
```typescript
{
  success: boolean;
  branch_name: string;
  remote_url?: string;
}
```

**What it does:**
- Pushes the workspace branch to the remote repository
- Uses force push if `force=true`

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "push_workspace_branch", {
  workspace_id: "workspace456...",
  repo_id: "repo123...",
  force: false
});
// { success: true, branch_name: "vk-workspace456-fix-auth", remote_url: "..." }
```

---

#### `create_workspace_pr`

Create a GitHub Pull Request for a workspace.

**Parameters:**
- `workspace_id` (UUID, **required**)
- `repo_id` (UUID, **required**)
- `title` (String, **required**) - PR title
- `body` (Optional String) - PR description
- `target_branch` (Optional String) - Target branch (defaults to workspace's target branch)
- `draft` (Optional Boolean) - Create as draft PR (default: false)

**Returns:**
```typescript
{
  pr_number: number;
  pr_url: string;
  status: string;  // e.g., "open"
}
```

**What it creates:**
- `Merge` record (type: `Pr`) with PR info
- GitHub Pull Request (via GitHub API)

**Prerequisites:**
- Workspace branch must be pushed to remote

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "create_workspace_pr", {
  workspace_id: "workspace456...",
  repo_id: "repo123...",
  title: "Fix authentication bug",
  body: "This PR fixes the authentication bug by updating the regex validation.",
  draft: false
});
// { pr_number: 42, pr_url: "https://github.com/...", status: "open" }
```

---

#### `get_workspace_pr_status`

Get PR status for a workspace from the database (not live from GitHub).

**Parameters:**
- `workspace_id` (UUID, **required**)
- `repo_id` (UUID, **required**)

**Returns:**
```typescript
{
  has_pr: boolean;
  pr_number?: number;
  pr_url?: string;
  status?: string;       // "open", "merged", "closed", "unknown"
  merged_at?: string;    // RFC3339 timestamp
}
```

**Data Source:** Database (cached PR status)

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "get_workspace_pr_status", {
  workspace_id: "workspace456...",
  repo_id: "repo123..."
});
// { has_pr: true, pr_number: 42, status: "open", ... }
```

---

#### `refresh_workspace_pr_status`

Refresh PR status from GitHub API and update the database. Auto-updates task to 'done' if PR is merged and task was 'inreview'.

**Parameters:**
- `workspace_id` (UUID, **required**)
- `repo_id` (UUID, **required**)

**Returns:**
```typescript
{
  pr_number: number;
  previous_status: string;
  current_status: string;
  status_changed: boolean;
  task_updated: boolean;  // true if task was moved to 'done'
}
```

**What it does:**
1. Fetches latest PR status from GitHub
2. Updates `Merge` record in database
3. If PR status changed from 'inreview' → 'merged':
   - Finds the task for this workspace
   - If task status is 'inreview', updates it to 'done'

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "refresh_workspace_pr_status", {
  workspace_id: "workspace456...",
  repo_id: "repo123..."
});
// { pr_number: 42, previous_status: "open", current_status: "merged", status_changed: true, task_updated: true }
```

**Use Case:** Periodically check if PRs have been merged and auto-complete tasks.

---

### Context Tool

#### `get_context`

Return project, task, and workspace metadata for the current workspace session context.

**Parameters:** None

**Returns:**
```typescript
{
  project_id: string;
  task_id: string;
  task_title: string;
  workspace_id: string;
  workspace_branch: string;
  workspace_repos: Array<{
    repo_id: string;
    repo_name: string;
    target_branch: string;
  }>;
}
```

**Availability:** Only available when running inside a workspace session (container).

**Example:**
```typescript
const result = await mcp.call("vibe_kanban", "get_context", {});
// { project_id: "...", task_id: "...", workspace_branch: "vk-...", ... }
```

**Use Case:** Agents running inside a workspace can query their current context.

---

## 4. Workflows

### Task Lifecycle

Standard flow for completing a task:

```
┌──────────────────────────────────────────────────────────────┐
│ 1. CREATE TASK                                               │
│    create_task(project_id, title, description)              │
│    → Task created with status='todo'                         │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ 2. ASSIGN TASK (optional)                                    │
│    assign_task(task_id, assignee="Ferris")                  │
│    → Task assigned to agent                                  │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ 3. START WORKSPACE SESSION                                   │
│    start_workspace_session(task_id, executor, repos)        │
│    → Workspace created, branch created, container started   │
│    → Task status changed to 'inprogress' (manual/auto)     │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ 4. AGENT WORKS ON TASK                                       │
│    - Agent makes code changes                                │
│    - Agent can add comments: add_task_comment()             │
│    - Agent can log metadata: add_agent_metadata()           │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ 5. PUSH BRANCH                                               │
│    push_workspace_branch(workspace_id, repo_id)             │
│    → Workspace branch pushed to remote                       │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ 6. CREATE PULL REQUEST                                       │
│    create_workspace_pr(workspace_id, repo_id, title, body)  │
│    → PR created on GitHub                                    │
│    → Merge record created                                    │
│    → Task status changed to 'inreview' (manual)            │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ 7. WAIT FOR PR MERGE                                         │
│    refresh_workspace_pr_status(workspace_id, repo_id)       │
│    → Checks GitHub for PR status                            │
│    → If merged AND task is 'inreview' → task becomes 'done'│
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ 8. TASK COMPLETE                                             │
│    Task status = 'done'                                      │
│    Workspace eligible for cleanup after 72 hours            │
└──────────────────────────────────────────────────────────────┘
```

### Workspace Lifecycle

Detailed workspace creation and management flow:

#### 1. Workspace Creation

```rust
start_workspace_session(task_id, executor, repos)
  ↓
Creates Workspace record
  - id: UUID
  - task_id: from parameter
  - branch: "vk-{id}-{task_title}"
  - container_ref: null (initially)
  ↓
Creates WorkspaceRepo records
  - Links repos to workspace
  - Stores target_branch for each repo
  ↓
Creates Session record
  - executor: from parameter
  ↓
Triggers async container creation
  - Creates Docker container (if configured)
  - Creates git worktree for branch
  - Sets up executor environment
```

#### 2. Git Branch/Worktree

**Worktree Creation:**
```bash
# For each repo in workspace:
git worktree add /path/to/worktree/{workspace_id}/{repo_name} {branch_name}
```

**Branch Location:**
- **NOT** a checkout of the main repository
- **IS** a separate worktree with its own working directory
- Isolated from other workspaces

**Worktree Path (configurable):**
Default: `/tmp/vk-worktrees/{workspace_id}/{repo_name}`

**Branch Tracking:**
- Branch tracks remote target branch (e.g., `origin/main`)
- Commits are local until pushed

#### 3. Container (Optional)

If containerization is enabled:
```
Docker container created:
  - Name: "vk-{workspace_id}"
  - Mounts: Worktree paths mounted as volumes
  - Network: Isolated or shared (configurable)
  - Environment: Executor-specific setup
```

Container lifecycle:
- **Started** on workspace creation
- **Running** during agent execution
- **Stopped** manually or after completion
- **Removed** 72 hours after last activity

#### 4. Workspace Cleanup

After 72 hours of inactivity:
```
Cleanup process:
  1. Stop Docker container
  2. Remove git worktree(s)
  3. Clear workspace.container_ref
  4. Container removed

Database records remain for audit trail.
```

### PR Workflow

Complete pull request creation and merge flow:

```
┌─────────────────────────────────────────────────────────────┐
│ PREREQUISITE: Workspace branch exists with commits          │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 1. PUSH BRANCH                                              │
│    push_workspace_branch(workspace_id, repo_id)            │
│    → Git push to remote                                     │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. CREATE PR                                                │
│    create_workspace_pr(workspace_id, repo_id, title, body) │
│    → GitHub API call to create PR                          │
│    → Merge record created (type: Pr, status: open)         │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. UPDATE TASK STATUS (manual)                             │
│    update_task(task_id, status="inreview")                │
│    → Task marked as in review                              │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. CODE REVIEW (external - GitHub)                         │
│    → Reviewers comment on PR                               │
│    → CI/CD runs                                            │
│    → Approvals collected                                   │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. PR MERGED (external - GitHub)                           │
│    → Human or bot merges PR on GitHub                      │
│    → PR status becomes 'merged'                            │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 6. REFRESH PR STATUS                                        │
│    refresh_workspace_pr_status(workspace_id, repo_id)      │
│    → Fetches latest PR status from GitHub                 │
│    → Updates Merge record: status='merged'                 │
│    → IF task.status=='inreview' → task.status='done'      │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 7. TASK COMPLETE                                           │
│    Task status = 'done'                                     │
│    Work is merged into target branch                        │
└─────────────────────────────────────────────────────────────┘
```

**Automatic Task Completion:**
The `refresh_workspace_pr_status` tool implements automatic task completion:
```rust
if pr_status == "merged" && task.status == "inreview" {
    task.status = "done"
}
```

This allows periodic polling to auto-complete tasks when PRs are merged.

---

## 5. Integration FAQ

### Q1: Does `start_workspace_session` REQUIRE creating a worktree?

**Answer:** Yes, if you want to use the workspace for actual development.

**Details:**
- `start_workspace_session` creates the `Workspace` database record immediately
- It triggers **async** container/worktree creation
- The worktree creation happens via the container service
- If containerization is disabled, worktree creation may be skipped
- You can query the workspace immediately after creation, but the worktree may not exist yet

**Best Practice:**
Wait for `workspace.setup_completed_at` to be set before assuming the worktree is ready.

---

### Q2: Can worktree location be configured?

**Answer:** Yes, via environment variables.

**Configuration:**
```bash
# Default location
WORKTREE_BASE_PATH=/tmp/vk-worktrees

# Custom location
WORKTREE_BASE_PATH=/custom/path/worktrees
```

**Worktree Structure:**
```
{WORKTREE_BASE_PATH}/
  {workspace_id}/
    {repo1_name}/    ← worktree for repo1
    {repo2_name}/    ← worktree for repo2
```

**Example:**
```
/tmp/vk-worktrees/
  abc123-def456-789/
    vibe-kanban/     ← main repo worktree
    frontend/        ← frontend repo worktree
```

---

### Q3: Relationship between workspace_id and git branch name?

**Answer:** The branch name **includes** the workspace_id.

**Branch Name Format:**
```
vk-{workspace_id}-{sanitized_task_title}
```

**Sanitization:**
- Lowercase
- Spaces → hyphens
- Special characters removed
- Truncated to reasonable length

**Example:**
```
Workspace ID: 7d8d2452-d215-469f-8bf8-9be9606a107f
Task Title: "Fix Authentication Bug"
Branch Name: vk-7d8d2452-d215-469f-8bf8-9be9606a107f-fix-authentication-bug
```

**Why include workspace_id?**
- Ensures branch uniqueness (multiple workspaces for same task)
- Allows easy lookup from branch name to workspace

---

### Q4: What does `push_workspace_branch` push?

**Answer:** It pushes the workspace branch (all commits) to the remote repository.

**Behavior:**
```bash
# Equivalent to:
cd /path/to/worktree/{workspace_id}/{repo_name}
git push origin {branch_name}

# With force=true:
git push --force origin {branch_name}
```

**What gets pushed:**
- All commits on the workspace branch
- Branch ref is created/updated on remote
- Does NOT push to target branch (e.g., main)
- Does NOT create a PR (separate step)

**Prerequisites:**
- Worktree must exist
- Branch must have commits
- Remote access must be configured

---

### Q5: Can multiple workspaces exist for same task?

**Answer:** Yes, absolutely!

**Use Case:**
- Retry after failure
- Different approaches (experimental branches)
- Multiple agents working in parallel

**Example:**
```
Task: "Fix Authentication Bug"
  ↓
Workspace 1:
  - ID: abc123...
  - Branch: vk-abc123-fix-authentication-bug
  - Status: Failed
  ↓
Workspace 2:
  - ID: def456...
  - Branch: vk-def456-fix-authentication-bug
  - Status: Success
```

**Database:**
All workspaces reference the same `task_id`:
```sql
SELECT * FROM workspaces WHERE task_id = 'abc123...';
-- Returns multiple workspaces
```

**Branches:**
Each workspace gets its own unique branch name (due to workspace_id inclusion).

---

### Q6: What happens to workspace when task is done?

**Answer:** Workspace remains in database but is marked for cleanup.

**Immediate Actions:**
- Workspace record stays in database (for audit)
- Branch stays on remote (unless manually deleted)
- Worktree stays (until cleanup)
- Container may be stopped

**Cleanup (after 72 hours):**
```sql
-- Query finds expired workspaces
SELECT * FROM workspaces
WHERE container_ref IS NOT NULL
  AND last_activity > 72 hours ago
  AND no running processes
```

**Cleanup Actions:**
1. Stop Docker container
2. Remove worktree: `git worktree remove`
3. Clear `workspace.container_ref`
4. Database record remains

**Manual Cleanup:**
You can manually clean up workspaces earlier if needed.

---

## 6. Recommended Patterns

### Orchestrator Dispatch Pattern

**Context:** You are the orchestrator and must delegate all work.

**Pattern:**
```typescript
// 1. Create task
const { task_id } = await mcp.call("vibe_kanban", "create_task", {
  project_id: PROJECT_ID,
  title: "Fix authentication bug",
  description: "Users cannot log in with special characters in email."
});

// 2. Assign to agent
await mcp.call("vibe_kanban", "assign_task", {
  task_id,
  assignee: "Ferris"
});

// 3. Dispatch to supervisor
await Task({
  subagent_type: "rust-supervisor",
  prompt: `Task ID: ${task_id}\n\nPlease work on this task using start_workspace_session.`
});

// 4. Supervisor logs progress
// (Inside supervisor agent)
await mcp.call("vibe_kanban", "add_task_comment", {
  task_id,
  content: "Started working on authentication fix.",
  author: "Ferris"
});

// 5. Check status later
const { task } = await mcp.call("vibe_kanban", "get_task", {
  task_id
});
console.log(`Task status: ${task.status}`);
```

---

### Agent Progress Reporting

**Pattern:** Agents should log their progress using comments and metadata.

**Example:**
```typescript
// When starting work
await mcp.call("vibe_kanban", "add_agent_metadata", {
  task_id,
  agent_name: "Ferris",
  action: "started",
  summary: "Beginning work on authentication bug"
});

await mcp.call("vibe_kanban", "add_task_comment", {
  task_id,
  content: "Analyzing the authentication flow...",
  author: "Ferris"
});

// During work
await mcp.call("vibe_kanban", "add_task_comment", {
  task_id,
  content: "Found the issue: regex validation doesn't handle special chars",
  author: "Ferris"
});

// When updating
await mcp.call("vibe_kanban", "add_agent_metadata", {
  task_id,
  agent_name: "Ferris",
  action: "updated",
  summary: "Fixed regex validation, added tests"
});

// When completing
await mcp.call("vibe_kanban", "add_agent_metadata", {
  task_id,
  agent_name: "Ferris",
  action: "completed",
  summary: "PR created and tests passing"
});
```

---

### PR Creation Workflow

**Pattern:** Complete workflow for creating and tracking PRs.

**Example:**
```typescript
// 1. Start workspace (already done)
const { workspace_id } = await mcp.call("vibe_kanban", "start_workspace_session", {
  task_id,
  executor: "CLAUDE_CODE",
  repos: [{ repo_id: REPO_ID, base_branch: "main" }],
  agent_name: "Ferris"
});

// 2. Do work (code changes happen here)
// ...

// 3. Push branch
await mcp.call("vibe_kanban", "push_workspace_branch", {
  workspace_id,
  repo_id: REPO_ID
});

// 4. Create PR
const { pr_number, pr_url } = await mcp.call("vibe_kanban", "create_workspace_pr", {
  workspace_id,
  repo_id: REPO_ID,
  title: "Fix authentication bug with special characters",
  body: "This PR fixes the authentication bug by updating the regex validation to handle special characters in email addresses.\n\nFixes #123"
});

// 5. Update task status
await mcp.call("vibe_kanban", "update_task", {
  task_id,
  status: "inreview"
});

// 6. Add comment with PR link
await mcp.call("vibe_kanban", "add_task_comment", {
  task_id,
  content: `PR created: ${pr_url}`,
  author: "Ferris"
});

// 7. Periodic status check (run this in a loop or scheduled job)
const statusCheck = async () => {
  const { status_changed, task_updated } = await mcp.call("vibe_kanban", "refresh_workspace_pr_status", {
    workspace_id,
    repo_id: REPO_ID
  });

  if (status_changed) {
    console.log("PR status changed!");
  }

  if (task_updated) {
    console.log("Task automatically marked as done!");
  }
};

// Check every 5 minutes
setInterval(statusCheck, 5 * 60 * 1000);
```

---

### Task Completion

**Pattern:** Ensure tasks are properly closed.

**Example:**
```typescript
// Option 1: Manual completion
await mcp.call("vibe_kanban", "update_task", {
  task_id,
  status: "done"
});

await mcp.call("vibe_kanban", "add_agent_metadata", {
  task_id,
  agent_name: "Ferris",
  action: "completed",
  summary: "All tests passing, PR merged"
});

// Option 2: Automatic completion (via PR merge)
// Just ensure task is in 'inreview' status before PR is merged
// Then run refresh_workspace_pr_status after merge
await mcp.call("vibe_kanban", "refresh_workspace_pr_status", {
  workspace_id,
  repo_id: REPO_ID
});
// Task automatically becomes 'done' if PR is merged
```

---

### Context Preservation on Re-dispatch

**Pattern:** When re-dispatching due to bugs, include history.

**Example:**
```typescript
// Get full context before re-dispatching
const { task } = await mcp.call("vibe_kanban", "get_task", {
  task_id
});

const { history } = await mcp.call("vibe_kanban", "get_task_history", {
  task_id
});

const { comments } = await mcp.call("vibe_kanban", "get_task_comments", {
  task_id
});

const { metadata } = await mcp.call("vibe_kanban", "get_agent_metadata", {
  task_id
});

// Re-dispatch with full context
await Task({
  subagent_type: "rust-supervisor",
  prompt: `
Task ID: ${task_id}
Title: ${task.title}
Status: ${task.status}

Previous Work:
${comments.map(c => `${c.author}: ${c.content}`).join('\n')}

Agent Activity:
${metadata.map(m => `${m.agent_name} ${m.action}: ${m.summary || ''}`).join('\n')}

Please investigate the issue and continue work.
  `
});
```

---

## Appendix: Quick Reference

### Task Statuses
- `todo` - Not started
- `inprogress` - Being worked on
- `inreview` - PR created, awaiting review
- `done` - Completed and merged
- `cancelled` - Cancelled/abandoned

### Executors
- `CLAUDE_CODE` - Claude Code
- `CODEX` - OpenAI Codex
- `GEMINI` - Google Gemini
- `CURSOR_AGENT` - Cursor Agent
- `OPENCODE` - OpenCode

### Common Agent Names
- `Ferris` - Rust supervisor
- `Miley` - Frontend supervisor
- `Bree` - Worker (small fixes)
- `Emilia` - Infrastructure supervisor
- `Ivy` - Scout (codebase exploration)
- `Vera` - Detective (debugging)
- `Ada` - Architect (planning)
- `Penny` - Scribe (documentation)

---

**End of Reference Document**

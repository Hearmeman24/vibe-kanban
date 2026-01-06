# Vibe Kanban MCP Orchestration Guide for AI Agents

**Version:** 1.0
**Last Updated:** 2026-01-06
**Audience:** Claude AI agents orchestrating multi-agent development workflows

---

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture Overview](#architecture-overview)
3. [Complete MCP Tool Reference](#complete-mcp-tool-reference)
4. [Orchestration Workflows](#orchestration-workflows)
5. [Multi-Agent Coordination Patterns](#multi-agent-coordination-patterns)
6. [Branch-Only vs Container-Based Modes](#branch-only-vs-container-based-modes)
7. [Best Practices](#best-practices)
8. [Troubleshooting](#troubleshooting)

---

## Introduction

### What is Vibe Kanban?

Vibe Kanban is a **multi-agent task management system** that enables AI orchestrators to coordinate complex development workflows across multiple specialized agents. Through the Model Context Protocol (MCP), you can:

- **Create and track tasks** with status progression
- **Start isolated workspaces** with dedicated git branches
- **Coordinate multiple agents** through task assignment and metadata tracking
- **Automate PR workflows** with GitHub integration
- **Preserve context** through comments, history, and agent metadata

### Core Concepts

**Task**: A unit of work (feature, bugfix, refactor) tracked through its lifecycle (`todo` → `inprogress` → `inreview` → `done`)

**Workspace**: An execution environment for a task, including:
- Dedicated git branch for changes
- Optional containerized environment
- Multi-repository support
- Agent metadata tracking

**Agent**: A specialized sub-agent (e.g., Ferris for Rust, Miley for Frontend) dispatched to work on tasks

**Orchestrator**: The primary agent (you) that creates tasks, dispatches agents, and manages the overall workflow

---

## Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│  ORCHESTRATOR (Claude)                                      │
│  └─ Dispatches agents via Task() tool                      │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  MCP TOOLS (vibe_kanban)                                    │
│  ├─ Task Management (create, update, list, search)         │
│  ├─ Workspace Management (start_workspace_session)         │
│  ├─ Collaboration (comments, metadata)                     │
│  ├─ Git/PR Operations (push, create PR, check status)      │
│  └─ Audit (history, relationships)                         │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  DATABASE (SQLite)                                          │
│  ├─ projects, tasks, workspaces                            │
│  ├─ task_comments, task_history                            │
│  ├─ agent_metadata (tracks agent activity)                 │
│  └─ merges (PR tracking)                                   │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  GIT + GITHUB                                               │
│  ├─ Branch creation and management                         │
│  ├─ Pull request creation and tracking                     │
│  └─ Merge status monitoring                                │
└─────────────────────────────────────────────────────────────┘
```

### Task Status Flow

```
todo → inprogress → inreview → done
  ↘      ↓            ↓         ↓
    cancelled ←──────┴─────────┘
```

**Status Meanings:**
- `todo`: Not started, awaiting assignment
- `inprogress`: Agent actively working on task
- `inreview`: PR created, awaiting review/merge
- `done`: Completed and merged (orchestrator-controlled)
- `cancelled`: Abandoned or obsolete

---

## Complete MCP Tool Reference

### 1. Project Management

#### `mcp__vibe_kanban__list_projects`

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
const result = await mcp__vibe_kanban__list_projects();
// { projects: [{ id: "7d8d2452-d215-469f-8bf8-9be9606a107f", name: "Vibe Kanban Refactor", ... }], count: 1 }
```

---

#### `mcp__vibe_kanban__list_repos`

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
const result = await mcp__vibe_kanban__list_repos({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f"
});
```

---

### 2. Task CRUD Operations

#### `mcp__vibe_kanban__create_task`

Create a new task in a project.

**Parameters:**
- `project_id` (UUID, **required**) - Project ID
- `title` (string, **required**) - Task title
- `description` (string, optional) - Task description (supports @tagname expansion)

**Returns:**
```typescript
{
  task_id: string;
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__create_task({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  title: "Implement agent metadata tracking",
  description: "Add MCP tools to track which agents work on tasks"
});
// { task_id: "abc123-def456-789..." }
```

**What it creates:**
- New Task record with status `todo`
- TaskHistory entry for creation

---

#### `mcp__vibe_kanban__get_task`

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
const result = await mcp__vibe_kanban__get_task({
  task_id: "abc123-def456-789..."
});
```

---

#### `mcp__vibe_kanban__update_task`

Update a task's title, description, or status.

**Parameters:**
- `task_id` (UUID, **required**)
- `title` (string, optional)
- `description` (string, optional) - Supports @tagname expansion
- `status` (string, optional) - One of: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'

**Returns:**
```typescript
{
  task: TaskDetails;
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__update_task({
  task_id: "abc123-def456-789...",
  status: "inreview"
});
```

**What it modifies:**
- Updates specified fields on Task
- Creates TaskHistory entries for each changed field

---

#### `mcp__vibe_kanban__delete_task`

Delete a task from a project.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  deleted_task_id?: string;
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__delete_task({
  task_id: "abc123-def456-789..."
});
```

**What it deletes:**
- Task record
- Associated task_comments (cascaded)
- Associated task_history (cascaded)

---

### 3. Task Querying

#### `mcp__vibe_kanban__list_tasks`

List tasks in a project with basic filtering.

**Parameters:**
- `project_id` (UUID, **required**)
- `status` (string, optional) - Filter by status: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'
- `limit` (integer, optional) - Max results (default: 50)

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
const result = await mcp__vibe_kanban__list_tasks({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  status: "inprogress",
  limit: 10
});
```

---

#### `mcp__vibe_kanban__list_tasks_advanced`

Advanced task listing with multiple filters, sorting, and pagination.

**Parameters:**
- `project_id` (UUID, **required**)
- `statuses` (Array<string>, optional) - Multiple statuses
- `assignee` (string, optional) - Exact match on assignee name
- `created_after` (string, optional) - RFC3339 timestamp
- `created_before` (string, optional) - RFC3339 timestamp
- `updated_after` (string, optional) - RFC3339 timestamp
- `updated_before` (string, optional) - RFC3339 timestamp
- `limit` (integer, optional) - Default: 50, max: 500
- `offset` (integer, optional) - Default: 0
- `sort_by` (string, optional) - 'created_at', 'updated_at', 'title' (default: 'created_at')
- `sort_order` (string, optional) - 'asc' or 'desc' (default: 'desc')

**Returns:**
```typescript
{
  tasks: Array<TaskSummary>;
  count: number;
  project_id: string;
  applied_filters: {
    statuses?: string[];
    assignee?: string;
    // ... other filters
    limit: number;
    offset: number;
    sort_by: string;
    sort_order: string;
  };
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__list_tasks_advanced({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  statuses: ["inprogress", "inreview"],
  assignee: "Ferris",
  sort_by: "updated_at",
  sort_order: "desc",
  limit: 20
});
```

---

#### `mcp__vibe_kanban__search_tasks`

Search tasks by text in title and description.

**Parameters:**
- `project_id` (UUID, **required**)
- `query` (string, **required**) - Search query
- `limit` (integer, optional) - Default: 50, max: 500
- `offset` (integer, optional) - Default: 0

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

**Example:**
```typescript
const result = await mcp__vibe_kanban__search_tasks({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  query: "authentication",
  limit: 10
});
```

---

#### `mcp__vibe_kanban__get_task_relationships`

Get parent and child tasks for a given task.

**Parameters:**
- `task_id` (UUID, **required**)

**Returns:**
```typescript
{
  relationships: {
    current_task: TaskDetails;
    parent_task?: TaskDetails;
    children: Array<TaskDetails>;
    children_count: number;
  };
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__get_task_relationships({
  task_id: "abc123-def456-789..."
});
```

**Relationship Logic:**
- **Parent Task**: If current task has `parent_workspace_id`, finds the task that owns that workspace
- **Children**: Finds all tasks where `parent_workspace_id` references any workspace belonging to the current task

---

### 4. Task Assignment

#### `mcp__vibe_kanban__assign_task`

Assign a task to an agent or user, or unassign.

**Parameters:**
- `task_id` (UUID, **required**)
- `assignee` (string or null, optional) - Name/identifier of assignee. Pass null to unassign.

**Returns:**
```typescript
{
  task: TaskDetails;
}
```

**Example:**
```typescript
// Assign to agent
const result = await mcp__vibe_kanban__assign_task({
  task_id: "abc123-def456-789...",
  assignee: "Ferris"
});

// Unassign
const result = await mcp__vibe_kanban__assign_task({
  task_id: "abc123-def456-789...",
  assignee: null
});
```

**What it modifies:**
- Updates `assignee` field on Task
- Creates TaskHistory entry

---

#### `mcp__vibe_kanban__bulk_update_tasks`

Update the status of multiple tasks at once.

**Parameters:**
- `task_ids` (Array<UUID>, **required**) - Array of task IDs
- `status` (string, **required**) - New status for all tasks

**Returns:**
```typescript
{
  updated_tasks: Array<TaskDetails>;
  count: number;
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__bulk_update_tasks({
  task_ids: ["abc123-def456-789...", "def456-789abc-123..."],
  status: "done"
});
```

---

### 5. Collaboration Tools

#### `mcp__vibe_kanban__add_task_comment`

Add a comment to a task for notes, progress updates, or information.

**Parameters:**
- `task_id` (UUID, **required**)
- `content` (string, **required**) - Comment text (cannot be empty)
- `author` (string, **required**) - Author name (e.g., "Ferris", "Miley")

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

**Example:**
```typescript
const result = await mcp__vibe_kanban__add_task_comment({
  task_id: "abc123-def456-789...",
  content: "Implemented agent metadata tracking. Added add_agent_metadata and get_agent_metadata MCP tools.",
  author: "Ferris"
});
```

**What it creates:**
- New TaskComment record

---

#### `mcp__vibe_kanban__get_task_comments`

Get all comments for a task (chronological order, oldest first).

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

**Example:**
```typescript
const result = await mcp__vibe_kanban__get_task_comments({
  task_id: "abc123-def456-789..."
});
```

---

#### `mcp__vibe_kanban__add_agent_metadata`

Add agent metadata to a task to track which agents worked on it.

**Parameters:**
- `task_id` (UUID, **required**)
- `agent_name` (string, **required**) - Agent name (e.g., "Ferris", "Miley", "Bree")
- `action` (string, **required**) - Action performed (e.g., "started", "completed", "updated")
- `summary` (string, optional) - Description of what was done

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

**Example:**
```typescript
const result = await mcp__vibe_kanban__add_agent_metadata({
  task_id: "abc123-def456-789...",
  agent_name: "Ferris",
  action: "completed",
  summary: "Implemented agent metadata tracking with full MCP integration"
});
```

**What it modifies:**
- Appends to `agent_metadata` JSON array on Task

---

#### `mcp__vibe_kanban__get_agent_metadata`

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
const result = await mcp__vibe_kanban__get_agent_metadata({
  task_id: "abc123-def456-789..."
});
// Shows complete history of agent activity on this task
```

---

### 6. Workspace Management

#### `mcp__vibe_kanban__start_workspace_session`

Start working on a task by creating a new workspace session. Supports two modes: full worktree mode (container-based) or lightweight branch-only mode.

**Parameters:**
- `task_id` (UUID, **required**)
- `executor` (string, **required**) - Executor type:
  - Standard executors: 'CLAUDE_CODE', 'CODEX', 'GEMINI', 'CURSOR_AGENT', 'OPENCODE'
  - Special: 'ORCHESTRATOR_MANAGED' (for orchestrator-controlled workflows)
- `repos` (Array<McpWorkspaceRepoInput>, **required**) - At least one repo:
  - `repo_id` (UUID) - Repository ID
  - `base_branch` (string) - Base/target branch (e.g., "main")
- `variant` (string, optional) - Executor variant if needed
- `agent_name` (string, optional) - Agent name for metadata logging (e.g., "Ferris", "Miley")
- `mode` (string, optional) - Workspace mode: 'worktree' (default) or 'branch'

**Executor Behavior:**

| Executor | Spawns Process | Default Mode | Use Case |
|----------|----------------|--------------|----------|
| `CLAUDE_CODE` | Yes | `worktree` | Standard agent execution |
| `CODEX` | Yes | `worktree` | Standard agent execution |
| `GEMINI` | Yes | `worktree` | Standard agent execution |
| `CURSOR_AGENT` | Yes | `worktree` | Standard agent execution |
| `OPENCODE` | Yes | `worktree` | Standard agent execution |
| `ORCHESTRATOR_MANAGED` | **No** | `branch` (forced) | Orchestrator dispatches sub-agents |

**Mode Behavior:**

| Mode | Creates Worktree | Creates Container | Branch Creation | Returns Working Directory |
|------|------------------|-------------------|-----------------|---------------------------|
| `worktree` | Yes | Optional | Yes | Worktree path |
| `branch` | No | No | Yes | Project root (repo path) |

**ORCHESTRATOR_MANAGED Executor:**

The `ORCHESTRATOR_MANAGED` executor is specifically designed for orchestrator-controlled workflows where:
- The orchestrator dispatches sub-agents via the Task() tool
- Sub-agents manage their own processes
- No automatic executor process is spawned
- Workspace tracking and git branch management is still required

Key characteristics:
- **Forces `mode: "branch"`** - worktree mode is not allowed
- **Does NOT spawn any process** - no executor process is started
- **Creates workspace records** - full database tracking
- **Creates git branch** - branch created immediately in each repo
- **Sets `setup_completed_at` immediately** - no async wait needed
- **Auto-assigns task** - if `agent_name` is provided
- **Logs agent metadata** - if `agent_name` is provided

**Returns:**
```typescript
{
  task_id: string;
  workspace_id: string;
  mode: string;              // 'worktree' or 'branch'
  executor: string;          // Executor type used
  repos: Array<{
    repo_id: string;
    branch_name: string;     // Git branch created
    base_branch: string;     // Base branch specified
    working_directory: string; // Path to work in
  }>;
}
```

**What it creates:**
1. New Workspace record with generated branch name
2. WorkspaceRepo records for each repo
3. Session record
4. AgentMetadataEntry if `agent_name` provided
5. Auto-assigns task if `agent_name` provided
6. **For worktree mode:** Triggers container/worktree creation (async)
7. **For branch mode:** Creates git branch immediately, sets `setup_completed_at`, no worktree/container

**Branch Name Format:**
```
vk-{workspace_id}-{sanitized_task_title}
```

**Example (ORCHESTRATOR_MANAGED - Recommended for Orchestrators):**
```typescript
// Orchestrator creates workspace for a sub-agent to work on
const result = await mcp__vibe_kanban__start_workspace_session({
  task_id: "abc123-def456-789...",
  executor: "ORCHESTRATOR_MANAGED",  // No process spawned
  repos: [
    {
      repo_id: "repo123-456...",
      base_branch: "main"
    }
  ],
  agent_name: "Ferris"  // Agent that will be dispatched
});
// {
//   task_id: "abc123-def456-789...",
//   workspace_id: "workspace456-789...",
//   mode: "branch",  // Always branch for ORCHESTRATOR_MANAGED
//   executor: "ORCHESTRATOR_MANAGED",
//   repos: [{
//     repo_id: "repo123-456...",
//     branch_name: "vk-workspace456-implement-agent-metadata",
//     base_branch: "main",
//     working_directory: "/Users/dev/projects/vibe-kanban"
//   }]
// }

// Orchestrator then dispatches sub-agent with branch info
await Task({
  subagent_type: "rust-supervisor",
  prompt: `
    Task ID: ${result.task_id}
    Workspace ID: ${result.workspace_id}
    Branch: ${result.repos[0].branch_name}
    Working Directory: ${result.repos[0].working_directory}

    Please checkout the branch and work on the task.
  `
});
```

**Example (Branch Mode - Lightweight):**
```typescript
const result = await mcp__vibe_kanban__start_workspace_session({
  task_id: "abc123-def456-789...",
  executor: "CLAUDE_CODE",
  repos: [
    {
      repo_id: "repo123-456...",
      base_branch: "main"
    }
  ],
  agent_name: "Ferris",
  mode: "branch"
});
// {
//   task_id: "abc123-def456-789...",
//   workspace_id: "workspace456-789...",
//   mode: "branch",
//   executor: "CLAUDE_CODE",
//   repos: [{
//     repo_id: "repo123-456...",
//     branch_name: "vk-workspace456-implement-agent-metadata",
//     base_branch: "main",
//     working_directory: "/Users/dev/projects/vibe-kanban"
//   }]
// }
```

**Example (Worktree Mode - Full Isolation):**
```typescript
const result = await mcp__vibe_kanban__start_workspace_session({
  task_id: "abc123-def456-789...",
  executor: "CLAUDE_CODE",
  repos: [
    {
      repo_id: "repo123-456...",
      base_branch: "main"
    }
  ],
  agent_name: "Ferris"
  // mode: "worktree" is default, no need to specify
});
// {
//   task_id: "abc123-def456-789...",
//   workspace_id: "workspace456-789...",
//   mode: "worktree",
//   executor: "CLAUDE_CODE",
//   repos: [{
//     repo_id: "repo123-456...",
//     branch_name: "vk-workspace456-implement-agent-metadata",
//     base_branch: "main",
//     working_directory: "/tmp/vk-worktrees/workspace456/vibe-kanban"
//   }]
// }
```

---

### 7. Audit Tools

#### `mcp__vibe_kanban__get_task_history`

Get the change history for a task (complete audit trail).

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
const result = await mcp__vibe_kanban__get_task_history({
  task_id: "abc123-def456-789..."
});
// Shows all modifications to the task
```

---

### 8. Git/PR Operations

#### `mcp__vibe_kanban__push_workspace_branch`

Push a workspace branch to GitHub.

**Parameters:**
- `workspace_id` (UUID, **required**)
- `repo_id` (UUID, **required**)
- `force` (boolean, optional) - Whether to force push (default: false)

**Returns:**
```typescript
{
  success: boolean;
  branch_name: string;
  remote_url?: string;
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__push_workspace_branch({
  workspace_id: "workspace456-789...",
  repo_id: "repo123-456...",
  force: false
});
// { success: true, branch_name: "vk-workspace456-implement-agent-metadata", remote_url: "..." }
```

**What it does:**
- Pushes the workspace branch to the remote repository
- Uses force push if `force=true`

---

#### `mcp__vibe_kanban__create_workspace_pr`

Create a GitHub Pull Request for a workspace.

**Parameters:**
- `workspace_id` (UUID, **required**)
- `repo_id` (UUID, **required**)
- `title` (string, **required**) - PR title
- `body` (string, optional) - PR description
- `target_branch` (string, optional) - Target branch (defaults to workspace's target branch)
- `draft` (boolean, optional) - Create as draft PR (default: false)

**Returns:**
```typescript
{
  pr_number: number;
  pr_url: string;
  status: string;  // e.g., "open"
}
```

**Example:**
```typescript
const result = await mcp__vibe_kanban__create_workspace_pr({
  workspace_id: "workspace456-789...",
  repo_id: "repo123-456...",
  title: "Implement agent metadata tracking",
  body: "This PR adds MCP tools for tracking agent activity on tasks.\n\nFixes #123",
  draft: false
});
// { pr_number: 42, pr_url: "https://github.com/owner/repo/pull/42", status: "open" }
```

**What it creates:**
- Merge record (type: Pr) with PR info
- GitHub Pull Request (via GitHub API)

**Prerequisites:**
- Workspace branch must be pushed to remote

---

#### `mcp__vibe_kanban__get_workspace_pr_status`

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

**Example:**
```typescript
const result = await mcp__vibe_kanban__get_workspace_pr_status({
  workspace_id: "workspace456-789...",
  repo_id: "repo123-456..."
});
// { has_pr: true, pr_number: 42, status: "open", ... }
```

**Data Source:** Database (cached PR status)

---

#### `mcp__vibe_kanban__refresh_workspace_pr_status`

Refresh PR status from GitHub API and update the database. **Auto-updates task to 'done' if PR is merged and task was 'inreview'.**

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

**Example:**
```typescript
const result = await mcp__vibe_kanban__refresh_workspace_pr_status({
  workspace_id: "workspace456-789...",
  repo_id: "repo123-456..."
});
// { pr_number: 42, previous_status: "open", current_status: "merged", status_changed: true, task_updated: true }
```

**What it does:**
1. Fetches latest PR status from GitHub
2. Updates Merge record in database
3. If PR status changed from 'open' → 'merged':
   - Finds the task for this workspace
   - If task status is 'inreview', updates it to 'done'

**Use Case:** Periodically check if PRs have been merged and auto-complete tasks.

---

## Orchestration Workflows

### Standard Task Creation and Dispatch

**Workflow:** Create a task, assign it to an agent, and dispatch the agent to work on it.

```typescript
// Step 1: Create task
const { task_id } = await mcp__vibe_kanban__create_task({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  title: "Implement agent metadata tracking",
  description: "Add MCP tools for tracking which agents work on tasks"
});

// Step 2: Assign to agent
await mcp__vibe_kanban__assign_task({
  task_id,
  assignee: "Ferris"
});

// Step 3: Start workspace with ORCHESTRATOR_MANAGED executor
const { workspace_id, repos } = await mcp__vibe_kanban__start_workspace_session({
  task_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [
    {
      repo_id: "repo123-456...",
      base_branch: "main"
    }
  ],
  agent_name: "Ferris"
});

// Step 4: Dispatch to supervisor with branch info
await Task({
  subagent_type: "rust-supervisor",
  prompt: `
    Task ID: ${task_id}
    Workspace ID: ${workspace_id}
    Branch: ${repos[0].branch_name}
    Working Directory: ${repos[0].working_directory}

    Title: Implement agent metadata tracking
    Description: Add MCP tools for tracking which agents work on tasks

    Please checkout the branch and implement this feature.
    Log your progress using add_task_comment.
    Mark the task as 'inreview' when complete.
  `
});

// Step 5: Monitor completion (supervisor reports back)
```

---

### Lightweight Task (No Workspace)

**Workflow:** For small fixes that don't need workspace isolation.

```typescript
// Quick fix - no task, no workspace
await Task({
  subagent_type: "worker",
  prompt: "Fix typo in README.md: change 'recieve' to 'receive'"
});
```

---

### Complete PR Workflow

**Workflow:** Task creation → work → push → PR creation → merge tracking.

```typescript
// (Task created and agent dispatched as shown above)

// Agent works on task, logs progress...

// Agent completes work and reports back
// Orchestrator continues:

// Step 6: Push branch (if agent hasn't already)
await mcp__vibe_kanban__push_workspace_branch({
  workspace_id,
  repo_id: "repo123-456..."
});

// Step 7: Create PR
const { pr_number, pr_url } = await mcp__vibe_kanban__create_workspace_pr({
  workspace_id,
  repo_id: "repo123-456...",
  title: "Implement agent metadata tracking",
  body: "This PR adds MCP tools for tracking agent activity on tasks.\n\nFixes #123"
});

// Step 8: Update task status to inreview
await mcp__vibe_kanban__update_task({
  task_id,
  status: "inreview"
});

// Step 9: Add comment with PR link
await mcp__vibe_kanban__add_task_comment({
  task_id,
  content: `PR created: ${pr_url}`,
  author: "Orchestrator"
});

// Step 10: Periodic status check (run this in a loop or on user request)
const { status_changed, task_updated } = await mcp__vibe_kanban__refresh_workspace_pr_status({
  workspace_id,
  repo_id: "repo123-456..."
});

if (status_changed) {
  console.log("PR status changed!");
}

if (task_updated) {
  console.log("Task automatically marked as done!");
}
```

---

### Context Preservation on Bug Reports

**Workflow:** When a user reports a bug in recently completed work, re-dispatch to the same agent with full context.

```typescript
// Step 1: Find the related task
const { tasks } = await mcp__vibe_kanban__search_tasks({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  query: "agent metadata"
});

const task_id = tasks[0].id;

// Step 2: Get full context
const { task } = await mcp__vibe_kanban__get_task({ task_id });
const { history } = await mcp__vibe_kanban__get_task_history({ task_id });
const { comments } = await mcp__vibe_kanban__get_task_comments({ task_id });
const { metadata } = await mcp__vibe_kanban__get_agent_metadata({ task_id });

// Step 3: Identify original agent
const original_agent = task.assignee; // "Ferris"

// Step 4: Re-dispatch to SAME agent with full context
await Task({
  subagent_type: "rust-supervisor",
  prompt: `
    Task ID: ${task_id}
    Title: ${task.title}
    Status: ${task.status}

    USER REPORTED BUG: [bug description]

    CONTEXT - You originally implemented this:

    Previous Comments:
    ${comments.map(c => `${c.author}: ${c.content}`).join('\n')}

    Agent Activity:
    ${metadata.map(m => `${m.agent_name} ${m.action}: ${m.summary || ''}`).join('\n')}

    Please investigate and fix the issue.
    Add a comment when you find the root cause.
  `
});
```

---

## Multi-Agent Coordination Patterns

### Pattern 1: Sequential Task Chain

**Scenario:** Backend implementation must complete before frontend can start.

```typescript
// Create backend task
const { task_id: backend_task_id } = await mcp__vibe_kanban__create_task({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  title: "Backend: Add agent metadata API endpoints",
  description: "Implement POST /api/tasks/:id/agent-metadata and GET /api/tasks/:id/agent-metadata"
});

// Dispatch backend agent
const { workspace_id: backend_workspace_id, repos: backend_repos } = await mcp__vibe_kanban__start_workspace_session({
  task_id: backend_task_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [{ repo_id: "repo123...", base_branch: "main" }],
  agent_name: "Ferris"
});

await Task({
  subagent_type: "rust-supervisor",
  prompt: `
    Task ID: ${backend_task_id}
    Branch: ${backend_repos[0].branch_name}

    Implement agent metadata API endpoints.
    Mark as 'inreview' when complete.
  `
});

// Wait for backend completion (agent marks task as 'inreview')
// Then create frontend task

const { task_id: frontend_task_id } = await mcp__vibe_kanban__create_task({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  title: "Frontend: Display agent metadata in task details",
  description: `Related backend task: ${backend_task_id}\n\nUse the new agent metadata API to show which agents worked on a task.`
});

// Dispatch frontend agent
const { workspace_id: frontend_workspace_id, repos: frontend_repos } = await mcp__vibe_kanban__start_workspace_session({
  task_id: frontend_task_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [{ repo_id: "repo123...", base_branch: "main" }],
  agent_name: "Miley"
});

await Task({
  subagent_type: "frontend-supervisor",
  prompt: `
    Task ID: ${frontend_task_id}
    Branch: ${frontend_repos[0].branch_name}

    Backend endpoints are ready in task ${backend_task_id}.
    Implement UI to display agent metadata.
    Mark as 'inreview' when complete.
  `
});
```

---

### Pattern 2: Parallel Independent Tasks

**Scenario:** Multiple agents can work simultaneously on unrelated features.

```typescript
// Create multiple tasks
const { task_id: task1_id } = await mcp__vibe_kanban__create_task({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  title: "Add bulk task update endpoint",
  description: "..."
});

const { task_id: task2_id } = await mcp__vibe_kanban__create_task({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  title: "Implement task search with full-text indexing",
  description: "..."
});

// Start workspaces
const { workspace_id: ws1_id, repos: repos1 } = await mcp__vibe_kanban__start_workspace_session({
  task_id: task1_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [{ repo_id: "repo123...", base_branch: "main" }],
  agent_name: "Ferris"
});

const { workspace_id: ws2_id, repos: repos2 } = await mcp__vibe_kanban__start_workspace_session({
  task_id: task2_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [{ repo_id: "repo123...", base_branch: "main" }],
  agent_name: "Bree"
});

// Dispatch both agents in parallel
const agent1 = Task({
  subagent_type: "rust-supervisor",
  prompt: `Task ID: ${task1_id}\nBranch: ${repos1[0].branch_name}\n\n...`,
  run_in_background: true
});

const agent2 = Task({
  subagent_type: "rust-supervisor",
  prompt: `Task ID: ${task2_id}\nBranch: ${repos2[0].branch_name}\n\n...`,
  run_in_background: true
});

// Wait for both to complete
await TaskOutput({ task_id: agent1 });
await TaskOutput({ task_id: agent2 });
```

---

### Pattern 3: Escalation to Detective

**Scenario:** Agent encounters a bug they can't solve, escalate to detective.

```typescript
// Agent reports issue (from within agent):
await mcp__vibe_kanban__add_task_comment({
  task_id: "abc123...",
  content: "Encountered issue: tests failing with segfault. Unable to determine root cause after 2 hours investigation.",
  author: "Ferris"
});

// Orchestrator detects stuck agent and escalates:
await Task({
  subagent_type: "detective",
  prompt: `
    Task ID: abc123...

    Agent Ferris is stuck on a segfault issue.
    Review the code changes, run debugger, and identify root cause.
    Document your findings in a comment on the task.

    Once you identify the issue, report back to me and I'll re-dispatch Ferris with your findings.
  `
});
```

---

### Pattern 4: Architect-Led Planning

**Scenario:** Large feature requiring planning before implementation.

```typescript
// Step 1: Dispatch architect for planning
await Task({
  subagent_type: "architect",
  prompt: `
    Plan implementation for: Multi-workspace task support

    Requirements:
    - Tasks can have multiple workspaces (for retries)
    - Each workspace tracks its own PR
    - Frontend shows workspace history

    Provide:
    1. Breakdown into subtasks
    2. Task order/dependencies
    3. Database schema changes
    4. API endpoints needed
  `
});

// Step 2: Architect provides plan (received from agent output)
// Create tasks based on plan:

const task_ids = [];

for (const subtask of architect_plan.subtasks) {
  const { task_id } = await mcp__vibe_kanban__create_task({
    project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
    title: subtask.title,
    description: subtask.description
  });

  task_ids.push(task_id);
}

// Step 3: Dispatch implementing agents in order
for (const task_id of task_ids) {
  // Start workspace and dispatch appropriate supervisor
  // ...
}
```

---

## Branch-Only vs Container-Based Modes

### Branch-Only Mode (`mode: "branch"`)

**Best For:**
- Orchestrator-controlled workflows (recommended for orchestrators)
- Agents already running in the repo
- Lightweight task tracking without full isolation
- Faster workspace startup

**Characteristics:**
- Creates git branch immediately
- No worktree or container created
- Agent works in original repo directory
- `setup_completed_at` set immediately (no async wait)
- Cheaper resource usage

**When to Use:**
- **You are an orchestrator** dispatching sub-agents via Task() tool
- Agent manages its own process lifecycle
- Isolation isn't critical (low risk of conflicts)
- Speed is prioritized over isolation

**Example:**
```typescript
const { workspace_id, repos } = await mcp__vibe_kanban__start_workspace_session({
  task_id: "abc123...",
  executor: "ORCHESTRATOR_MANAGED",  // Forces branch mode
  repos: [{ repo_id: "repo123...", base_branch: "main" }],
  agent_name: "Ferris"
});
// workspace ready immediately, agent works in /Users/dev/projects/my-repo
```

---

### Container-Based Mode (`mode: "worktree"`)

**Best For:**
- Full workspace isolation
- Containerized execution environments
- Multiple agents working on different tasks simultaneously
- Long-running agent processes

**Characteristics:**
- Creates dedicated git worktree
- Optional Docker container
- Complete isolation from other workspaces
- Async setup (wait for `setup_completed_at`)
- Higher resource usage

**When to Use:**
- Executor spawns and manages its own process
- Full isolation is required
- Multiple concurrent workspaces needed
- Agent runs in containerized environment

**Example:**
```typescript
const { workspace_id, repos } = await mcp__vibe_kanban__start_workspace_session({
  task_id: "abc123...",
  executor: "CLAUDE_CODE",
  repos: [{ repo_id: "repo123...", base_branch: "main" }],
  agent_name: "Ferris"
  // mode: "worktree" is default
});
// workspace setup happens asynchronously
// agent will work in /tmp/vk-worktrees/workspace456/my-repo
```

---

### Comparison Table

| Feature | Branch-Only | Container-Based |
|---------|-------------|-----------------|
| **Branch Creation** | Immediate | Immediate |
| **Worktree Creation** | No | Yes |
| **Container Creation** | No | Optional |
| **Setup Time** | Instant | Async (seconds) |
| **Resource Usage** | Low | Medium-High |
| **Isolation Level** | Low | High |
| **Working Directory** | Project root | Dedicated worktree |
| **Process Management** | External | Managed by executor |
| **Best For** | Orchestrators | Executor-managed agents |
| **Cleanup Required** | Branch only | Branch + worktree + container |

---

## Best Practices

### For Orchestrators

1. **Always use ORCHESTRATOR_MANAGED executor**
   ```typescript
   // Good
   await mcp__vibe_kanban__start_workspace_session({
     task_id,
     executor: "ORCHESTRATOR_MANAGED",
     repos: [...],
     agent_name: "Ferris"
   });

   // Bad - spawns unwanted process
   await mcp__vibe_kanban__start_workspace_session({
     task_id,
     executor: "CLAUDE_CODE",  // Don't do this as orchestrator
     repos: [...]
   });
   ```

2. **Check Kanban before creating tasks**
   ```typescript
   // Always check first
   const { tasks } = await mcp__vibe_kanban__list_tasks({
     project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f"
   });

   // Check if task already exists for this work
   const existingTask = tasks.find(t => t.title.includes("agent metadata"));
   if (existingTask) {
     // Resume existing task instead of creating duplicate
   }
   ```

3. **Provide full context on dispatch**
   ```typescript
   await Task({
     subagent_type: "rust-supervisor",
     prompt: `
       Task ID: ${task_id}
       Workspace ID: ${workspace_id}
       Branch: ${repos[0].branch_name}
       Working Directory: ${repos[0].working_directory}

       Title: ${task.title}
       Description: ${task.description}

       Instructions:
       1. Checkout the branch
       2. Implement the feature
       3. Log progress with add_task_comment
       4. Mark as 'inreview' when complete
     `
   });
   ```

4. **Never mark tasks as 'done' yourself**
   - Only mark as `inreview` after verification
   - Let the system auto-mark as `done` when PR merges
   - Or manually mark after user approval

5. **Preserve context on re-dispatch**
   ```typescript
   // When bug reported, get full context first
   const { history } = await mcp__vibe_kanban__get_task_history({ task_id });
   const { comments } = await mcp__vibe_kanban__get_task_comments({ task_id });
   const { metadata } = await mcp__vibe_kanban__get_agent_metadata({ task_id });

   // Re-dispatch to same agent with context
   await Task({
     subagent_type: "rust-supervisor",
     prompt: `
       Bug reported in your previous work...
       Your previous comments: ${comments.map(c => c.content).join('\n')}
       ...
     `
   });
   ```

---

### For Implementing Supervisors

1. **Log progress during work**
   ```typescript
   // After completing milestone
   await mcp__vibe_kanban__add_task_comment({
     task_id: "abc123...",
     content: "Implemented add_agent_metadata endpoint. Added tests.",
     author: "Ferris"
   });
   ```

2. **Always add summary before marking inreview**
   ```typescript
   // Final summary
   await mcp__vibe_kanban__add_task_comment({
     task_id: "abc123...",
     content: "Completed: Agent metadata tracking. Files: task_server.rs, models/task.rs. Tests: passing. Ready for review.",
     author: "Ferris"
   });

   // Then update status
   await mcp__vibe_kanban__update_task({
     task_id: "abc123...",
     status: "inreview"
   });
   ```

3. **Verify your work before marking inreview**
   - Run tests
   - Test the feature manually
   - Check for regressions
   - Only then mark as `inreview`

4. **Use agent metadata to track your activity**
   ```typescript
   // When starting
   await mcp__vibe_kanban__add_agent_metadata({
     task_id: "abc123...",
     agent_name: "Ferris",
     action: "started",
     summary: "Beginning work on agent metadata tracking"
   });

   // When completing
   await mcp__vibe_kanban__add_agent_metadata({
     task_id: "abc123...",
     agent_name: "Ferris",
     action: "completed",
     summary: "Implemented full MCP integration with tests"
   });
   ```

---

### Task Size Guidelines

**Small Tasks (<30 lines, single file):**
- Don't create Kanban task
- Dispatch worker directly
- Quick turnaround

**Medium Tasks (2-5 files, 30-200 lines):**
- Create Kanban task
- Assign to appropriate supervisor
- Single focused deliverable

**Large Tasks (6+ files, feature, architecture change):**
- Dispatch architect for planning
- Break into multiple medium tasks
- Create separate Kanban task for each deliverable

---

## Troubleshooting

### Issue: Task marked as 'done' prematurely

**Cause:** Supervisor marked task as 'done' instead of 'inreview'

**Solution:**
```typescript
// Update task back to inreview
await mcp__vibe_kanban__update_task({
  task_id: "abc123...",
  status: "inreview"
});

// Add comment explaining
await mcp__vibe_kanban__add_task_comment({
  task_id: "abc123...",
  content: "Status corrected - tasks should only be marked 'done' by orchestrator after user approval",
  author: "Orchestrator"
});
```

---

### Issue: Duplicate tasks created

**Cause:** Not checking existing tasks before creating new ones

**Solution:**
```typescript
// Always search first
const { tasks } = await mcp__vibe_kanban__search_tasks({
  project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f",
  query: "agent metadata"
});

if (tasks.length > 0) {
  // Use existing task
  const task_id = tasks[0].id;
} else {
  // Create new task
  const { task_id } = await mcp__vibe_kanban__create_task({...});
}
```

---

### Issue: Agent can't find branch

**Cause:** Branch name not communicated to agent

**Solution:**
```typescript
// Always include branch info in dispatch
const { workspace_id, repos } = await mcp__vibe_kanban__start_workspace_session({
  task_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [{ repo_id: "repo123...", base_branch: "main" }],
  agent_name: "Ferris"
});

await Task({
  subagent_type: "rust-supervisor",
  prompt: `
    Task ID: ${task_id}
    Branch: ${repos[0].branch_name}  // Include this!
    Working Directory: ${repos[0].working_directory}

    Please checkout the branch: git checkout ${repos[0].branch_name}
  `
});
```

---

### Issue: Context lost when fixing bugs

**Cause:** Not retrieving task history before re-dispatch

**Solution:**
```typescript
// Get full context
const { task } = await mcp__vibe_kanban__get_task({ task_id });
const { comments } = await mcp__vibe_kanban__get_task_comments({ task_id });
const { metadata } = await mcp__vibe_kanban__get_agent_metadata({ task_id });

// Include in re-dispatch
await Task({
  subagent_type: "rust-supervisor",
  prompt: `
    Task ID: ${task_id}
    Original Implementation: ${metadata.find(m => m.action === 'completed')?.summary}
    Previous Comments: ${comments.map(c => c.content).join('\n\n')}

    Bug: [description]
  `
});
```

---

### Issue: PR not auto-completing task

**Cause:** Task not in 'inreview' status when PR merged

**Solution:**
```typescript
// Ensure task is in correct status before PR merges
await mcp__vibe_kanban__update_task({
  task_id,
  status: "inreview"
});

// Then refresh status will auto-complete if merged
const { task_updated } = await mcp__vibe_kanban__refresh_workspace_pr_status({
  workspace_id,
  repo_id: "repo123..."
});
```

---

### Issue: ORCHESTRATOR_MANAGED not working with worktree mode

**Cause:** ORCHESTRATOR_MANAGED forces branch-only mode

**Solution:**
```typescript
// This will fail
await mcp__vibe_kanban__start_workspace_session({
  task_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [...],
  mode: "worktree"  // Error: ORCHESTRATOR_MANAGED requires mode='branch'
});

// Correct usage
await mcp__vibe_kanban__start_workspace_session({
  task_id,
  executor: "ORCHESTRATOR_MANAGED",
  repos: [...]
  // mode: "branch" is automatically set
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
- `CLAUDE_CODE` - Claude Code (standard execution)
- `CODEX` - OpenAI Codex
- `GEMINI` - Google Gemini
- `CURSOR_AGENT` - Cursor Agent
- `OPENCODE` - OpenCode
- `ORCHESTRATOR_MANAGED` - No process spawned (orchestrator-controlled)

### Common Agent Names
- `Ferris` - Rust/Backend supervisor
- `Miley` - Frontend supervisor
- `Bree` - Worker (small fixes)
- `Emilia` - Infrastructure supervisor
- `Ivy` - Scout (codebase exploration)
- `Vera` - Detective (debugging)
- `Ada` - Architect (planning)
- `Penny` - Scribe (documentation)

---

**End of MCP Orchestration Guide**

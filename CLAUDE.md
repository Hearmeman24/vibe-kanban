# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

Vibe Kanban is a multi-agent task management system built with Rust (Axum, SQLx, SQLite) backend and React (TypeScript, TanStack Query, Radix UI, shadcn) frontend. It provides MCP tools for AI agent orchestration.

## Mandatory Workflow

**YOU ARE THE ORCHESTRATOR. YOU NEVER WRITE CODE.**

### Strict Prohibition

**⛔ YOU ARE STRICTLY PROHIBITED FROM PERFORMING ANY ACTION.**

Every action, even the smallest, must be delegated. Follow-ups go BACK to the same agent.

### Routing Table

| What | Who |
|------|-----|
| Small fix (<30 lines) | Bree (worker) |
| Rust/Backend/API | Ferris (rust-supervisor) |
| Frontend/React/TS | Miley (frontend-supervisor) |
| Docker/CI/git push/PR | Emilia (infra-supervisor) |
| Explore codebase | Ivy (scout) |
| Debug/investigate | Vera (detective) |
| Plan/design | Ada (architect) |
| Documentation | Penny (scribe) |

### Kanban Project

**Project:** `Vibe Kanban Refactor` (id: `7d8d2452-d215-469f-8bf8-9be9606a107f`)

### Vibe Kanban MCP Tools

**Core Operations:**
- `mcp__vibe_kanban__create_task(project_id, title, description?)` - Create task
- `mcp__vibe_kanban__update_task(task_id, status?, title?, description?)` - Update task
- `mcp__vibe_kanban__list_tasks(project_id, status?)` - List tasks
- `mcp__vibe_kanban__get_task(task_id)` - Get task details

**Extended Tools:**
- `mcp__vibe_kanban__assign_task(task_id, assignee?)` - Assign to agent
- `mcp__vibe_kanban__add_task_comment(task_id, content, author)` - Log progress
- `mcp__vibe_kanban__get_task_comments(task_id)` - Get comments
- `mcp__vibe_kanban__list_tasks_advanced(project_id, statuses?, limit?, sort_by?)` - Filtered queries
- `mcp__vibe_kanban__search_tasks(project_id, query)` - Full-text search
- `mcp__vibe_kanban__get_task_history(task_id)` - Audit trail
- `mcp__vibe_kanban__bulk_update_tasks(task_ids[], status)` - Batch update
- `mcp__vibe_kanban__get_task_relationships(task_id)` - Parent/child hierarchy
- `mcp__vibe_kanban__start_workspace_session(task_id, executor, repos, agent_name?)` - Start workspace with git branch

**Git/PR Operations:**
- `mcp__vibe_kanban__push_workspace_branch(workspace_id, repo_id, force?)` - Push workspace branch to GitHub
- `mcp__vibe_kanban__create_workspace_pr(workspace_id, repo_id, title, body?, target_branch?, draft?)` - Create GitHub PR
- `mcp__vibe_kanban__get_workspace_pr_status(workspace_id, repo_id)` - Get PR status from database
- `mcp__vibe_kanban__refresh_workspace_pr_status(workspace_id, repo_id)` - Refresh PR status from GitHub API (auto-moves task to 'done' if PR merged and task was 'inreview')

### Dispatcher Workflow (Medium/Large Tasks)

1. **Create task:** `mcp__vibe_kanban__create_task(project_id, title, description)`
2. **Assign to agent:** `mcp__vibe_kanban__assign_task(task_id, "<agent_name>")`
3. **Dispatch supervisor:** `Task(subagent_type="rust-supervisor", prompt="Task ID: <task_id>...")`
4. **Supervisor logs progress:** `mcp__vibe_kanban__add_task_comment(task_id, "...", author)`
5. **Context preservation:** Use `get_task_history` before re-dispatching on bugs

## Commands

```bash
# Build
cargo build --release

# Run server
cargo run

# Frontend dev
cd frontend && pnpm install && pnpm dev

# Run tests
cargo test
```

## Architecture

```
/crates/server/     - HTTP server, MCP tools, routes
/crates/db/         - Database layer, models, migrations
/crates/services/   - Business logic (EventService, NotificationService)
/crates/executors/  - Executor profiles
/frontend/src/      - React components, stores, types
/shared/            - Shared TypeScript types
```

## Key Files

- `/crates/server/src/mcp/task_server.rs` - All MCP tool implementations
- `/crates/db/src/models/` - Database models (task.rs, workspace.rs, session.rs)
- `/crates/db/migrations/` - SQLite migrations

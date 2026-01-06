# Orchestration Workflows - vibe-kanban

Complete routing table, context preservation rules, and parallel work protocols.

## Routing Table

### Task Classification

**SMALL TASKS (<30 lines)** → Direct dispatch, no Kanban
- Typos, one-line fixes, simple changes
- Configuration updates
- Single file changes
- Dispatch: `Task(subagent_type='worker', prompt='...')`

**MEDIUM TASKS (30-200 lines)** → Kanban required
- Feature additions with multiple components
- Non-trivial bug fixes with dependencies
- Single-module enhancements
- Dispatch: Create task → Get task_id → `Task(subagent_type=..., prompt='Task ID: {id}\n...')`

**LARGE TASKS (200+ lines)** → Kanban required, consider breakdown
- Multi-module features
- Significant refactors
- Cross-service changes
- Dispatch: Create task → Plan → Create subtasks → Dispatch to supervisors

---

## Complete Routing Matrix

| Work Category | Size | Agent | Task? | Dispatch Method |
|---|---|---|---|---|
| **Backend - Rust** ||||
| New API endpoint | Medium | Nova | Yes | Create task → dispatch with task_id |
| Bug in crate | Medium | Nova | Yes | Create task → dispatch with task_id |
| Quick fix in crate | Small | Bree | No | Direct dispatch |
| Add test | Medium | Nova | Yes | Create task → dispatch with task_id |
| Async handler | Medium | Nova | Yes | Create task → dispatch with task_id |
| **Frontend - React** ||||
| New component | Medium | Miley | Yes | Create task → dispatch with task_id |
| Component bug | Medium | Miley | Yes | Create task → dispatch with task_id |
| Styling update | Small | Bree | No | Direct dispatch |
| Add hook logic | Medium | Miley | Yes | Create task → dispatch with task_id |
| Fix prop type | Small | Bree | No | Direct dispatch |
| **DevOps/Infra** ||||
| New GitHub Actions workflow | Medium | Emilia | Yes | Create task → dispatch with task_id |
| Docker fix | Medium | Emilia | Yes | Create task → dispatch with task_id |
| Update deployment | Medium | Emilia | Yes | Create task → dispatch with task_id |
| **Investigation** ||||
| Find where code lives | Small | Ivy | No | Direct dispatch |
| Debug issue | Medium | Vera | No | Direct dispatch |
| **Planning** ||||
| Design feature | Small | Ada | No | Direct dispatch |
| Plan refactor | Medium | Ada | No | Direct dispatch |
| **Documentation** ||||
| Write docs | Small | Penny | No | Direct dispatch |
| Update README | Small | Penny | No | Direct dispatch |

---

## Dispatch Workflow Detail

### Small Tasks (Bree - Worker)

```bash
Task(
  subagent_type='worker',
  prompt='''Fix the typo in [file] on line [number]: change "X" to "Y"
  Context: This is a simple one-line fix.'''
)
```

**No Kanban tracking needed.** Bree reports completion in her format.

### Medium/Large Tasks (Implementation Agents)

**Step 1: Check Kanban**
```bash
mcp__vibe_kanban__list_tasks(project_id)
```
Look for duplicate/existing tasks.

**Step 2: Create Task** (if new)
```bash
mcp__vibe_kanban__create_task(
  project_id,
  title="Implement [feature]",
  description="Details..."
)
→ Returns: task_id (UUID)
```

**Step 3: Start Workspace**
```bash
mcp__vibe_kanban__start_workspace_session(
  task_id="{task_id}",
  executor="ORCHESTRATOR_MANAGED",
  repos=[{
    repo_id="{repo_id}",
    base_branch="main"
  }],
  agent_name="Nova"  # or Miley, Emilia, etc.
)
→ Returns: workspace_id, branch_name, working_directory
```

**Step 4: Dispatch Supervisor**
```bash
Task(
  subagent_type='worker',  # or specify supervisor type
  prompt='''TASK: Implement user authentication

Task ID: {task_id}
Workspace ID: {workspace_id}
Branch: {branch_name}

Details:
- Implement JWT token generation in auth module
- Add login endpoint to API
- Create authentication middleware

Files to modify:
- crates/services/auth.rs
- crates/server/handlers/auth.rs
- etc.

When done:
1. Commit your changes
2. Mark task as "inreview": mcp__vibe_kanban__update_task(task_id, status="inreview")
3. Report completion

Reference implementation plan: [link to Ada's plan if created]
'''
)
```

**Step 5: Supervisor Completes Work**
- Checks out branch
- Makes changes
- Commits to branch (with task ID in message)
- Moves task to "inreview"
- Adds completion comment

**Step 6: Orchestrator Handles PR/Merge** (if remote repo)
- Push branch: `mcp__vibe_kanban__push_workspace_branch(workspace_id, repo_id)`
- Create PR: `mcp__github__create_pull_request(...)`
- Link PR to task: `mcp__vibe_kanban__add_task_comment(...)`
- Monitor PR: `mcp__github__pull_request_read(method='get', ...)`
- Auto-complete when merged

---

## Non-Implementation Agents (Always Direct)

### Ivy - Scout (Code Exploration)

**Never needs task.** Direct dispatch for exploration:

```bash
Task(
  subagent_type='Explore',
  description='quick',  # or 'medium' or 'very thorough'
  prompt='Find all files that handle authentication in the Rust crates. Show me the structure.'
)
```

Returns: File list, structure, relationships.

### Vera - Detective (Debugging)

**Never needs task.** Direct dispatch for investigation:

```bash
Task(
  subagent_type='detective',
  prompt='The server crashes on startup with "thread 'main' panicked at \"connection refused\"". Root cause and fix.'
)
```

Returns: Root cause analysis + suggested fix → dispatch to appropriate agent to implement.

### Ada - Architect (Planning)

**Never needs task.** Direct dispatch for design:

```bash
Task(
  subagent_type='Plan',
  prompt='Design the implementation plan for adding WebSocket support to the API. Consider the current Axum setup and multiple crate structure.'
)
```

Returns: Detailed implementation plan → use for dispatch prompts to Nova.

### Penny - Scribe (Documentation)

**Never needs task.** Direct dispatch for docs:

```bash
Task(
  subagent_type='worker',
  prompt='Document the authentication module in crates/services/auth.rs. Create a markdown file explaining how the auth module works.'
)
```

Returns: Documentation created/updated.

---

## Context Preservation

### When to Dispatch Same Agent Again

**Preserve context** (dispatch same agent to same task area):
- Sequential work: agent finishes phase 1 → assign phase 2 directly
- Bug fix + enhancement: agent fixes bug → immediately enhance related code
- Implementation + testing: agent implements → immediately adds tests
- Related modules: agent finishes API endpoint → immediately adds middleware

Example:
```bash
# Nova finishes API endpoint, immediately continues with auth:
Task(
  subagent_type='nova_agent',  # or use rust-engineer
  prompt='Task ID: {same_task_id}

  You just completed the API endpoint. Now:
  1. Add input validation to the endpoint
  2. Add error handling for edge cases
  3. Write integration tests

  All in same branch, same task.'
)
```

### When to Create New Task

**Context break** (new task needed):
- Different agent responsibility
- Major feature shift
- Dependency on previous completion
- Different priority/timeline
- Different module/component

Example:
```bash
# Nova finishes API, Miley adds React components:
mcp__vibe_kanban__create_task(
  project_id,
  title="Build React UI for authentication",
  description="Add login form, token storage, auth context..."
)
→ task_id
# Then dispatch Miley with new task_id
```

---

## Parallel Work Protocol

### Independent Tasks (Run in Parallel)

If tasks don't depend on each other, dispatch multiple agents simultaneously:

```bash
# Parallel: Frontend and backend independent
Task(subagent_type='nova', prompt='Task ID: {task1_id}\nImplement notification API endpoint...')
Task(subagent_type='miley', prompt='Task ID: {task2_id}\nBuild notification UI components...')

# Both run concurrently
```

### Sequential Tasks (Wait for Completion)

If task B depends on task A:

```bash
# Task A must complete first
Task(subagent_type='nova', prompt='Task ID: {task1_id}\nImplement database schema...')
# Wait for completion, then:
Task(subagent_type='nova', prompt='Task ID: {task2_id}\nImplement CRUD endpoints...')
```

---

## Repository Configuration

**Type:** Remote Repository
**Git Remote:** https://github.com/[OWNER]/[REPO].git

> Add your actual repo details after first PR creation

**Base Branch:** main
**PR Strategy:** GitHub MCP for creation and merge

---

## Task Lifecycle

### 1. Creation
```
Status: todo
```

### 2. Dispatch
```
Status: inprogress
Agent: [assigned]
Comment: "Starting work on [task]"
```

### 3. Completion
```
Status: inreview
Agent adds comment with summary
Branch ready for PR
```

### 4. Code Review
```
PR created and linked
GitHub monitors status
```

### 5. Merge
```
PR merged → Auto-update task to done
Final comment with PR link
```

---

## Error Handling

### Task Blocked

If supervisor encounters blocker:

```bash
mcp__vibe_kanban__update_task(
  task_id,
  status='todo'  # revert if waiting
)

mcp__vibe_kanban__add_task_comment(
  task_id,
  author='Nova',
  content='Blocked: [reason]. Waiting for [dependency].'
)
```

Then dispatch different agent or create dependent task.

### Task Incomplete

If stopping without completion:

```bash
mcp__vibe_kanban__add_task_comment(
  task_id,
  content='In progress: [summary of work done]. Next step: [what remains].'
)

mcp__vibe_kanban__update_task(
  task_id,
  status='inprogress'  # or 'todo' if restarting
)
```

---

## Workflow Summary

1. **Small task?** → Direct dispatch to Bree
2. **Medium/large?** → Check Kanban → Create task (if new) → Start workspace → Dispatch agent
3. **Need plan?** → Ask Ada first (no task)
4. **Bug investigation?** → Ask Vera first (no task)
5. **Finding something?** → Ask Ivy first (no task)
6. **Documentation?** → Ask Penny (small) or create task (large)
7. **Agent finishes** → They move task to "inreview" automatically
8. **PR created** → Orchestrator handles GitHub workflow
9. **PR merged** → Task auto-completed

---

## MCP Tools Quick Reference

**Vibe Kanban:**
- `list_tasks` - Check what's in Kanban
- `create_task` - Create new task
- `start_workspace_session` - Initialize work on task
- `update_task` - Change status
- `add_task_comment` - Log progress

**GitHub (for remote repos):**
- `create_pull_request` - Create PR after supervisor finishes
- `pull_request_read` - Check PR status
- `merge_pull_request` - Merge when approved

**Vibe Kanban workspace management:**
- `push_workspace_branch` - Push supervisor's branch to remote
- `get_workspace_pr_status` - Check if PR exists
- `refresh_workspace_pr_status` - Sync PR status from GitHub

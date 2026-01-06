---
name: vibe-kanban-workflow
description: Orchestration workflow for vibe-kanban. Use when routing tasks, checking team composition, or making delegation decisions.
---

# vibe-kanban Orchestration Workflow

## Quick Reference

**Kanban Project ID:** [PROJECT_ID_TBD] ← Add after first task
**Main Role:** PURE ORCHESTRATOR (no direct code writing)
**Enforcement:** Hooks prevent direct code edits and tool misuse

---

## Task-First Workflow (MANDATORY)

### Small Tasks (<30 lines)
- Typos, one-liners, simple changes
- **Dispatch:** `Task(subagent_type='worker', prompt='...')`
- **No Kanban** needed

### Medium/Large Tasks (30+ lines)
1. Check Kanban: `mcp__vibe_kanban__list_tasks(project_id: "[PROJECT_ID]")`
2. Create task if new: `mcp__vibe_kanban__create_task(project_id, title, description)`
3. Start workspace: `mcp__vibe_kanban__start_workspace_session(task_id, executor='ORCHESTRATOR_MANAGED', repos=[...])`
4. Dispatch with task_id in prompt

---

## Complete Routing Table

| Work Category | Agent | Task? | Notes |
|---|---|---|---|
| **Backend - Rust** | Nova | Yes (30+) | Axum, async, crates |
| **Frontend - React** | Miley | Yes (30+) | Components, pages, styling |
| **Infra/CI-CD** | Emilia | Yes (30+) | GitHub Actions, Docker |
| **Quick Rust fix** | Bree | No | <30 lines only |
| **Quick React fix** | Bree | No | <30 lines only |
| **Code exploration** | Ivy | No | Direct, always |
| **Bug debugging** | Vera | No | Direct, always |
| **Feature design** | Ada | No | Direct, always |
| **Documentation** | Penny | No | Direct, always |

---

## Team Roster

### Non-Implementation (Direct Dispatch, Always Available)

| Agent | Name | When to Use |
|---|---|---|
| **Scout** | Ivy | Find files, understand architecture, explore |
| **Detective** | Vera | Investigate bugs, root cause analysis |
| **Architect** | Ada | Design features, plan implementations |
| **Scribe** | Penny | Write documentation, knowledge preservation |

### Implementation (Dispatch with Task, Kanban-based)

| Agent | Name | When to Use |
|---|---|---|
| **Worker** | Bree | <30 lines, quick fixes, no Kanban |
| **Rust Backend** | Nova | Axum APIs, crate development |
| **React Frontend** | Miley | React components, pages, UI |
| **DevOps/Infra** | Emilia | CI/CD, Docker, deployment |

---

## Step-by-Step Dispatch

### Small Task Example (No Kanban)

```
Bree can you fix the typo in README.md line 42?
Change "occured" to "occurred"

Task(
  subagent_type='worker',
  prompt='Fix typo in README.md line 42: change "occured" to "occurred"'
)
```

### Medium/Large Task Example (With Kanban)

```
Step 1: Check Kanban
mcp__vibe_kanban__list_tasks(project_id: "...")

Step 2: Create Task
mcp__vibe_kanban__create_task(
  project_id: "...",
  title: "Implement JWT authentication",
  description: "Add JWT token generation and validation..."
)
→ task_id = "abc123..."

Step 3: Start Workspace
mcp__vibe_kanban__start_workspace_session(
  task_id: "abc123...",
  executor: "ORCHESTRATOR_MANAGED",
  repos: [{repo_id: "...", base_branch: "main"}],
  agent_name: "Nova"
)
→ workspace_id, branch_name

Step 4: Dispatch
Task(
  subagent_type='worker',
  prompt='TASK: Implement JWT authentication

Task ID: abc123...
Workspace ID: ws456...
Branch: vk-ws456-jwt-auth

Implement:
1. JWT token generation in auth service
2. Token validation middleware
3. Login endpoint

When done:
- Commit changes
- Mark task "inreview": mcp__vibe_kanban__update_task(task_id, status="inreview")
- Report completion'
)
```

---

## Red Flags

❌ **NEVER DO:**
- Write code yourself
- Skip Kanban for medium/large work
- Forget task_id when dispatching supervisors
- Dispatch implementation agents without task tracking
- Create duplicate Kanban tasks
- Ignore orchestration hooks

✅ **ALWAYS DO:**
- Check Kanban first
- Create tasks for non-trivial work
- Include task_id in supervisor prompts
- Let agents implement directly
- Track progress in Kanban
- Respect hook validations

---

## Quick Reference Commands

### Check Kanban
```bash
mcp__vibe_kanban__list_tasks(project_id: "[PROJECT_ID]")
```

### Create Task
```bash
mcp__vibe_kanban__create_task(
  project_id: "[PROJECT_ID]",
  title: "Task title",
  description: "Detailed description"
)
```

### Update Task Status
```bash
mcp__vibe_kanban__update_task(task_id: "[TASK_ID]", status: "inreview")
```

### Add Task Comment
```bash
mcp__vibe_kanban__add_task_comment(
  task_id: "[TASK_ID]",
  author: "Nova",
  content: "Completed implementation of X feature"
)
```

### Start Workspace
```bash
mcp__vibe_kanban__start_workspace_session(
  task_id: "[TASK_ID]",
  executor: "ORCHESTRATOR_MANAGED",
  repos: [{repo_id: "[REPO_ID]", base_branch: "main"}],
  agent_name: "Nova"
)
```

---

## Full Workflows

**See `.claude/orchestration-workflows.md`** for:
- Detailed routing matrix
- Context preservation rules
- Parallel work protocol
- Error handling procedures
- Task lifecycle management
- PR workflow for remote repositories

---

## Red Flags Summary

**Task Validation Hooks** prevent:
- ❌ Medium/large tasks without Kanban
- ❌ Implementation agents without task_id
- ❌ Duplicate task creation

**Tool Blocking Hook** prevents:
- ❌ Direct code writing (Edit, Write, NotebookEdit)
- ❌ Git state changes (add, commit, push, merge)
- ❌ Task dispatch validation bypass

**Stop Hook** prevents:
- ❌ Abandoning incomplete tasks
- ❌ Leaving tasks in "inprogress" state

**SubagentStop Hook** prevents:
- ❌ Supervisor stopping without task completion
- ❌ Missing "inreview" status on completion
- ❌ Missing completion comments

These hooks enforce discipline and prevent workflow violations.

---

## Notes

- **Kanban Project ID:** Update `[PROJECT_ID]` once you create your first task and note the ID
- **Repository:** Update CLAUDE.md with actual GitHub repo details after first PR
- **MCP Tools:** All Vibe Kanban and GitHub tools available for orchestration
- **Hooks:** Active in `.claude/settings.json` - they enforce this workflow automatically

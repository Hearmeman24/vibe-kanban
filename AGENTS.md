# Repository Guidelines

## Project Structure & Module Organization
- `crates/`: Rust workspace crates — `server` (API + bins), `db` (SQLx models/migrations), `executors`, `services`, `utils`, `deployment`, `local-deployment`, `remote`.
- `frontend/`: React + TypeScript app (Vite, Tailwind). Source in `frontend/src`.
- `frontend/src/components/dialogs`: Dialog components for the frontend.
- `remote-frontend/`: Remote deployment frontend.
- `shared/`: Generated TypeScript types (`shared/types.ts`). Do not edit directly.
- `assets/`, `dev_assets_seed/`, `dev_assets/`: Packaged and local dev assets.
- `npx-cli/`: Files published to the npm CLI package.
- `scripts/`: Dev helpers (ports, DB preparation).
- `docs/`: Documentation files.

## Managing Shared Types Between Rust and TypeScript

ts-rs allows you to derive TypeScript types from Rust structs/enums. By annotating your Rust types with #[derive(TS)] and related macros, ts-rs will generate .ts declaration files for those types.
When making changes to the types, you can regenerate them using `pnpm run generate-types`
Do not manually edit shared/types.ts, instead edit crates/server/src/bin/generate_types.rs

## Build, Test, and Development Commands
- Install: `pnpm i`
- Run dev (frontend + backend with ports auto-assigned): `pnpm run dev`
- Backend (watch): `pnpm run backend:dev:watch`
- Frontend (dev): `pnpm run frontend:dev`
- Type checks: `pnpm run check` (frontend) and `pnpm run backend:check` (Rust cargo check)
- Rust tests: `cargo test --workspace`
- Generate TS types from Rust: `pnpm run generate-types` (or `generate-types:check` in CI)
- Prepare SQLx (offline): `pnpm run prepare-db`
- Prepare SQLx (remote package, postgres): `pnpm run remote:prepare-db`
- Local NPX build: `pnpm run build:npx` then `pnpm pack` in `npx-cli/`

## Coding Style & Naming Conventions
- Rust: `rustfmt` enforced (`rustfmt.toml`); group imports by crate; snake_case modules, PascalCase types.
- TypeScript/React: ESLint + Prettier (2 spaces, single quotes, 80 cols). PascalCase components, camelCase vars/functions, kebab-case file names where practical.
- Keep functions small, add `Debug`/`Serialize`/`Deserialize` where useful.

## Testing Guidelines
- Rust: prefer unit tests alongside code (`#[cfg(test)]`), run `cargo test --workspace`. Add tests for new logic and edge cases.
- Frontend: ensure `pnpm run check` and `pnpm run lint` pass. If adding runtime logic, include lightweight tests (e.g., Vitest) in the same directory.

## Security & Config Tips
- Use `.env` for local overrides; never commit secrets. Key envs: `FRONTEND_PORT`, `BACKEND_PORT`, `HOST`
- Dev ports and assets are managed by `scripts/setup-dev-environment.js`.

---

## Mandatory Workflow (Multi-Agent Orchestration)

**YOU ARE THE ORCHESTRATOR. YOU NEVER WRITE CODE.**

### Strict Prohibition

**YOU ARE STRICTLY PROHIBITED FROM PERFORMING ANY ACTION.**

Every action, even the smallest, must be delegated. Follow-ups go BACK to the same agent.

### The Team

| Agent | Name | Role | When to Use |
|-------|------|------|-------------|
| Scout | **Ivy** | Exploration | Find files, understand structure |
| Detective | **Vera** | Debugging | Investigate bugs, trace issues |
| Architect | **Ada** | Planning | Design approaches, break down tasks |
| Scribe | **Penny** | Documentation | Write/update docs |
| Worker | **Bree** | Small fixes | Changes < 30 lines, single file |
| Rust Supervisor | **Ferris** | Backend | MCP tools, models, migrations, Rust code |
| Frontend Supervisor | **Miley** | UI | React components, TypeScript, styling |
| Infra Supervisor | **Emilia** | DevOps | Docker, CI/CD, git push/PR/merge |

### Session Start (Every New Conversation)

1. **Check Kanban for in-progress tasks:**
   ```
   mcp__vibe_kanban__list_tasks(project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f", status: "inprogress")
   ```
2. **If tasks exist:** "Task X is in-progress. Resume or start fresh?"
3. **If resuming:** Load context from task description, dispatch to original supervisor

### Task Granularity (Architect-First + Size Check)

**Before dispatching to a supervisor, evaluate task size:**

| Trigger | Action |
|---------|--------|
| Request mentions 3+ components | Ada plans first → Break down |
| Request spans multiple features | Ada plans first → Break down |
| Single feature, 1-2 files | OK to dispatch directly |

**Architect-First Flow:**
```
Large request → Dispatch to Ada → Ada returns task breakdown →
Create Kanban task per deliverable → Dispatch supervisors sequentially
```

**One Kanban Task = One Deliverable**
Each task should be independently testable and reviewable.

### Task Size Classification

| Size | Criteria | Kanban Task? | Delegate To |
|------|----------|--------------|-------------|
| **Small** | Single file, <30 lines | No | Bree (worker) |
| **Medium** | 2-5 files, new component | Yes | Supervisor |
| **Large** | 6+ files, new feature | Yes | Supervisor |

### Routing Table

| What | Who |
|------|-----|
| Small fix (<30 lines) | Bree (worker) |
| Rust/MCP tools/Backend | Ferris (rust-supervisor) |
| Frontend/React/TypeScript | Miley (frontend-supervisor) |
| Docker/CI/git push/PR | Emilia (infra-supervisor) |
| Explore codebase | Ivy (scout) |
| Debug/investigate | Vera (detective) |
| Plan/design | Ada (architect) |
| Documentation | Penny (scribe) |
| Web research | researcher |

### Context Preservation (Follow-Up Rule)

**When user reports issue with recent work:**
1. Identify which agent implemented the feature
2. Resume or re-dispatch SAME agent: "User reported: [issue]. Debug and fix."
3. Agent debugs + fixes (they have full context)

**Use `resume` for complex multi-round fixes:**
```
Task(resume="<agent_id>", prompt="User found issue: [description]")
```

**Only use Vera (Detective) when:**
- Bug is in code NO agent touched this session
- Issue spans multiple supervisors' domains
- Implementing supervisor is stuck (escalation)

### Dispatching Supervisors (Medium/Large)

**Orchestrator creates task:**
```
mcp__vibe_kanban__create_task(project_id: "7d8d2452-d215-469f-8bf8-9be9606a107f", title: "...")
```

**Orchestrator dispatches with task_id (background by default):**
```
Task(subagent_type="rust-supervisor", prompt="Task ID: <task_id>\n\n<description>", run_in_background=true)
```

Check results when ready:
```
TaskOutput(task_id="<agent_id>")
```

**Supervisor manages status during work:**
- Start work: `update_task(task_id, status="inprogress")`
- Implementation complete: `update_task(task_id, status="inreview")`
- Bug fix cycle: `inreview` → `inprogress` → `inreview`

**Orchestrator marks done (after user approval):**
```
mcp__vibe_kanban__update_task(task_id: "<task_id>", status="done")
```

**Key Rules:**
- Supervisors NEVER mark `done` - only orchestrator after user approval
- Bug reported? Move back to `inprogress`, re-dispatch same supervisor
- User approves? Orchestrator marks `done`

### Parallel Work Protocol

**When request spans multiple domains (e.g., Rust + frontend):**

1. **Identify domains:** Which supervisors need to be involved?
2. **Create linked tasks:** One Kanban task per domain
3. **Dispatch in parallel:**
   ```
   Task(subagent_type="rust-supervisor", prompt="...", run_in_background=true)
   Task(subagent_type="frontend-supervisor", prompt="...", run_in_background=true)
   ```
4. **Wait for all:** `TaskOutput(task_id)` for each
5. **Review together:** All related tasks move to `inreview` together

### Background Execution

```
Task(subagent_type="...", prompt="...", run_in_background=true)
TaskOutput(task_id="<agent_id>")  # Get results when ready
```

**Kanban Project:** `7d8d2452-d215-469f-8bf8-9be9606a107f`

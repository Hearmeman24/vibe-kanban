#!/bin/bash
cat << 'EOF'
╔══════════════════════════════════════════════════════════════════╗
║                    ORCHESTRATOR REMINDER                          ║
╠══════════════════════════════════════════════════════════════════╣
║  YOU ARE STRICTLY PROHIBITED FROM PERFORMING ANY ACTION.         ║
║     Every action, even the smallest, must be delegated.          ║
║     Follow-ups go BACK to the same agent.                        ║
║                                                                   ║
║  TASK-FIRST WORKFLOW:                                             ║
║  ┌────────────────────────────────────────────────────────────┐  ║
║  │ Small (<30 lines)  → Dispatch Bree (worker) directly       │  ║
║  │ Medium/Large       → CREATE KANBAN TASK FIRST, then send   │  ║
║  │                      task_id to supervisor                 │  ║
║  └────────────────────────────────────────────────────────────┘  ║
║                                                                   ║
║  GRANULARITY: 3+ components? → Ada (architect) first!            ║
║                                                                   ║
║  CONTEXT PRESERVATION:                                            ║
║  ┌────────────────────────────────────────────────────────────┐  ║
║  │ Bug in RECENT work  → SAME agent (resume or re-dispatch)   │  ║
║  │ Bug in OLD code     → Vera (detective)                     │  ║
║  │ Cross-domain bug    → Vera first, then supervisors         │  ║
║  └────────────────────────────────────────────────────────────┘  ║
║                                                                   ║
║  ROUTING:                                                         ║
║  ┌────────────────────────────────────────────────────────────┐  ║
║  │ What                      → Who                            │  ║
║  ├────────────────────────────────────────────────────────────┤  ║
║  │ Small fix (<30 lines)     → Bree (worker)                  │  ║
║  │ Rust/MCP/Backend          → Ferris (rust-supervisor)       │  ║
║  │ Frontend/React/TS         → Miley (frontend-supervisor)    │  ║
║  │ Docker/CI/git push/PR     → Emilia (infra-supervisor)      │  ║
║  │ Explore codebase          → Ivy (scout)                    │  ║
║  │ Debug/investigate         → Vera (detective)               │  ║
║  │ Plan/design               → Ada (architect)                │  ║
║  │ Documentation             → Penny (scribe)                 │  ║
║  │ Web research              → researcher                     │  ║
║  └────────────────────────────────────────────────────────────┘  ║
║                                                                   ║
║  BACKGROUND: run_in_background=true, then TaskOutput(task_id)    ║
║  KANBAN: 7d8d2452-d215-469f-8bf8-9be9606a107f                    ║
╚══════════════════════════════════════════════════════════════════╝
EOF
exit 0

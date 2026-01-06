#!/bin/bash
# Orchestrator Reminder Hook
# Displayed on UserPromptSubmit to remind you of delegation workflow

cat << 'EOF'
⛔ ORCHESTRATOR MODE - Delegation Required

WORKFLOW:
  Small (<30 lines)    → Bree (Worker)
  Medium/Large (30+)   → Create Kanban task → Dispatch Supervisor

QUICK ROUTING:
  Backend (Rust)  → Nova (Rust Engineer)
  Frontend (React) → Miley (Frontend Supervisor)
  Infra (CI/CD)   → Emilia (Infra Supervisor)
  Debugging       → Vera (Detective)
  Exploring       → Ivy (Scout)
  Planning        → Ada (Architect)
  Docs            → Penny (Scribe)

FULL ROUTING: .claude/orchestration-workflows.md

Kanban: [PROJECT_ID_TBD]
Workflow Skill: /vibe-kanban-workflow
EOF
exit 0

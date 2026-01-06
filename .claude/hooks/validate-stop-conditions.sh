#!/bin/bash
#
# Stop Hook: Task Abandonment Prevention
#
# Prevents orchestrator from stopping session with incomplete tasks.
# Checks Vibe Kanban for tasks in "inprogress" state.
#

PROJECT_ID="[PROJECT_ID_TBD]"

# If PROJECT_ID not set, skip validation
if [[ "$PROJECT_ID" == "[PROJECT_ID_TBD]" ]]; then
  exit 0
fi

# Try to query Kanban for in-progress tasks
RESPONSE=$(curl -s "http://localhost:3000/api/projects/$PROJECT_ID/tasks?status=inprogress" 2>/dev/null)

if [[ $? -ne 0 ]]; then
  # Kanban not reachable - warn but don't block
  cat << 'EOF'
⚠️  WARNING: Could not connect to Vibe Kanban backend
   Unable to validate task status before stopping.
   Proceeding anyway...
EOF
  exit 0
fi

# Count in-progress tasks (simple JSON parsing)
INPROGRESS_COUNT=$(echo "$RESPONSE" | jq -r '.count // 0' 2>/dev/null || echo "0")

if [[ "$INPROGRESS_COUNT" -gt 0 ]]; then
  cat << EOF
╔═══════════════════════════════════════════════════════════════╗
║  ⛔ STOP BLOCKED: Incomplete tasks in progress                ║
╚═══════════════════════════════════════════════════════════════╝

You have $INPROGRESS_COUNT task(s) in progress.

REQUIRED ACTIONS (choose one):
1. Mark task complete: mcp__vibe_kanban__update_task(task_id, status="inreview")
2. Mark task cancelled: mcp__vibe_kanban__update_task(task_id, status="cancelled")
3. Add progress comment: mcp__vibe_kanban__add_task_comment(task_id, content="...")

THEN you can stop safely.
EOF
  exit 1
fi

# All clear
exit 0

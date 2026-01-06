#!/bin/bash
#
# SubagentStop Hook: Supervisor Quality Gate
#
# Validates supervisor completed workflow by checking Kanban state.
# Handles both task-based and taskless (small fixes) workflows.
#

PROJECT_ID="[PROJECT_ID_TBD]"

INPUT=$(cat)

# If PROJECT_ID not set, skip validation
if [[ "$PROJECT_ID" == "[PROJECT_ID_TBD]" ]]; then
  exit 0
fi

# Extract task_id from output (UUID format)
TASK_ID=$(echo "$INPUT" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)

# === CASE 1: No task_id found ===
if [[ -z "$TASK_ID" ]]; then
  # Check if supervisor mentioned "small task" or "no task"
  if echo "$INPUT" | grep -qiE "small (task|fix)|no (task|kanban)|direct dispatch|quick fix"; then
    exit 0
  fi

  # No task_id and no indication - warn but allow
  cat << 'EOF'
⚠️  WARNING: No task_id found in supervisor output

If this is a SMALL task (<30 lines, no Kanban): ✅ OK
If this is a MEDIUM/LARGE task: ❌ You should have task_id

Assuming small task. Allowing stop.
EOF
  exit 0
fi

# === CASE 2: task_id found - Validate Kanban state ===

RESPONSE=$(curl -s "http://localhost:3000/api/tasks/$TASK_ID" 2>/dev/null)

if [[ $? -ne 0 ]]; then
  cat << EOF
⚠️  WARNING: Could not connect to Vibe Kanban backend
   Task ID: $TASK_ID
   Cannot validate. Proceeding anyway.
EOF
  exit 0
fi

# Extract task status
TASK_STATUS=$(echo "$RESPONSE" | jq -r '.task.status // .status // empty')

if [[ -z "$TASK_STATUS" ]]; then
  cat << EOF
⚠️  WARNING: Task $TASK_ID not found. Cannot validate.
EOF
  exit 0
fi

# Validate task status is "inreview"
if [[ "$TASK_STATUS" != "inreview" ]]; then
  cat << EOF
╔═══════════════════════════════════════════════════════════════╗
║  ⛔ STOP BLOCKED: Task status is '$TASK_STATUS'               ║
╚═══════════════════════════════════════════════════════════════╝

Task ID: $TASK_ID
Current: $TASK_STATUS | Required: inreview

You must mark as inreview before stopping:
mcp__vibe_kanban__update_task(task_id: "$TASK_ID", status: "inreview")

BEFORE marking inreview:
1. Ensure all changes committed
2. Add completion comment with summary
3. Verify tests passing
EOF
  exit 1
fi

# Validate comments exist
COMMENT_RESPONSE=$(curl -s "http://localhost:3000/api/tasks/$TASK_ID/comments" 2>/dev/null)
COMMENT_COUNT=$(echo "$COMMENT_RESPONSE" | jq -r '.count // 0')

if [[ "$COMMENT_COUNT" -lt 1 ]]; then
  cat << EOF
╔═══════════════════════════════════════════════════════════════╗
║  ⛔ STOP BLOCKED: No completion comment found                 ║
╚═══════════════════════════════════════════════════════════════╝

Task ID: $TASK_ID
Status: inreview ✓ | Comments: 0 ✗

Add completion comment:
mcp__vibe_kanban__add_task_comment(
  task_id: "$TASK_ID",
  author: "[AGENT_NAME]",
  content: "Completed: [summary]. Files: [list]. Tests: [status]."
)
EOF
  exit 1
fi

# All critical checks passed
echo "✅ Workflow validated: status=inreview, comments=$COMMENT_COUNT"
exit 0

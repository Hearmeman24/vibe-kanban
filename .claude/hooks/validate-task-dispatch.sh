#!/bin/bash
#
# PreToolUse Hook: Task Dispatch Validation
#
# Ensures orchestrator creates Kanban task for medium/large work before dispatching.
# Allows small tasks (<30 lines) to be dispatched directly.
#

DEBUG_LOG="$HOME/.claude/logs/hook-debug.log"
mkdir -p "$HOME/.claude/logs" 2>/dev/null

INPUT=$(cat)

echo "[$(date)] Task dispatch validation triggered" >> "$DEBUG_LOG" 2>/dev/null

PROMPT=$(echo "$INPUT" | jq -r '.tool_input.prompt // empty')

if [[ -z "$PROMPT" ]] || [[ "$PROMPT" == "null" ]] || [[ "$PROMPT" == "empty" ]]; then
  exit 0
fi

echo "[$(date)] Task prompt length: ${#PROMPT} chars" >> "$DEBUG_LOG" 2>/dev/null

SUBAGENT_TYPE=$(echo "$INPUT" | jq -r '.tool_input.subagent_type // empty')
echo "[$(date)] Subagent type: $SUBAGENT_TYPE" >> "$DEBUG_LOG" 2>/dev/null

# Non-implementation agents (exploration, investigation, planning) - ALWAYS allow
if [[ "$SUBAGENT_TYPE" =~ ^(scout|Explore|detective|architect|scribe)$ ]]; then
  echo "[$(date)] ALLOWED: Non-implementation agent ($SUBAGENT_TYPE)" >> "$DEBUG_LOG" 2>/dev/null
  exit 0
fi

# Check if prompt contains task_id (UUID format)
TASK_ID=$(echo "$PROMPT" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)

# If task_id found - proper workflow, allow
if [[ -n "$TASK_ID" ]]; then
  echo "[$(date)] ALLOWED: Task dispatch with task_id=$TASK_ID" >> "$DEBUG_LOG" 2>/dev/null
  exit 0
fi

echo "[$(date)] No task_id found. Checking if small task..." >> "$DEBUG_LOG" 2>/dev/null

# Check if explicitly marked as small task
if echo "$PROMPT" | grep -qiE "small (task|fix)|quick fix|typo|single file|<30 lines|one-line|simple change"; then
  echo "[$(date)] ALLOWED: Small task (keyword detected)" >> "$DEBUG_LOG" 2>/dev/null
  exit 0
fi

# Estimate complexity heuristically
PROMPT_LENGTH=${#PROMPT}
MULTI_FILE=$(echo "$PROMPT" | grep -ciE "files:|multiple files|several files|src/.*and.*src/")
IMPLEMENT=$(echo "$PROMPT" | grep -ciE "implement|create.*endpoint|add.*feature|build.*component")

echo "[$(date)] Heuristics: length=$PROMPT_LENGTH, multifile=$MULTI_FILE, implement=$IMPLEMENT" >> "$DEBUG_LOG" 2>/dev/null

# If looks complex - likely medium/large task
if [[ "$PROMPT_LENGTH" -gt 500 ]] || [[ "$MULTI_FILE" -gt 0 ]] || [[ "$IMPLEMENT" -gt 0 ]]; then
  echo "[$(date)] BLOCKED: Medium/large task without task_id" >> "$DEBUG_LOG" 2>/dev/null
  cat << EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "This appears to be a MEDIUM/LARGE task (complex prompt, implementation work) but no task_id found. You must create a Kanban task first:\n\n1. mcp__vibe_kanban__create_task(project_id, title, description)\n2. mcp__vibe_kanban__start_workspace_session(task_id, executor='ORCHESTRATOR_MANAGED', repos=[...])\n3. Then dispatch: Task(prompt='Task ID: {task_id}\\n...')\n\nFor SMALL tasks (<30 lines, quick fixes), add 'small task' to your prompt to bypass this check."
  }
}
EOF
  exit 0
fi

# Looks like small task - allow
echo "[$(date)] ALLOWED: Small task (simple prompt)" >> "$DEBUG_LOG" 2>/dev/null
exit 0

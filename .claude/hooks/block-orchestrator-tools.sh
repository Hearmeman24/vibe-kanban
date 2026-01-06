#!/bin/bash
#
# PreToolUse Hook: Allowlist-Based Tool Blocking (OWASP Best Practice)
#
# DENY-BY-DEFAULT: Only allows orchestration tools, blocks everything else.
# Based on OWASP Authorization principles and Kubernetes RBAC least-privilege.
#

DEBUG_LOG="$HOME/.claude/logs/hook-debug.log"
mkdir -p "$HOME/.claude/logs" 2>/dev/null

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

if [[ -z "$TOOL_NAME" ]] || [[ "$TOOL_NAME" == "null" ]]; then
  exit 0
fi

echo "[$(date)] Checking tool: $TOOL_NAME" >> "$DEBUG_LOG" 2>/dev/null

# ALLOWLIST: Tools permitted for orchestrator
ALLOWED_CORE="Task|Bash|Glob|Read|AskUserQuestion"
ALLOWED_OPTIONAL="WebFetch|Grep|TodoWrite"
ALLOWED_MCP="mcp__vibe_kanban__.*|mcp__github__search_.*|mcp__github__list_.*|mcp__github__get_.*|mcp__github__issue_read|mcp__github__pull_request_read|mcp__github__create_pull_request|mcp__github__create_branch|mcp__github__add_issue_comment"

ALLOWLIST="$ALLOWED_CORE|$ALLOWED_OPTIONAL|$ALLOWED_MCP"

# Check if tool is allowed
if [[ ! "$TOOL_NAME" =~ ^($ALLOWLIST)$ ]]; then
  cat << EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "Tool '$TOOL_NAME' not in orchestrator allowlist. Orchestrators delegate work via Task() tool - they don't implement directly. Use Task(subagent_type='<agent>', prompt='...') to delegate. See .claude/orchestration-workflows.md for routing."
  }
}
EOF
  exit 0
fi

# Validate Bash commands
if [[ "$TOOL_NAME" == "Bash" ]]; then
  COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

  if [[ -z "$COMMAND" ]] || [[ "$COMMAND" == "null" ]]; then
    exit 0
  fi

  # Block git state-modifying commands
  if [[ "$COMMAND" =~ ^git\ (add|commit|push|merge|rebase|reset) ]]; then
    echo "[$(date)] BLOCKED: Git command '$COMMAND'" >> "$DEBUG_LOG" 2>/dev/null
    cat << EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "Git command '$COMMAND' is blocked. Orchestrators don't run git operations directly. Supervisors commit code. Orchestrators handle push/PR via GitHub MCP. See .claude/orchestration-workflows.md"
  }
}
EOF
    exit 0
  fi
fi

echo "[$(date)] ALLOWED: $TOOL_NAME" >> "$DEBUG_LOG" 2>/dev/null
exit 0

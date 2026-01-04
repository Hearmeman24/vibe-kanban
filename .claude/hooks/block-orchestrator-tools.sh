#!/bin/bash
#
# PreToolUse Hook: Block Orchestrator Direct Actions
#
# Prevents the orchestrator from using Edit, Write, or certain Bash commands.
# Forces delegation to appropriate agents.
#

TOOL_NAME="$1"

# Block direct file edits - must delegate to supervisors
if [[ "$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write" ]]; then
  cat << 'EOF'
╔═══════════════════════════════════════════════════════════════╗
║  BLOCKED: Orchestrator cannot use Edit/Write directly.        ║
║                                                               ║
║  DELEGATE TO:                                                 ║
║  • Rust (.rs)         → rust-supervisor (Ferris)              ║
║  • Frontend (.tsx/ts) → frontend-supervisor (Miley)           ║
║  • Small fix          → worker (Bree)                         ║
║  • Documentation      → scribe (Penny)                        ║
╚═══════════════════════════════════════════════════════════════╝
EOF
  exit 1
fi

# Block direct git commands - must delegate
if [[ "$TOOL_NAME" == "Bash" ]]; then
  # Read command from stdin (Claude Code passes tool input via stdin)
  INPUT=$(cat)
  COMMAND=$(echo "$INPUT" | grep -o '"command":"[^"]*"' | cut -d'"' -f4)

  if [[ "$COMMAND" == git\ add* || "$COMMAND" == git\ commit* ]]; then
    cat << 'EOF'
╔═══════════════════════════════════════════════════════════════╗
║  BLOCKED: Orchestrator cannot run git add/commit.             ║
║                                                               ║
║  DELEGATE TO: The supervisor who made the changes.            ║
║  They have context of what they changed.                      ║
╚═══════════════════════════════════════════════════════════════╝
EOF
    exit 1
  fi

  if [[ "$COMMAND" == git\ push* || "$COMMAND" == git\ merge* ]]; then
    cat << 'EOF'
╔═══════════════════════════════════════════════════════════════╗
║  BLOCKED: Orchestrator cannot run git push/merge.             ║
║                                                               ║
║  DELEGATE TO: infra-supervisor (Emilia)                       ║
║  Emilia handles all cross-cutting git operations.             ║
╚═══════════════════════════════════════════════════════════════╝
EOF
    exit 1
  fi
fi

exit 0

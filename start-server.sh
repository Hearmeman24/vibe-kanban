#!/bin/bash
# Quick start - runs existing binary without rebuilding

PROJECT_DIR="/Users/avivkaplan/src/vibe-kanban"
BINARY="$PROJECT_DIR/target/release/server"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Vibe Kanban: Binary not found. Run start-prod.sh first to build."
    exit 1
fi

cd "$PROJECT_DIR"
PORT=3000 "$BINARY"

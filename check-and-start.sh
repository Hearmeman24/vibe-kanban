#!/bin/bash

PROJECT_DIR="/Users/avivkaplan/src/vibe-kanban"
BINARY="$PROJECT_DIR/target/release/server"
LOG_FILE="$PROJECT_DIR/server.log"

# Check if Vibe Kanban is already running
if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo "Vibe Kanban: Already running on localhost:3000"
    exit 0
fi

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Vibe Kanban: Binary not found at $BINARY"
    echo "Vibe Kanban: Run 'bash $PROJECT_DIR/start-prod.sh' to build first"
    exit 0
fi

# Start server in background
echo "Vibe Kanban: Starting server on port 3000..."
cd "$PROJECT_DIR"
nohup env PORT=3000 "$BINARY" > "$LOG_FILE" 2>&1 &

# Wait briefly for server to start
sleep 2

# Verify it started
if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo "Vibe Kanban: Server started successfully"
else
    echo "Vibe Kanban: Server starting... (check $LOG_FILE)"
fi
exit 0

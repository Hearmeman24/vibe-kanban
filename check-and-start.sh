#!/bin/bash

# Check if Vibe Kanban is already running
if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo "Vibe Kanban: Already running on localhost:3000"
    exit 0
fi

# Not running, start it in the background
echo "Vibe Kanban: Not running, starting server..."
cd /Users/avivkaplan/src/vibe-kanban
nohup ./start-prod.sh > /Users/avivkaplan/src/vibe-kanban/server.log 2>&1 &
echo "Vibe Kanban: Server started in background (log: /Users/avivkaplan/src/vibe-kanban/server.log)"
exit 0

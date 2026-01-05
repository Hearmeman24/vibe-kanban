#!/bin/bash
set -e

PROJECT_DIR="/Users/avivkaplan/src/vibe-kanban"
cd "$PROJECT_DIR"

echo "=== Building Frontend ==="
cd frontend
pnpm install
pnpm build
cd "$PROJECT_DIR"

echo "=== Building Rust Server (Release) ==="
cargo build --release

echo "=== Starting Server ==="
./target/release/server

#!/bin/bash
set -e

echo "=== Building Frontend ==="
cd frontend
pnpm install
pnpm build
cd ..

echo "=== Building Rust Server (Release) ==="
cargo build --release

echo "=== Starting Server ==="
./target/release/server

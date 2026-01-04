# syntax=docker/dockerfile:1

# ==============================================================================
# Stage 1: Chef base - Install cargo-chef for dependency caching
# ==============================================================================
FROM rust:alpine AS chef

RUN apk add --no-cache \
    build-base \
    perl \
    llvm-dev \
    clang-dev \
    musl-dev

# Allow linking libclang on musl
ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN cargo install cargo-chef
WORKDIR /app

# ==============================================================================
# Stage 2: Planner - Analyze dependencies and create recipe
# ==============================================================================
FROM chef AS planner

# Copy only what's needed for dependency analysis
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Generate recipe.json (dependency manifest)
RUN cargo chef prepare --recipe-path recipe.json

# ==============================================================================
# Stage 3: Rust Types Builder - Build generate_types binary first
# ==============================================================================
FROM chef AS rust-types-builder

# Copy recipe and cook dependencies (this layer is cached)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Now copy the actual source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# Assets directory (scripts, sounds - not frontend)
COPY assets ./assets

# Build only generate_types binary first
RUN cargo build --release --bin generate_types

# ==============================================================================
# Stage 4: Node Builder - Build frontend with generated types
# ==============================================================================
FROM node:24-alpine AS node-builder

WORKDIR /app

ARG POSTHOG_API_KEY
ARG POSTHOG_API_ENDPOINT

ENV VITE_PUBLIC_POSTHOG_KEY=$POSTHOG_API_KEY
ENV VITE_PUBLIC_POSTHOG_HOST=$POSTHOG_API_ENDPOINT

# Copy package files for dependency caching
COPY package*.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY frontend/package*.json ./frontend/
COPY npx-cli/package*.json ./npx-cli/

# Install pnpm and dependencies
RUN npm install -g pnpm && pnpm install

# Copy generate_types binary from rust types builder
COPY --from=rust-types-builder /app/target/release/generate_types /usr/local/bin/generate_types

# Copy source code needed for type generation and frontend build
COPY shared ./shared
COPY frontend ./frontend

# Generate TypeScript types
RUN generate_types

# Build frontend
RUN cd frontend && pnpm run build

# ==============================================================================
# Stage 5: Rust Server Builder - Build server with frontend embedded
# ==============================================================================
FROM rust-types-builder AS rust-builder

# Copy the built frontend from node-builder BEFORE compiling server
# This is critical: rust-embed embeds frontend/dist at compile time
COPY --from=node-builder /app/frontend/dist ./frontend/dist

# Build server binary (now with frontend assets available for embedding)
RUN cargo build --release --bin server

# ==============================================================================
# Stage 6: Runtime - Minimal production image
# ==============================================================================
FROM alpine:latest AS runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    tini \
    libgcc \
    wget

# Create app user for security
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

# Copy server binary from rust builder
COPY --from=rust-builder /app/target/release/server /usr/local/bin/server

# Create repos directory and set permissions
RUN mkdir -p /repos && \
    chown -R appuser:appgroup /repos

# Switch to non-root user
USER appuser

# Set runtime environment
ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

# Set working directory
WORKDIR /repos

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider "http://${HOST:-localhost}:${PORT:-3000}" || exit 1

# Run the application
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["server"]

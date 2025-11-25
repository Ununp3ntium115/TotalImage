# TotalImage - Multi-stage Rust build
# Forensic disk image analysis platform

# ============================================
# Stage 1: Builder
# ============================================
FROM rust:1.75-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/totalimage-core/Cargo.toml crates/totalimage-core/
COPY crates/totalimage-pipeline/Cargo.toml crates/totalimage-pipeline/
COPY crates/totalimage-vaults/Cargo.toml crates/totalimage-vaults/
COPY crates/totalimage-zones/Cargo.toml crates/totalimage-zones/
COPY crates/totalimage-territories/Cargo.toml crates/totalimage-territories/
COPY crates/totalimage-cli/Cargo.toml crates/totalimage-cli/
COPY crates/totalimage-web/Cargo.toml crates/totalimage-web/
COPY crates/totalimage-mcp/Cargo.toml crates/totalimage-mcp/
COPY crates/totalimage-acquire/Cargo.toml crates/totalimage-acquire/
COPY crates/fire-marshal/Cargo.toml crates/fire-marshal/

# Create dummy source files for dependency compilation
RUN mkdir -p crates/totalimage-core/src && echo "pub fn dummy() {}" > crates/totalimage-core/src/lib.rs && \
    mkdir -p crates/totalimage-pipeline/src && echo "pub fn dummy() {}" > crates/totalimage-pipeline/src/lib.rs && \
    mkdir -p crates/totalimage-vaults/src && echo "pub fn dummy() {}" > crates/totalimage-vaults/src/lib.rs && \
    mkdir -p crates/totalimage-zones/src && echo "pub fn dummy() {}" > crates/totalimage-zones/src/lib.rs && \
    mkdir -p crates/totalimage-territories/src && echo "pub fn dummy() {}" > crates/totalimage-territories/src/lib.rs && \
    mkdir -p crates/totalimage-cli/src && echo "fn main() {}" > crates/totalimage-cli/src/main.rs && \
    mkdir -p crates/totalimage-web/src && echo "fn main() {}" > crates/totalimage-web/src/main.rs && \
    mkdir -p crates/totalimage-mcp/src && echo "fn main() {}" > crates/totalimage-mcp/src/main.rs && \
    mkdir -p crates/totalimage-acquire/src && echo "pub fn dummy() {}" > crates/totalimage-acquire/src/lib.rs && \
    mkdir -p crates/fire-marshal/src && echo "fn main() {}" > crates/fire-marshal/src/main.rs

# Build dependencies only (cached layer)
RUN cargo build --release || true

# Copy actual source code
COPY crates/ crates/

# Touch files to force rebuild with actual sources
RUN touch crates/*/src/*.rs

# Build release binaries
RUN cargo build --release --workspace

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash totalimage

# Copy binaries from builder
COPY --from=builder /build/target/release/totalimage /usr/local/bin/
COPY --from=builder /build/target/release/totalimage-web /usr/local/bin/
COPY --from=builder /build/target/release/totalimage-mcp /usr/local/bin/
COPY --from=builder /build/target/release/fire-marshal /usr/local/bin/

# Create directories for images and cache
RUN mkdir -p /data/images /data/cache && \
    chown -R totalimage:totalimage /data

# Set working directory
WORKDIR /data

# Switch to non-root user
USER totalimage

# Default environment variables
ENV RUST_LOG=info
ENV TOTALIMAGE_CACHE_DIR=/data/cache

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Default command (web server)
CMD ["totalimage-web"]

# Expose ports
# 3000 - Web API
# 3001 - Fire Marshal
# 3002 - MCP Server (integrated mode)
EXPOSE 3000 3001 3002

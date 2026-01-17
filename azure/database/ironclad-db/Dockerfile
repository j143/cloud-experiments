# Multi-stage Dockerfile for IronClad-DB
# Stage 1: Builder
FROM rust:1.92-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml ./
# Copy source tree
COPY src ./src
COPY tests ./tests

# Build for release
RUN cargo build --release --locked

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ironclad .

# Set labels
LABEL org.opencontainers.image.title="IronClad-DB"
LABEL org.opencontainers.image.description="Persistent, crash-safe KV Store on Azure Page Blobs"
LABEL org.opencontainers.image.version="1.0"

# Expose port for monitoring
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["curl", "-f", "http://localhost:8080/health", "||" "exit", "1"]

# Run the application
ENTRYPOINT ["./ironclad"]

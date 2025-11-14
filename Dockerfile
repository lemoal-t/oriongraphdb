FROM rust:1.75-slim as builder

WORKDIR /build

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src/ ./src/

# Build release binary
RUN cargo build --release --bin axongraph-server

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/axongraph-server /app/axongraph-server

# Expose port
EXPOSE 8081

# Health check
HEALTHCHECK --interval=5s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8081/health || exit 1

# Run AxonGraph server with real indices
CMD ["/app/axongraph-server", "--use-real"]

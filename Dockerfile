# TrimX CLI Video Clipper - Dockerfile
# Multi-stage build for development and production

# Build stage
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libavformat-dev \
    libavcodec-dev \
    libavutil-dev \
    libswscale-dev \
    libswresample-dev \
    libavfilter-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src/ ./src/
COPY tests/ ./tests/
COPY config/ ./config/

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libavformat59 \
    libavcodec59 \
    libavutil57 \
    libswscale6 \
    libswresample4 \
    libavfilter8 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false trimx

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/trimx_cli_based_video_clipper /usr/local/bin/trimx

# Copy configuration files
COPY --from=builder /app/config/ ./config/

# Create necessary directories
RUN mkdir -p logs temp cache && \
    chown -R trimx:trimx /app

# Switch to app user
USER trimx

# Set environment variables
ENV RUST_LOG=info
ENV TRIMX_CONFIG_DIR=/app/config

# Expose port for health checks (if needed)
EXPOSE 8080

# Default command
CMD ["trimx", "--help"]

# Development stage
FROM builder as development

# Install development tools
RUN cargo install cargo-watch cargo-expand

# Set working directory
WORKDIR /app

# Copy all source files
COPY . .

# Set environment variables for development
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1

# Default command for development
CMD ["cargo", "test"]

# Test stage
FROM builder as test

# Install test dependencies
RUN cargo install cargo-tarpaulin

# Set working directory
WORKDIR /app

# Copy all source files
COPY . .

# Run tests
RUN cargo test --all-features

# Coverage stage
FROM test as coverage

# Generate coverage report
RUN cargo tarpaulin --out Html --output-dir coverage

# Copy coverage report
COPY --from=coverage /app/coverage/ ./coverage/

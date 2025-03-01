# Build stage
FROM rust:1.72 as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libpq-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml ./

# Create dummy src directory and file
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy src files and target directory
RUN rm -rf src target

# Copy the actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    curl \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/borrower-cli-tester /app/borrower-cli-tester

# Create a data directory for the SQLite database
RUN mkdir -p /app/data && \
    chmod 777 /app/data

# Set environment variables
ENV DATABASE_PATH=/app/data/test_results.db
ENV HOST=0.0.0.0
ENV PORT=8080

# Expose the port
EXPOSE 8080

# Start the application
CMD ["/app/borrower-cli-tester"]
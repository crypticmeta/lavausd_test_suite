# Build stage
FROM rust:latest as builder

# Install dependencies
RUN apt-get update && apt-get install -y libpq-dev

# Create a new empty project
WORKDIR /app

# Copy your source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y libpq5 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/borrower-cli-tester /app/

# Create data directory
RUN mkdir -p /app/data

# Copy the CLI file if it exists, or create a placeholder
COPY --from=builder /app/loans-borrower-cli /app/

# Set executable permissions for the main binary
RUN chmod +x /app/borrower-cli-tester

# Try to set executable permissions for the CLI (will be ignored if file doesn't exist)
RUN if [ -f /app/loans-borrower-cli ]; then chmod +x /app/loans-borrower-cli; else echo "CLI file not found, will be downloaded at runtime"; fi

# Expose the port
EXPOSE 8080

# Run the application
CMD ["/app/borrower-cli-tester"]
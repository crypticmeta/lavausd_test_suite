FROM rust:1.74-slim as builder

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libpq-dev && \
    rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:12-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl-dev libpq-dev ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/borrower-cli-tester .

# Create directories
RUN mkdir -p data

# Download CLI on image build (optional, the server will handle it if not present)
RUN curl -o loans-borrower-cli https://loans-borrower-cli.s3.amazonaws.com/loans-borrower-cli-linux && \
    chmod +x loans-borrower-cli

# Default environment variables
ENV DATABASE_PATH=/app/data/test_results.db
ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

# Run the application
CMD ["./borrower-cli-tester"]
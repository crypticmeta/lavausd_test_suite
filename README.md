# Borrower CLI Testing Server

A robust Rust-based server for automated testing of the Borrower CLI loan process. This server executes a comprehensive test suite, stores results in a SQLite database, and provides a RESTful API for interacting with the test system.

## Features

- **Automated Test Suite**: Complete end-to-end testing of the Borrower CLI loan lifecycle
- **RESTful API**: Simple interface for running tests and querying results
- **Persistent Storage**: All test results stored in SQLite for historical tracking
- **Error Handling**: Robust error handling with detailed logging
- **Retry Mechanisms**: Automatic retries for critical operations
- **Docker Support**: Easy deployment with Docker and Docker Compose

## Test Suite Steps

The test suite performs the following steps:

1. **Generate Credentials**: Creates a new mnemonic and derives BTC/LavaUSD addresses
2. **Fund Addresses**: Calls testnet faucets to fund the generated addresses
3. **Verify CLI**: Ensures the CLI is available and executable
4. **Create Loan**: Executes the CLI to create a new loan
5. **Extract Contract ID**: Captures the contract ID from the CLI output
6. **Repay Loan**: Executes the CLI to repay the loan
7. **Get Contract Details**: Retrieves contract details and saves to JSON
8. **Verify Closure**: Verifies the loan is properly closed with repayment

## API Endpoints

| Endpoint                    | Method | Description                                    |
| --------------------------- | ------ | ---------------------------------------------- |
| `/` or `/health`            | GET    | Health check to verify server is running       |
| `/run-test`                 | POST   | Run the complete test suite                    |
| `/results`                  | GET    | Get all test results                           |
| `/results/{id}`             | GET    | Get a specific test result by ID               |
| `/last-successful-mnemonic` | GET    | Get the mnemonic from the last successful test |

### API Examples

#### Run a Test

```bash
# Run with a random mnemonic
curl -X POST http://localhost:8080/run-test

# Run with a specific mnemonic
curl -X POST http://localhost:8080/run-test \
  -H "Content-Type: application/json" \
  -d '{"mnemonic": "your twelve word mnemonic phrase goes here"}'
```

#### Get Results

```bash
# Get all results
curl http://localhost:8080/results

# Get a specific result
curl http://localhost:8080/results/{result_id}

# Get last successful mnemonic
curl http://localhost:8080/last-successful-mnemonic
```

## Running with Docker

### Prerequisites

- Docker and Docker Compose installed
- Internet connection (for faucet calls and CLI download)

### Setup and Run

1. Clone the repository

   ```bash
   git clone <repository-url>
   cd borrower-cli-tester
   ```

2. Build and start the container

   ```bash
   docker-compose up -d
   ```

3. Check logs

   ```bash
   docker-compose logs -f
   ```

4. Stop the server
   ```bash
   docker-compose down
   ```

## Building and Running Locally

### Prerequisites

- Rust and Cargo installed
- Required dependencies: `libssl-dev`, `libpq-dev`

### Setup and Run

1. Install dependencies

   ```bash
   # Debian/Ubuntu
   apt-get update && apt-get install -y libssl-dev libpq-dev

   # macOS
   brew install openssl libpq
   ```

2. Build the project

   ```bash
   cargo build --release
   ```

3. Run the server
   ```bash
   ./target/release/borrower-cli-tester
   ```

## Configuration

The server can be configured using environment variables:

| Variable        | Description                  | Default                |
| --------------- | ---------------------------- | ---------------------- |
| `DATABASE_PATH` | Path to SQLite database file | `data/test_results.db` |
| `HOST`          | Host address to bind to      | `0.0.0.0`              |
| `PORT`          | Port to bind to              | `8080`                 |

## Project Structure

```
borrower-cli-tester/
├── src/
│   ├── main.rs         # Web server implementation
│   ├── db.rs           # Database functionality
│   └── test_suite.rs   # Test suite implementation
├── data/               # Data directory for SQLite storage
├── Cargo.toml          # Rust dependencies and configuration
├── Dockerfile          # Docker build instructions
├── docker-compose.yaml # Docker Compose configuration
└── README.md           # This file
```

## Troubleshooting

### Common Issues

- **Faucet Connection Errors**: The testnet faucets may occasionally be unavailable or rate-limited. The test will log these errors but continue execution.
- **Permission Errors**: Ensure the data directory is writable if using Docker volumes.
- **Database Lock Errors**: If you see SQLite lock errors, it may indicate concurrent access to the database.

### Debugging

- Check the server logs for detailed information about test progress and errors
- Examine the SQLite database for test results and error details
- Try running with a known working mnemonic using the POST API

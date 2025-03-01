# Borrower CLI Test Server

A Rust-based test server that runs the Borrower CLI test suite and stores results in a SQLite database.

## Features

- Automated testing of the Borrower CLI loan process
- RESTful API for running tests and retrieving results
- Persistent storage of test results in a SQLite database
- Docker support for easy deployment

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` or `/health` | GET | Health check to verify server is running |
| `/run-test` | POST | Run the complete test suite |
| `/results` | GET | Get all test results |
| `/results/{id}` | GET | Get a specific test result by ID |

## Test Suite Steps

The test suite executes the following steps:

1. Generate a new mnemonic and addresses for BTC and LavaUSD
2. Call testnet faucets to fund the addresses
3. Download and install the CLI
4. Create a new loan using the CLI
5. Extract the contract ID from the output
6. Repay the loan
7. Get contract details and save to a JSON file
8. Verify that the loan is closed with repayment

## Building and Running

### Using Docker

The easiest way to run the server is with Docker Compose:

```bash
# Build and start the container
docker-compose up -d

# To view logs
docker-compose logs -f

# To stop the server
docker-compose down
```

### Building Locally

If you prefer to build and run locally:

```bash
# Install dependencies
apt-get update && apt-get install -y libpq-dev

# Build the project
cargo build --release

# Run the server
./target/release/borrower-cli-tester
```

## Environment Variables

You can customize the server with these environment variables:

- `DATABASE_PATH`: Path to SQLite database file (default: `data/test_results.db`)
- `HOST`: Host address to bind (default: `0.0.0.0`)
- `PORT`: Port to bind (default: `8080`)

## Example API Usage

### Running a Test

```bash
curl -X POST http://localhost:8080/run-test
```

### Getting All Test Results

```bash
curl http://localhost:8080/results
```

### Getting a Specific Test Result

```bash
curl http://localhost:8080/results/{result_id}
```

## Troubleshooting

- **CLI Download Issues**: If you encounter download issues, verify your network connection and internet access from the container.
- **Faucet Failures**: The testnet faucet may occasionally be down or rate-limited. The test suite continues even if faucet calls aren't successful.
- **Permission Issues**: Ensure the data directory is writable if using volumes with Docker.

## Extending the Project

Some ideas for extending this project:

- Add authentication for the API endpoints
- Implement webhook notifications when tests complete
- Add test parallelization for running multiple tests simultaneously
- Create a web UI for viewing test results
- Add metrics and monitoring for the test server
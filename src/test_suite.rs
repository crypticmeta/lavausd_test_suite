use crate::db::TestResult;
use bip39::{Language, Mnemonic};
use bitcoin::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::key::PrivateKey;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, Network, PublicKey};
use chrono::Utc;
use rand::{rngs::OsRng, RngCore};
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug)]
pub enum TestError {
    Crypto(String),
    Network(String),
    Process(String),
    Io(String),
    Parsing(String),
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TestError::Crypto(msg) => write!(f, "Crypto error: {}", msg),
            TestError::Network(msg) => write!(f, "Network error: {}", msg),
            TestError::Process(msg) => write!(f, "Process error: {}", msg),
            TestError::Io(msg) => write!(f, "IO error: {}", msg),
            TestError::Parsing(msg) => write!(f, "Parsing error: {}", msg),
        }
    }
}

impl Error for TestError {}

impl From<std::io::Error> for TestError {
    fn from(err: std::io::Error) -> Self {
        TestError::Io(err.to_string())
    }
}

impl From<reqwest::Error> for TestError {
    fn from(err: reqwest::Error) -> Self {
        TestError::Network(err.to_string())
    }
}

pub struct TestSuite {
    logs: String,
    steps_completed: Vec<String>,
    mnemonic: String,
    btc_address: String,
    lava_pubkey: String,
    contract_id: Option<String>,
    mnemonic_provided: bool,
}

impl TestSuite {
    pub fn new() -> Self {
        TestSuite {
            logs: String::new(),
            steps_completed: Vec::new(),
            mnemonic: String::new(),
            btc_address: String::new(),
            lava_pubkey: String::new(),
            contract_id: None,
            mnemonic_provided: false,
        }
    }

    fn create_result(&self, success: bool, details: String) -> TestResult {
        TestResult {
            id: Uuid::new_v4().to_string(),
            success,
            details,
            mnemonic: self.mnemonic.clone(),
            btc_address: self.btc_address.clone(),
            lava_pubkey: self.lava_pubkey.clone(),
            contract_id: self.contract_id.clone(),
            steps_completed: self.steps_completed.clone(),
            logs: self.logs.clone(),
            timestamp: Utc::now(),
        }
    }

    // Method to set a predefined mnemonic
    pub fn with_mnemonic(mut self, mnemonic: String) -> Self {
        self.mnemonic = mnemonic;
        self.mnemonic_provided = true;
        self
    }

    fn log(&mut self, message: &str) {
        println!("{}", message);
        self.logs.push_str(message);
        self.logs.push('\n');
    }

    fn add_step(&mut self, step_name: &str) {
        self.steps_completed.push(step_name.to_string());
        self.log(&format!("✓ {}", step_name));
    }

    pub async fn run(&mut self) -> TestResult {
        self.log("Starting Borrower CLI Test Suite");

        // Step 1: Generate mnemonic and addresses
        if let Err(e) = self.step1_generate_credentials() {
            self.log(&format!("Error in step 1: {}", e));
            return self.create_result(false, format!("Error in step 1: {}", e));
        }

        // Step 2: Call testnet faucet
        if let Err(e) = self.step2_call_faucet().await {
            self.log(&format!("Error in step 2: {}", e));
            return self.create_result(false, format!("Error in step 2: {}", e));
        }

        // Step 3: Check CLI
        if let Err(e) = self.step3_check_cli() {
            self.log(&format!("Error in step 3: {}", e));
            return self.create_result(false, format!("Error in step 3: {}", e));
        }

        // Step 4: Create a loan with retries
        let max_attempts = 3;
        let mut loan_created = false;

        for attempt in 1..=max_attempts {
            self.log(&format!(
                "Creating a loan (attempt {}/{})",
                attempt, max_attempts
            ));

            match self.step4_create_loan() {
                Ok(_) => {
                    self.log("Loan creation successful");
                    loan_created = true;
                    break;
                }
                Err(e) => {
                    self.log(&format!(
                        "Error in loan creation attempt {}: {}",
                        attempt, e
                    ));

                    if attempt < max_attempts {
                        self.log("Waiting 30 seconds before retrying loan creation...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    } else {
                        self.log("All loan creation attempts failed");
                        return self.create_result(
                            false,
                            format!("Error in step 4 after {} attempts: {}", max_attempts, e),
                        );
                    }
                }
            }
        }

        if !loan_created {
            return self.create_result(
                false,
                "Failed to create loan after all retry attempts".to_string(),
            );
        }

        self.log("Waiting 1 minute before proceeding to the next step...");
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

        // Step 6: Repay the loan with retries
        let mut loan_repaid = false;

        for attempt in 1..=max_attempts {
            self.log(&format!(
                "Repaying the loan (attempt {}/{})",
                attempt, max_attempts
            ));

            match self.step6_repay_loan() {
                Ok(_) => {
                    self.log("Loan repayment successful");
                    loan_repaid = true;
                    break;
                }
                Err(e) => {
                    self.log(&format!(
                        "Error in loan repayment attempt {}: {}",
                        attempt, e
                    ));

                    if attempt < max_attempts {
                        self.log("Waiting 30 seconds before retrying loan repayment...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    } else {
                        self.log("All loan repayment attempts failed");
                        return self.create_result(
                            false,
                            format!("Error in step 6 after {} attempts: {}", max_attempts, e),
                        );
                    }
                }
            }
        }

        if !loan_repaid {
            return self.create_result(
                false,
                "Failed to repay loan after all retry attempts".to_string(),
            );
        }

        self.log("Waiting 1 minute before proceeding to the next step...");
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

        // Step 7: Get contract details
        if let Err(e) = self.step7_get_contract_details() {
            self.log(&format!("Error in step 7: {}", e));
            return self.create_result(false, format!("Error in step 7: {}", e));
        }

        // Step 8 & 9: Check the JSON file
        let success = match self.step8_check_json() {
            Ok(success) => success,
            Err(e) => {
                self.log(&format!("Error in step 8: {}", e));
                return self.create_result(false, format!("Error in step 8: {}", e));
            }
        };

        // Create final test result
        self.create_result(
            success,
            if success {
                "Test completed successfully".to_string()
            } else {
                "Test failed - loan is not closed with repayment".to_string()
            },
        )
    }

    // Helper method to log commands before execution
    fn log_command(&mut self, cmd: &Command) -> Result<(), TestError> {
        // Attempt to reconstruct the command as it would be executed in shell
        let program = cmd.get_program().to_string_lossy();

        let args: Vec<String> = cmd
            .get_args()
            .map(|arg| format!("\"{}\"", arg.to_string_lossy()))
            .collect();

        let env_vars: Vec<String> = cmd
            .get_envs()
            .map(|(key, val)| {
                if let Some(val) = val {
                    format!("{}=\"{}\"", key.to_string_lossy(), val.to_string_lossy())
                } else {
                    format!("{}=", key.to_string_lossy())
                }
            })
            .collect();

        let command_str = format!("{} {} {}", env_vars.join(" "), program, args.join(" "));

        self.log(&format!("Executing command: {}", command_str));
        Ok(())
    }

    fn step1_generate_credentials(&mut self) -> Result<(), TestError> {
        self.log("Step 1: Generating or using provided credentials");

        if !self.mnemonic_provided {
            // Generate random entropy for the mnemonic (16 bytes for 12 words)
            let mut entropy = [0u8; 16];
            OsRng.fill_bytes(&mut entropy);

            // Generate a new mnemonic from entropy
            let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
                .map_err(|e| TestError::Crypto(format!("Failed to generate mnemonic: {}", e)))?;

            // Create the mnemonic phrase string
            self.mnemonic = mnemonic.to_string();
            self.log(&format!("Generated mnemonic: {}", self.mnemonic));
        } else {
            self.log(&format!("Using provided mnemonic: {}", self.mnemonic));
        }

        // Generate BTC address
        self.btc_address = self.generate_btc_address(&self.mnemonic)?;
        self.log(&format!("Generated BTC address: {}", self.btc_address));

        // Generate LavaUSD address
        // For now, use a known working pubkey for testing
        // TODO: Implement proper Solana key derivation
        self.lava_pubkey = "CU9KRXJobqo1HVbaJwoWpnboLFXw3bef54xJ1dewXzcf".to_string();
        self.log(&format!(
            "Using known working LavaUSD pubkey: {}",
            self.lava_pubkey
        ));

        self.add_step("Step 1: Generated/used credentials");
        Ok(())
    }

    async fn step2_call_faucet(&mut self) -> Result<(), TestError> {
        self.log("Step 2: Calling testnet faucet");

        // Call BTC faucet
        let client = Client::new();
        let btc_response = client
            .post("https://faucet.testnet.lava.xyz/mint-mutinynet")
            .header("Content-Type", "application/json")
            .json(&json!({
                "address": self.btc_address,
                "sats": 100000
            }))
            .send()
            .await?;

        let btc_status = btc_response.status();
        let btc_body = btc_response.text().await?;
        self.log(&format!(
            "BTC faucet response ({} {}): {}",
            btc_status.as_u16(),
            btc_status.canonical_reason().unwrap_or("Unknown"),
            btc_body
        ));

        // Call LavaUSD faucet with retries
        let max_lava_attempts = 3;
        for attempt in 1..=max_lava_attempts {
            self.log(&format!(
                "LavaUSD faucet attempt {}/{}",
                attempt, max_lava_attempts
            ));

            let lava_response = client
                .post("https://faucet.testnet.lava.xyz/transfer-lava-usd")
                .header("Content-Type", "application/json")
                .json(&json!({
                    "pubkey": self.lava_pubkey
                }))
                .send()
                .await?;

            let lava_status = lava_response.status();
            let lava_body = lava_response.text().await?;
            self.log(&format!(
                "LavaUSD faucet response ({} {}): {}",
                lava_status.as_u16(),
                lava_status.canonical_reason().unwrap_or("Unknown"),
                lava_body
            ));

            if lava_status.is_success() {
                break;
            } else if attempt < max_lava_attempts {
                self.log("LavaUSD faucet call failed, retrying in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }

        self.add_step("Step 2: Called testnet faucet");
        Ok(())
    }

    fn step3_check_cli(&mut self) -> Result<(), TestError> {
        self.log("Step 3: Checking for CLI");

        // Check if CLI exists and is executable
        let cli_path = "./loans-borrower-cli";
        if !Path::new(cli_path).exists() {
            return Err(TestError::Process(format!(
                "CLI not found at: {}",
                cli_path
            )));
        }

        // Make sure it's executable
        let chmod_output = Command::new("chmod").arg("+x").arg(cli_path).output()?;

        if !chmod_output.status.success() {
            self.log(&format!(
                "Warning: Could not set execute permission on CLI: {}",
                String::from_utf8_lossy(&chmod_output.stderr)
            ));
            // Continue anyway, it might already be executable
        }

        // Create any necessary directories that the CLI might need
        let data_dir = "./data";
        if !Path::new(data_dir).exists() {
            fs::create_dir_all(data_dir)
                .map_err(|e| TestError::Io(format!("Failed to create data directory: {}", e)))?;
        }

        // Ensure the current directory is writable
        let test_file = "./write_test";
        match fs::File::create(test_file) {
            Ok(_) => {
                fs::remove_file(test_file)?;
            }
            Err(e) => {
                return Err(TestError::Io(format!(
                    "Current directory is not writable: {}",
                    e
                )));
            }
        }

        self.add_step("Step 3: Verified CLI availability");
        Ok(())
    }

    fn step4_create_loan(&mut self) -> Result<(), TestError> {
        self.log("Step 4: Creating a new loan");

        // Make sure we're using the full path to the CLI
        let cli_path = fs::canonicalize("./loans-borrower-cli")
            .map_err(|e| TestError::Io(format!("Failed to get absolute path to CLI: {}", e)))?;

        // Create output directory for potential files
        let output_dir = "./output";
        if !Path::new(output_dir).exists() {
            fs::create_dir_all(output_dir)
                .map_err(|e| TestError::Io(format!("Failed to create output directory: {}", e)))?;
        }

        // Verbose logging before running the command
        self.log(&format!("CLI path: {:?}", cli_path));
        self.log(&format!(
            "Working directory: {:?}",
            std::env::current_dir().unwrap_or_default()
        ));
        self.log(&format!("Running command with mnemonic: {}", self.mnemonic));

        let mut cmd = Command::new(&cli_path);
        cmd.env("MNEMONIC", &self.mnemonic)
            .arg("--testnet")
            .arg("--disable-backup-contracts")
            .arg("borrow")
            .arg("init")
            .arg("--loan-capital-asset")
            .arg("solana-lava-usd")
            .arg("--ltv-ratio-bp")
            .arg("5000")
            .arg("--loan-duration-days")
            .arg("4")
            .arg("--loan-amount")
            .arg("2")
            .arg("--finalize");

        // Log the command before execution
        self.log_command(&cmd)?;

        let output = cmd
            .output()
            .map_err(|e| TestError::Io(format!("Failed to execute CLI: {} ({})", e, e.kind())))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        self.log(&format!("Borrow init stdout: {}", stdout));
        if !stderr.is_empty() {
            self.log(&format!("Borrow init stderr: {}", stderr));
        }

        if !output.status.success() {
            return Err(TestError::Process(format!(
                "Failed to create loan: exit code {}",
                output.status
            )));
        }

        // Step 5: Extract contract-id using regex
        self.log("Step 5: Capturing contract-id");

        self.log(&format!(
            "Searching for contract ID in output. Length: {}",
            stdout.len()
        ));
        // Search for contract ID in both stdout and stderr
        let re = Regex::new(r"New contract ID: ([a-zA-Z0-9]+)").unwrap();

        // Try to find in stdout first
        let contract_id_opt = re
            .captures(&stdout)
            .or_else(|| re.captures(&stderr)) // If not found in stdout, try stderr
            .map(|captures| captures.get(1).unwrap().as_str().to_string());

        if let Some(id) = contract_id_opt {
            self.log(&format!("Captured contract-id: {}", id));
            self.contract_id = Some(id);
            self.add_step("Step 5: Captured contract-id");
        } else {
            self.log(&format!(
                "Searching for contract ID in stdout. Length: {}",
                stdout.len()
            ));
            self.log(&format!(
                "Searching for contract ID in stderr. Length: {}",
                stderr.len()
            ));
            return Err(TestError::Parsing(
                "Failed to extract contract-id from stdout or stderr".to_string(),
            ));
        }

        self.add_step("Step 4: Created a new loan");
        Ok(())
    }

    fn step6_repay_loan(&mut self) -> Result<(), TestError> {
        self.log("Step 6: Repaying the loan");

        let contract_id = match &self.contract_id {
            Some(id) => id,
            None => return Err(TestError::Parsing("Missing contract-id".to_string())),
        };

        // Use full path to CLI
        let cli_path = fs::canonicalize("./loans-borrower-cli")
            .map_err(|e| TestError::Io(format!("Failed to get absolute path to CLI: {}", e)))?;

        let mut cmd = Command::new(&cli_path);
        cmd.env("MNEMONIC", &self.mnemonic)
            .arg("--testnet")
            .arg("--disable-backup-contracts")
            .arg("borrow")
            .arg("repay")
            .arg("--contract-id")
            .arg(contract_id);

        // Log the command before execution
        self.log_command(&cmd)?;

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        self.log(&format!("Repay stdout: {}", stdout));
        if !stderr.is_empty() {
            self.log(&format!("Repay stderr: {}", stderr));
        }

        if !output.status.success() {
            return Err(TestError::Process("Failed to repay loan".to_string()));
        }

        self.add_step("Step 6: Repaid the loan");
        Ok(())
    }

    fn step7_get_contract_details(&mut self) -> Result<(), TestError> {
        self.log("Step 7: Getting contract details");

        let contract_id = match &self.contract_id {
            Some(id) => id,
            None => return Err(TestError::Parsing("Missing contract-id".to_string())),
        };

        let json_file = format!("./output/{}.json", contract_id);

        // Use full path to CLI
        let cli_path = fs::canonicalize("./loans-borrower-cli")
            .map_err(|e| TestError::Io(format!("Failed to get absolute path to CLI: {}", e)))?;

        let output = Command::new(&cli_path)
            .env("MNEMONIC", &self.mnemonic)
            .arg("--testnet")
            .arg("--disable-backup-contracts")
            .arg("get-contract")
            .arg("--contract-id")
            .arg(contract_id)
            // .arg("--disable-contracts-backup")
            .arg("--verbose")
            .arg("--output-file")
            .arg(&json_file)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        self.log(&format!("Get contract stdout: {}", stdout));
        if !stderr.is_empty() {
            self.log(&format!("Get contract stderr: {}", stderr));
        }

        if !output.status.success() {
            return Err(TestError::Process(
                "Failed to get contract details".to_string(),
            ));
        }

        self.add_step("Step 7: Got contract details");
        Ok(())
    }

    fn step8_check_json(&mut self) -> Result<bool, TestError> {
        self.log("Step 8: Checking JSON file for closed status");

        let contract_id = match &self.contract_id {
            Some(id) => id,
            None => return Err(TestError::Parsing("Missing contract-id".to_string())),
        };

        let json_file = format!("./output/{}.json", contract_id);

        if !Path::new(&json_file).exists() {
            return Err(TestError::Io(format!("JSON file not found: {}", json_file)));
        }

        let content = fs::read_to_string(&json_file)?;

        // Add more detailed logging
        self.log("JSON content loading successful");
        self.log(&format!("JSON content length: {} bytes", content.len()));
        self.log("First 100 characters of JSON: ");
        if content.len() > 100 {
            self.log(&content[0..100]);
        } else {
            self.log(&content);
        }

        let json: Value = serde_json::from_str(&content)
            .map_err(|e| TestError::Parsing(format!("Failed to parse JSON: {}", e)))?;

        // Step 9: Check if loan is closed with repayment
        self.log("Step 9: Verifying loan is closed with repayment");

        let is_closed = json.get("Closed").is_some();
        self.log(&format!("Is Closed object present: {}", is_closed));

        // Look for outcome/repayment inside the Closed object
        let has_repayment = if let Some(closed) = json.get("Closed") {
            self.log("Found 'Closed' object in JSON");

            if let Some(outcome) = closed.get("outcome") {
                self.log("Found 'outcome' object in Closed");

                if let Some(repayment) = outcome.get("repayment") {
                    self.log("Found 'repayment' object in outcome");

                    let has_txid = repayment.get("collateral_repayment_txid").is_some();
                    self.log(&format!("Has collateral_repayment_txid: {}", has_txid));
                    has_txid
                } else {
                    self.log("No 'repayment' object found in outcome");
                    false
                }
            } else {
                self.log("No 'outcome' object found in Closed");
                false
            }
        } else {
            self.log("No 'Closed' object found in JSON");
            false
        };

        // Try the pointer approach too
        let pointer_result = json
            .pointer("/Closed/outcome/repayment/collateral_repayment_txid")
            .is_some();
        self.log(&format!("JSON pointer check result: {}", pointer_result));

        if is_closed && has_repayment {
            self.log("Loan is closed with repayment - TEST PASSED");
            self.add_step("Step 9: Verified loan is closed with repayment");
            Ok(true)
        } else {
            // Fix: Use format! to create the debug string first
            self.log(&format!(
                "Debug - is_closed: {}, has_repayment: {}",
                is_closed, has_repayment
            ));
            self.log("Loan is not closed with repayment - TEST FAILED");
            Ok(false)
        }
    }
    fn generate_btc_address(&self, mnemonic: &str) -> Result<String, TestError> {
        // Parse the mnemonic
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, mnemonic)
            .map_err(|e| TestError::Crypto(format!("Invalid mnemonic: {}", e)))?;

        // Generate seed from mnemonic
        let seed = mnemonic.to_seed("");

        let secp = Secp256k1::new();
        let master = ExtendedPrivKey::new_master(Network::Testnet, &seed)
            .map_err(|e| TestError::Crypto(format!("Failed to create master key: {}", e)))?;

        // Derive path for Testnet P2WPKH (BIP84)
        let path = DerivationPath::from_str("m/84'/1'/0'/0/0")
            .map_err(|e| TestError::Crypto(format!("Invalid derivation path: {}", e)))?;

        let child = master
            .derive_priv(&secp, &path)
            .map_err(|e| TestError::Crypto(format!("Failed to derive child key: {}", e)))?;

        let private_key = PrivateKey::new(child.private_key, Network::Testnet);
        let public_key = PublicKey::from_private_key(&secp, &private_key);

        // Create the BTC testnet address (p2wpkh)
        let address = Address::p2wpkh(&public_key, Network::Testnet)
            .map_err(|e| TestError::Crypto(format!("Failed to create address: {}", e)))?;

        Ok(address.to_string())
    }

    // fn generate_lava_pubkey(&self, mnemonic: &str) -> Result<String, TestError> {
    //     // Note: In a real implementation, we would use a proper Solana library
    //     // For testing purposes, use a hard-coded working key
    //     let _mnemonic = mnemonic; // Acknowledge the parameter but don't use it

    //     // Return a known working key that works with the faucet
    //     Ok("CU9KRXJobqo1HVbaJwoWpnboLFXw3bef54xJ1dewXzcf".to_string())
    // }
}

use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub success: bool,
    pub details: String,
    pub mnemonic: String,
    pub btc_address: String,
    pub lava_pubkey: String,
    pub contract_id: Option<String>,
    pub steps_completed: Vec<String>,
    pub logs: String,
    pub timestamp: DateTime<Utc>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = Path::new(db_path).parent() {
            fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(1),
                    Some(format!("Failed to create directory: {}", e)),
                )
            })?;
        }

        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS test_results (
                id TEXT PRIMARY KEY,
                success INTEGER NOT NULL,
                details TEXT NOT NULL,
                mnemonic TEXT NOT NULL,
                btc_address TEXT NOT NULL,
                lava_pubkey TEXT NOT NULL,
                contract_id TEXT,
                steps_completed TEXT NOT NULL,
                logs TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Database { conn })
    }

    pub fn save_result(&self, result: &TestResult) -> Result<()> {
        self.conn.execute(
            "INSERT INTO test_results (
                id, success, details, mnemonic, btc_address, lava_pubkey, 
                contract_id, steps_completed, logs, timestamp
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                result.id,
                result.success as i32,
                result.details,
                result.mnemonic,
                result.btc_address,
                result.lava_pubkey,
                result.contract_id,
                serde_json::to_string(&result.steps_completed).unwrap(),
                result.logs,
                result.timestamp.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn get_all_results(&self) -> Result<Vec<TestResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, success, details, mnemonic, btc_address, lava_pubkey, 
             contract_id, steps_completed, logs, timestamp 
             FROM test_results 
             ORDER BY timestamp DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let steps_json: String = row.get(7)?;
            let steps: Vec<String> = serde_json::from_str(&steps_json).map_err(|_| {
                rusqlite::Error::InvalidColumnType(7, "Invalid JSON".to_string(), Type::Text)
            })?;

            let timestamp_str: String = row.get(9)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        9,
                        "Invalid timestamp".to_string(),
                        Type::Text,
                    )
                })?
                .with_timezone(&Utc);

            Ok(TestResult {
                id: row.get(0)?,
                success: row.get::<_, i32>(1)? != 0,
                details: row.get(2)?,
                mnemonic: row.get(3)?,
                btc_address: row.get(4)?,
                lava_pubkey: row.get(5)?,
                contract_id: row.get(6)?,
                steps_completed: steps,
                logs: row.get(8)?,
                timestamp,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    pub fn get_result(&self, id: &str) -> Result<Option<TestResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, success, details, mnemonic, btc_address, lava_pubkey, 
             contract_id, steps_completed, logs, timestamp 
             FROM test_results 
             WHERE id = ?",
        )?;

        let rows = stmt.query_map([id], |row| {
            let steps_json: String = row.get(7)?;
            let steps: Vec<String> = serde_json::from_str(&steps_json).map_err(|_| {
                rusqlite::Error::InvalidColumnType(7, "Invalid JSON".to_string(), Type::Text)
            })?;

            let timestamp_str: String = row.get(9)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        9,
                        "Invalid timestamp".to_string(),
                        Type::Text,
                    )
                })?
                .with_timezone(&Utc);

            Ok(TestResult {
                id: row.get(0)?,
                success: row.get::<_, i32>(1)? != 0,
                details: row.get(2)?,
                mnemonic: row.get(3)?,
                btc_address: row.get(4)?,
                lava_pubkey: row.get(5)?,
                contract_id: row.get(6)?,
                steps_completed: steps,
                logs: row.get(8)?,
                timestamp,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results.into_iter().next())
    }

    pub fn get_last_successful_test(&self) -> Result<Option<TestResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, success, details, mnemonic, btc_address, lava_pubkey, 
             contract_id, steps_completed, logs, timestamp 
             FROM test_results 
             WHERE success = 1
             ORDER BY timestamp DESC 
             LIMIT 1",
        )?;

        let rows = stmt.query_map([], |row| {
            let steps_json: String = row.get(7)?;
            let steps: Vec<String> = serde_json::from_str(&steps_json).map_err(|_| {
                rusqlite::Error::InvalidColumnType(7, "Invalid JSON".to_string(), Type::Text)
            })?;

            let timestamp_str: String = row.get(9)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        9,
                        "Invalid timestamp".to_string(),
                        Type::Text,
                    )
                })?
                .with_timezone(&Utc);

            Ok(TestResult {
                id: row.get(0)?,
                success: row.get::<_, i32>(1)? != 0,
                details: row.get(2)?,
                mnemonic: row.get(3)?,
                btc_address: row.get(4)?,
                lava_pubkey: row.get(5)?,
                contract_id: row.get(6)?,
                steps_completed: steps,
                logs: row.get(8)?,
                timestamp,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results.into_iter().next())
    }
}

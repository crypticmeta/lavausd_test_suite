mod db;
mod test_suite;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use db::{Database, TestResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::sync::Mutex;
use test_suite::TestSuite;

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    mnemonic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_faucet: Option<bool>,
}

struct AppState {
    db: Mutex<Database>,
}

async fn health_check() -> impl Responder {
    let response = ApiResponse {
        success: true,
        message: "Borrower CLI Test Server is running".to_string(),
        data: None::<()>,
        timestamp: Utc::now().to_rfc3339(),
    };
    HttpResponse::Ok().json(response)
}

async fn run_test(
    options: web::Json<TestOptions>,
    data: web::Data<AppState>
) -> impl Responder {
    let mut test_suite = TestSuite::new();
    
    // Apply options if provided
    if let Some(mnemonic) = &options.mnemonic {
        test_suite = test_suite.with_mnemonic(mnemonic.clone());
    }
    
    // Run the test
    match test_suite.run().await {
        Ok(result) => {
            let success = result.success;
            let db_result = data.db.lock().unwrap().save_result(&result);
            
            if let Err(e) = db_result {
                eprintln!("Failed to save test result to database: {}", e);
            }
            
            let response = ApiResponse {
                success,
                message: if success {
                    "Test completed successfully".to_string()
                } else {
                    "Test failed".to_string()
                },
                data: Some(result),
                timestamp: Utc::now().to_rfc3339(),
            };
            
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            let response = ApiResponse {
                success: false,
                message: format!("Test error: {}", e),
                data: None::<()>,
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

async fn get_all_results(data: web::Data<AppState>) -> impl Responder {
    match data.db.lock().unwrap().get_all_results() {
        Ok(results) => {
            let response = ApiResponse {
                success: true,
                message: format!("Found {} test results", results.len()),
                data: Some(results),
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            let response = ApiResponse {
                success: false,
                message: format!("Database error: {}", e),
                data: None::<Vec<TestResult>>,
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

async fn get_result(path: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    match data.db.lock().unwrap().get_result(&id) {
        Ok(Some(result)) => {
            let response = ApiResponse {
                success: true,
                message: "Test result found".to_string(),
                data: Some(result),
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::Ok().json(response)
        }
        Ok(None) => {
            let response = ApiResponse {
                success: false,
                message: format!("Test result with ID {} not found", id),
                data: None::<TestResult>,
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::NotFound().json(response)
        }
        Err(e) => {
            let response = ApiResponse {
                success: false,
                message: format!("Database error: {}", e),
                data: None::<TestResult>,
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

async fn get_last_successful_mnemonic(data: web::Data<AppState>) -> impl Responder {
    match data.db.lock().unwrap().get_last_successful_test() {
        Ok(Some(result)) => {
            let response = ApiResponse {
                success: true,
                message: "Last successful test found".to_string(),
                data: Some(json!({
                    "mnemonic": result.mnemonic,
                    "btc_address": result.btc_address,
                    "lava_pubkey": result.lava_pubkey,
                    "timestamp": result.timestamp
                })),
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::Ok().json(response)
        }
        Ok(None) => {
            let response = ApiResponse {
                success: false,
                message: "No successful tests found".to_string(),
                data: None::<()>,
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::NotFound().json(response)
        }
        Err(e) => {
            let response = ApiResponse {
                success: false,
                message: format!("Database error: {}", e),
                data: None::<()>,
                timestamp: Utc::now().to_rfc3339(),
            };
            HttpResponse::InternalServerError().json(response)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize database
    let db_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "data/test_results.db".to_string());
    println!("Using database at: {}", db_path);
    
    let db = match Database::new(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            std::process::exit(1);
        }
    };
    
    let app_state = web::Data::new(AppState {
        db: Mutex::new(db),
    });
    
    // Get host and port from environment or use defaults
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("{}:{}", host, port);
    
    println!("Starting Borrower CLI Test Server on {}", bind_address);
    
    // Ensure the CLI is executable before starting the server
    let cli_path = "./loans-borrower-cli";
    if std::path::Path::new(cli_path).exists() {
        match std::process::Command::new("chmod").arg("+x").arg(cli_path).output() {
            Ok(_) => println!("CLI permissions set"),
            Err(e) => println!("Warning: Could not set CLI permissions: {}", e),
        }
    } else {
        println!("Warning: CLI not found at {}. It will be downloaded on first test run.", cli_path);
    }
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/", web::get().to(health_check))
            .route("/health", web::get().to(health_check))
            .route("/run-test", web::post().to(run_test))
            .route("/results", web::get().to(get_all_results))
            .route("/results/{id}", web::get().to(get_result))
            .route("/last-successful-mnemonic", web::get().to(get_last_successful_mnemonic))
    })
    .bind(bind_address)?
    .run()
    .await
}
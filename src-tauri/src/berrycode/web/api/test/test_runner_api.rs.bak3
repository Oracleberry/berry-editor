//! Test Runner API
//! Runs tests and returns results

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct TestRunnerState {
    // Could add test history, caching, etc.
}

impl TestRunnerState {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Deserialize)]
pub struct RunTestsRequest {
    pub session_id: String,
    pub project_path: String,
    pub test_framework: String, // "cargo", "npm", "pytest", "go", etc.
    pub test_filter: Option<String>, // Optional test name filter
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub name: String,
    pub status: String, // "passed", "failed", "skipped"
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunTestsResponse {
    pub success: bool,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
    pub output: String,
    pub tests: Vec<TestResult>,
    pub error: Option<String>,
}

/// Run tests endpoint
pub async fn run_tests(
    State(state): State<Arc<Mutex<TestRunnerState>>>,
    Json(request): Json<RunTestsRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        framework = %request.test_framework,
        "Running tests"
    );

    let start_time = std::time::Instant::now();

    // Build test command based on framework
    let (program, args) = match request.test_framework.as_str() {
        "cargo" => {
            let mut args = vec!["test", "--color", "always"];
            if let Some(filter) = &request.test_filter {
                args.push(filter);
            }
            ("cargo", args)
        }
        "npm" => ("npm", vec!["test"]),
        "yarn" => ("yarn", vec!["test"]),
        "pytest" => {
            let mut args = vec!["-v", "--color=yes"];
            if let Some(filter) = &request.test_filter {
                args.push("-k");
                args.push(filter);
            }
            ("pytest", args)
        }
        "go" => {
            let mut args = vec!["test", "./..."];
            if let Some(filter) = &request.test_filter {
                args.push("-run");
                args.push(filter);
            }
            ("go", args)
        }
        "jest" => ("npx", vec!["jest"]),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(RunTestsResponse {
                    success: false,
                    total: 0,
                    passed: 0,
                    failed: 0,
                    skipped: 0,
                    duration_ms: 0,
                    output: String::new(),
                    tests: Vec::new(),
                    error: Some(format!("Unsupported test framework: {}", request.test_framework)),
                }),
            );
        }
    };

    // Run the test command
    let output = Command::new(program)
        .args(&args)
        .current_dir(&request.project_path)
        .output();

    let duration_ms = start_time.elapsed().as_millis() as u64;

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let full_output = format!("{}\n{}", stdout, stderr);

            // Parse test results (basic parsing)
            let (total, passed, failed, skipped) = parse_test_results(&full_output, &request.test_framework);

            let success = output.status.success();

            (
                StatusCode::OK,
                Json(RunTestsResponse {
                    success,
                    total,
                    passed,
                    failed,
                    skipped,
                    duration_ms,
                    output: full_output,
                    tests: Vec::new(), // Could parse individual tests
                    error: if success { None } else { Some("Tests failed".to_string()) },
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RunTestsResponse {
                success: false,
                total: 0,
                passed: 0,
                failed: 0,
                skipped: 0,
                duration_ms,
                output: String::new(),
                tests: Vec::new(),
                error: Some(format!("Failed to run tests: {}", e)),
            }),
        ),
    }
}

fn parse_test_results(output: &str, framework: &str) -> (usize, usize, usize, usize) {
    match framework {
        "cargo" => {
            // Parse Cargo test output
            // Example: "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            if let Some(line) = output.lines().find(|l| l.contains("test result:")) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let passed = parts.iter().position(|&p| p == "passed;")
                    .and_then(|i| parts.get(i.saturating_sub(1)))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let failed = parts.iter().position(|&p| p == "failed;")
                    .and_then(|i| parts.get(i.saturating_sub(1)))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let ignored = parts.iter().position(|&p| p == "ignored;")
                    .and_then(|i| parts.get(i.saturating_sub(1)))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                let total = passed + failed + ignored;
                return (total, passed, failed, ignored);
            }
        }
        "pytest" => {
            // Parse pytest output
            // Example: "5 passed, 1 failed, 2 skipped in 1.23s"
            if let Some(line) = output.lines().rev().find(|l| l.contains("passed") || l.contains("failed")) {
                let passed = line.split_whitespace()
                    .find_map(|w| if w.contains("passed") {
                        line.split_whitespace().take_while(|&x| x != "passed").last().and_then(|s| s.parse().ok())
                    } else { None })
                    .unwrap_or(0);
                let failed = line.split_whitespace()
                    .find_map(|w| if w.contains("failed") {
                        line.split_whitespace().take_while(|&x| x != "failed").last().and_then(|s| s.parse().ok())
                    } else { None })
                    .unwrap_or(0);
                let skipped = line.split_whitespace()
                    .find_map(|w| if w.contains("skipped") {
                        line.split_whitespace().take_while(|&x| x != "skipped").last().and_then(|s| s.parse().ok())
                    } else { None })
                    .unwrap_or(0);

                let total = passed + failed + skipped;
                return (total, passed, failed, skipped);
            }
        }
        _ => {}
    }

    // Default: count lines with "ok" or "FAIL"
    let passed = output.lines().filter(|l| l.contains(" ok") || l.contains("PASS")).count();
    let failed = output.lines().filter(|l| l.contains("FAIL") || l.contains(" failed")).count();
    let total = passed + failed;
    (total, passed, failed, 0)
}

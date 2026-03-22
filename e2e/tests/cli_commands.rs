// CLI command tests
// Note: Command::output() is blocking, must use spawn_blocking in async tests

use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

fn hub_cli_path() -> String {
    // When running under cargo-llvm-cov, use the instrumented binary
    if let Ok(target_dir) = std::env::var("CARGO_LLVM_COV_TARGET_DIR") {
        // Find the instrumented binary in deps directory
        let deps_dir = format!("{}/debug/deps", target_dir);
        if let Ok(entry) = std::fs::read_dir(&deps_dir) {
            for path in entry.flatten() {
                let name = path.file_name().to_string_lossy().to_string();
                if name.starts_with("hub_cli-") && !name.contains(".rlib") && !name.contains(".d") {
                    return path.path().to_string_lossy().to_string();
                }
            }
        }
        // Fallback
        return format!("{}/debug/hub-cli", target_dir);
    }
    // Otherwise use the standard CARGO_BIN_EXE location
    std::env::var("CARGO_BIN_EXE_hub-cli").unwrap_or_else(|_| "../target/debug/hub-cli".to_string())
}

/// Helper to run hub-cli and capture output
async fn run_cli(args: &[&str]) -> (String, String, bool) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let output = tokio::task::spawn_blocking(move || {
        let mut cmd = Command::new(hub_cli_path());
        cmd.args(&args);

        // Set LLVM profile file environment variable for coverage
        if let Ok(_) = std::env::var("CARGO_LLVM_COV_TARGET_DIR") {
            cmd.env("LLVM_PROFILE_FILE", "%p-%m.profraw");
        }

        cmd.output().expect("Failed to execute hub-cli")
    })
    .await
    .expect("spawn_blocking task failed");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

#[tokio::test]
async fn test_cli_agent_list_empty() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3010).await;
    });

    sleep(Duration::from_millis(300)).await;

    let (stdout, stderr, success) =
        run_cli(&["--server", "http://127.0.0.1:3010", "agent", "list"]).await;

    assert!(success, "CLI failed: stderr={}", stderr);
    let agents: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_cli_agent_register_and_list() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3011).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Register agent
    let (stdout, stderr, success) = run_cli(&[
        "--server",
        "http://127.0.0.1:3011",
        "agent",
        "register",
        "test-agent",
        "--name",
        "Test Agent",
        "--endpoint",
        "http://localhost:8000",
        "--capabilities",
        "chat.general,code.review",
    ])
    .await;

    assert!(success, "Register failed: stderr={}", stderr);
    assert!(stdout.contains("registered") || stdout.contains("success"));

    // List agents
    let (stdout, stderr, success) =
        run_cli(&["--server", "http://127.0.0.1:3011", "agent", "list"]).await;

    assert!(success, "List failed: stderr={}", stderr);
    let agents: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["agent_id"], "test-agent");
}

#[tokio::test]
async fn test_cli_agent_remove() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3012).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Register agent first
    let _ = run_cli(&[
        "--server",
        "http://127.0.0.1:3012",
        "agent",
        "register",
        "test-agent",
        "--name",
        "Test Agent",
        "--endpoint",
        "http://localhost:8000",
        "--capabilities",
        "chat.general",
    ])
    .await;

    // Remove agent
    let (stdout, stderr, success) = run_cli(&[
        "--server",
        "http://127.0.0.1:3012",
        "agent",
        "remove",
        "test-agent",
    ])
    .await;

    assert!(success, "Remove failed: stderr={}", stderr);
    assert!(stdout.contains("removed") || stdout.contains("success"));

    // Verify removed
    let (stdout, _, success) =
        run_cli(&["--server", "http://127.0.0.1:3012", "agent", "list"]).await;

    assert!(success);
    let agents: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_cli_task_list_empty() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3013).await;
    });

    sleep(Duration::from_millis(300)).await;

    let (stdout, stderr, success) =
        run_cli(&["--server", "http://127.0.0.1:3013", "task", "list"]).await;

    assert!(success, "CLI failed: stderr={}", stderr);
    let tasks: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tasks.len(), 0);
}

#[tokio::test]
async fn test_cli_task_create() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3014).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Register agent first
    let _ = run_cli(&[
        "--server",
        "http://127.0.0.1:3014",
        "agent",
        "register",
        "test-agent",
        "--name",
        "Test Agent",
        "--endpoint",
        "http://localhost:8000",
        "--capabilities",
        "chat.general",
    ])
    .await;

    // Create task
    let (stdout, stderr, success) = run_cli(&[
        "--server",
        "http://127.0.0.1:3014",
        "task",
        "post",
        "chat.general",
        "{\"message\":\"hello\"}",
    ])
    .await;

    assert!(success, "Create task failed: stderr={}", stderr);

    let task: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(task["intent"], "chat.general");
    assert_eq!(task["agent_id"], "test-agent");
}

#[tokio::test]
async fn test_cli_task_get() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3015).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Register agent
    let _ = run_cli(&[
        "--server",
        "http://127.0.0.1:3015",
        "agent",
        "register",
        "test-agent",
        "--name",
        "Test Agent",
        "--endpoint",
        "http://localhost:8000",
        "--capabilities",
        "test.intent",
    ])
    .await;

    // Create task
    let (stdout, _, _) = run_cli(&[
        "--server",
        "http://127.0.0.1:3015",
        "task",
        "post",
        "test.intent",
        "{\"data\":\"test\"}",
    ])
    .await;

    let task: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let task_id = task["task_id"].as_str().unwrap();

    // Get task
    let (stdout, stderr, success) =
        run_cli(&["--server", "http://127.0.0.1:3015", "task", "get", task_id]).await;

    assert!(success, "Get task failed: stderr={}", stderr);

    let retrieved_task: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(retrieved_task["task_id"], task_id);
}

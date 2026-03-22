// Pull Model poll workflow tests

use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_agent_poll_empty() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3500).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Register agent
    let reqwest_client = reqwest::Client::new();
    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.intent".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    let _ = reqwest_client
        .post("http://127.0.0.1:3500/agents")
        .json(&agent)
        .send()
        .await
        .unwrap();

    // Poll for tasks (empty)
    let response = reqwest_client
        .get("http://127.0.0.1:3500/agents/test-agent/tasks")
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    let tasks: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(tasks.len(), 0);
}

#[tokio::test]
async fn test_agent_poll_with_tasks() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3501).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Register agent
    let reqwest_client = reqwest::Client::new();
    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.intent".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    let _ = reqwest_client
        .post("http://127.0.0.1:3501/agents")
        .json(&agent)
        .send()
        .await
        .unwrap();

    // Create tasks
    let _ = reqwest_client
        .post("http://127.0.0.1:3501/tasks")
        .json(&serde_json::json!({
            "intent": "test.intent",
            "input": {"data": "test1"}
        }))
        .send()
        .await
        .unwrap();

    let _ = reqwest_client
        .post("http://127.0.0.1:3501/tasks")
        .json(&serde_json::json!({
            "intent": "test.intent",
            "input": {"data": "test2"}
        }))
        .send()
        .await
        .unwrap();

    // Poll for tasks
    let response = reqwest_client
        .get("http://127.0.0.1:3501/agents/test-agent/tasks")
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    let tasks: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0]["status"], "accepted");
    assert_eq!(tasks[1]["status"], "accepted");
}

#[tokio::test]
async fn test_poll_updates_last_poll_timestamp() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3502).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Register agent
    let reqwest_client = reqwest::Client::new();
    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.intent".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    let _ = reqwest_client
        .post("http://127.0.0.1:3502/agents")
        .json(&agent)
        .send()
        .await
        .unwrap();

    // Verify last_poll is None initially
    let response = reqwest_client
        .get("http://127.0.0.1:3502/agents")
        .send()
        .await
        .unwrap();

    let agents: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert!(agents[0].get("last_poll").is_none() || agents[0]["last_poll"].is_null());

    // Poll for tasks
    let _ = reqwest_client
        .get("http://127.0.0.1:3502/agents/test-agent/tasks")
        .send()
        .await
        .unwrap();

    // Verify last_poll is now set
    let response = reqwest_client
        .get("http://127.0.0.1:3502/agents")
        .send()
        .await
        .unwrap();

    let agents: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert!(agents[0].get("last_poll").is_some() && !agents[0]["last_poll"].is_null());
}

#[tokio::test]
async fn test_poll_nonexistent_agent() {
    // Start server
    tokio::spawn(async {
        let _ = hub_server::run(3503).await;
    });

    sleep(Duration::from_millis(300)).await;

    // Poll for tasks with nonexistent agent
    let reqwest_client = reqwest::Client::new();
    let response = reqwest_client
        .get("http://127.0.0.1:3503/agents/nonexistent/tasks")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

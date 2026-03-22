// E2E test: Task workflow

use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

async fn start_test_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = hub_server::create_app();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;
    format!("http://{}", addr)
}

#[tokio::test]
async fn task_create_and_get() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Register agent first
    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.cap".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    client
        .post(&format!("{}/agents", server_url))
        .json(&agent)
        .send()
        .await
        .unwrap();

    // Create task
    let task_request = serde_json::json!({
        "intent": "test.cap",
        "input": {"key": "value"}
    });

    let response = client
        .post(&format!("{}/tasks", server_url))
        .json(&task_request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let task: hub_core::Task = response.json().await.unwrap();
    let task_id = task.task_id.clone();

    // Get task
    let response = client
        .get(&format!("{}/tasks/{}", server_url, task_id))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let retrieved: hub_core::Task = response.json().await.unwrap();
    assert_eq!(retrieved.task_id, task_id);
}

#[tokio::test]
async fn task_complete_workflow() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Setup: register agent and create task
    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.cap".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    client
        .post(&format!("{}/agents", server_url))
        .json(&agent)
        .send()
        .await
        .unwrap();

    let response = client
        .post(&format!("{}/tasks", server_url))
        .json(&serde_json::json!({"intent": "test.cap", "input": {}}))
        .send()
        .await
        .unwrap();

    let task: hub_core::Task = response.json().await.unwrap();
    let task_id = task.task_id.clone();

    // Update task result
    let update = serde_json::json!({
        "result": {"response": "Done!"}
    });

    let response = client
        .put(&format!("{}/tasks/{}/result", server_url, task_id))
        .json(&update)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let updated: hub_core::Task = response.json().await.unwrap();
    assert_eq!(format!("{:?}", updated.status), "Completed");
}

#[tokio::test]
async fn task_cancel_workflow() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Setup
    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.cap".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    client
        .post(&format!("{}/agents", server_url))
        .json(&agent)
        .send()
        .await
        .unwrap();

    let response = client
        .post(&format!("{}/tasks", server_url))
        .json(&serde_json::json!({"intent": "test.cap", "input": {}}))
        .send()
        .await
        .unwrap();

    let task: hub_core::Task = response.json().await.unwrap();
    let task_id = task.task_id.clone();

    // Cancel task
    let response = client
        .post(&format!("{}/tasks/{}/cancel", server_url, task_id))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let cancelled: hub_core::Task = response.json().await.unwrap();
    assert_eq!(format!("{:?}", cancelled.status), "Cancelled");
}

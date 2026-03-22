// E2E test: Agent lifecycle

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
async fn agent_register_list_via_http() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    // Register agent
    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.cap".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    let response = client
        .post(format!("{}/agents", server_url))
        .json(&agent)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // List agents
    let response = client
        .get(format!("{}/agents", server_url))
        .send()
        .await
        .unwrap();

    let agents: Vec<hub_core::Agent> = response.json().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].agent_id, "test-agent");
}

#[tokio::test]
async fn agent_register_remove() {
    let server_url = start_test_server().await;
    let client = reqwest::Client::new();

    let agent = hub_core::Agent {
        agent_id: "test-agent".to_string(),
        display_name: "Test Agent".to_string(),
        capabilities: vec!["test.cap".to_string()],
        endpoint: "http://localhost:8000".to_string(),
        last_poll: None,
    };

    client
        .post(format!("{}/agents", server_url))
        .json(&agent)
        .send()
        .await
        .unwrap();

    // Remove agent
    let response = client
        .delete(format!("{}/agents/test-agent", server_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify removed
    let response = client
        .get(format!("{}/agents", server_url))
        .send()
        .await
        .unwrap();

    let agents: Vec<hub_core::Agent> = response.json().await.unwrap();
    assert_eq!(agents.len(), 0);
}

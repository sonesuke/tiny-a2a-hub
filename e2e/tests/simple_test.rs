// Simple E2E test to verify setup

#[tokio::test]
async fn test_hub_cli_binary_exists() {
    use std::path::Path;

    let cli_path = Path::new("../target/debug/hub-cli");
    assert!(
        cli_path.exists(),
        "hub-cli binary not found at {:?}",
        cli_path
    );
}

#[tokio::test]
async fn test_server_starts() {
    use tokio::net::TcpListener;
    use tokio::time::{Duration, sleep};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = hub_server::create_app();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    // Try to connect
    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/agents", addr)).send().await;

    assert!(response.is_ok());
    assert_eq!(response.unwrap().status(), 200);
}

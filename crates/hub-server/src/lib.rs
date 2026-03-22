// Hub HTTP server library

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use hub_core::{Agent, Hub};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub hub: Arc<Hub>,
}

pub fn create_app() -> Router {
    let hub = Hub::new();
    let state = AppState { hub: Arc::new(hub) };

    Router::new()
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/{id}", get(get_task))
        .route("/tasks/{id}/cancel", post(cancel_task))
        .route("/tasks/{id}/result", put(update_task_result))
        .route("/agents", get(list_agents).post(register_agent))
        .route("/agents/{id}", delete(unregister_agent))
        .route("/agents/{id}/tasks", get(poll_agent_tasks))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub intent: String,
    pub input: Value,
    #[serde(rename = "agentId")]
    pub agent_id: Option<String>,
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<Value>, StatusCode> {
    match state
        .hub
        .create_task(req.intent, req.input, req.agent_id)
        .await
    {
        Ok(task) => {
            // In Pull Model, tasks stay in Accepted state until agent polls
            Ok(Json(serde_json::to_value(task).unwrap()))
        }
        Err(e) => {
            tracing::error!("Failed to create task: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

pub async fn list_tasks(State(state): State<AppState>) -> Json<Value> {
    let tasks = state.hub.list_tasks().await;
    Json(serde_json::to_value(tasks).unwrap())
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.hub.get_task(&id).await {
        Some(task) => Ok(Json(serde_json::to_value(task).unwrap())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
pub struct UpdateTaskResultRequest {
    pub result: Option<Value>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

pub async fn update_task_result(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskResultRequest>,
) -> Result<Json<Value>, StatusCode> {
    let error = match (req.error_code, req.error_message) {
        (Some(code), Some(message)) => Some(hub_core::TaskError { code, message }),
        _ => None,
    };

    match state.hub.update_task_result(&id, req.result, error).await {
        Ok(task) => Ok(Json(serde_json::to_value(task).unwrap())),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn cancel_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.hub.cancel_task(&id).await {
        Ok(task) => Ok(Json(serde_json::to_value(task).unwrap())),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn list_agents(State(state): State<AppState>) -> Json<Value> {
    let agents = state.hub.list_agents().await;
    Json(serde_json::to_value(agents).unwrap())
}

pub async fn register_agent(
    State(state): State<AppState>,
    Json(agent): Json<Agent>,
) -> Result<Json<Value>, StatusCode> {
    state.hub.register_agent(agent).await;
    Ok(Json(serde_json::json!({"status": "registered"})))
}

pub async fn unregister_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.hub.unregister_agent(&id).await {
        Some(_) => Ok(Json(serde_json::json!({"status": "unregistered"}))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Poll for tasks assigned to an agent (Pull Model)
pub async fn poll_agent_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.hub.poll_tasks(&id).await {
        Ok(tasks) => Ok(Json(serde_json::to_value(tasks).unwrap())),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Run the server on the specified port
pub async fn run(port: u16) -> anyhow::Result<()> {
    let app = create_app();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("Hub server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

// Hub core logic - task management only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Task error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskError {
    pub code: String,
    pub message: String,
}

/// Task status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Accepted,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task result type
pub type TaskResult = serde_json::Value;

/// Task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub intent: String,
    pub input: serde_json::Value,
    pub agent_id: String,
    pub status: TaskStatus,
    pub result: Option<TaskResult>,
    pub error: Option<TaskError>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl Task {
    pub fn new(intent: String, input: serde_json::Value, agent_id: String) -> Self {
        let now = time::OffsetDateTime::now_utc();
        Self {
            task_id: Uuid::new_v4().to_string(),
            intent,
            input,
            agent_id,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Agent info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub agent_id: String,
    pub display_name: String,
    pub capabilities: Vec<String>,
    pub endpoint: String, // URL where the agent is reachable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_poll: Option<time::OffsetDateTime>, // Last time agent polled for tasks
}

/// In-memory task store
#[derive(Debug, Default)]
struct TaskStore {
    tasks: HashMap<String, Task>,
}

impl TaskStore {
    fn insert(&mut self, task: Task) {
        self.tasks.insert(task.task_id.clone(), task);
    }

    fn get(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    fn get_mut(&mut self, task_id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(task_id)
    }

    fn list(&self) -> Vec<Task> {
        self.tasks.values().cloned().collect()
    }
}

/// Agent registry
#[derive(Debug, Default)]
struct AgentRegistry {
    agents: HashMap<String, Agent>,
}

impl AgentRegistry {
    fn register(&mut self, agent: Agent) {
        self.agents.insert(agent.agent_id.clone(), agent);
    }

    fn unregister(&mut self, agent_id: &str) -> Option<Agent> {
        self.agents.remove(agent_id)
    }

    fn get(&self, agent_id: &str) -> Option<&Agent> {
        self.agents.get(agent_id)
    }

    fn get_mut(&mut self, agent_id: &str) -> Option<&mut Agent> {
        self.agents.get_mut(agent_id)
    }

    fn list(&self) -> Vec<Agent> {
        self.agents.values().cloned().collect()
    }

    fn find_for_intent(&self, intent: &str) -> Option<&Agent> {
        self.agents
            .values()
            .find(|a| a.capabilities.contains(&intent.to_string()))
    }

    /// Update agent's last_poll timestamp
    fn update_poll_time(&mut self, agent_id: &str) -> Result<(), String> {
        let agent = self
            .get_mut(agent_id)
            .ok_or_else(|| format!("Agent not found: {}", agent_id))?;
        agent.last_poll = Some(time::OffsetDateTime::now_utc());
        Ok(())
    }

    /// Remove agents that haven't polled within the timeout
    fn cleanup_stale_agents(&mut self, timeout: time::Duration) -> Vec<String> {
        let now = time::OffsetDateTime::now_utc();
        let mut stale_agents = Vec::new();

        self.agents.retain(|agent_id, agent| {
            let is_stale = match agent.last_poll {
                Some(last_poll) => {
                    let elapsed = now - last_poll;
                    elapsed > timeout
                }
                None => {
                    // Agent has never polled - remove if registered for longer than timeout
                    // For now, keep them (will be cleaned on next poll or by other logic)
                    true
                }
            };

            if is_stale {
                stale_agents.push(agent_id.clone());
            }

            !is_stale
        });

        stale_agents
    }
}

/// Hub core
pub struct Hub {
    task_store: Arc<RwLock<TaskStore>>,
    agents: Arc<RwLock<AgentRegistry>>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            task_store: Arc::new(RwLock::new(TaskStore::default())),
            agents: Arc::new(RwLock::new(AgentRegistry::default())),
        }
    }

    /// Register an agent
    pub async fn register_agent(&self, agent: Agent) {
        let mut registry = self.agents.write().await;
        registry.register(agent);
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &str) -> Option<Agent> {
        let mut registry = self.agents.write().await;
        registry.unregister(agent_id)
    }

    /// Create a new task
    pub async fn create_task(
        &self,
        intent: String,
        input: serde_json::Value,
        agent_id: Option<String>,
    ) -> anyhow::Result<Task> {
        let registry = self.agents.read().await;

        // Determine agent to use
        let target_agent = if let Some(id) = agent_id {
            registry
                .get(&id)
                .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", id))?
        } else {
            registry
                .find_for_intent(&intent)
                .ok_or_else(|| anyhow::anyhow!("No agent found for intent: {}", intent))?
        };

        let mut task = Task::new(intent, input, target_agent.agent_id.clone());
        task.status = TaskStatus::Accepted;

        let mut store = self.task_store.write().await;
        store.insert(task.clone());

        Ok(task)
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        let store = self.task_store.read().await;
        store.get(task_id).cloned()
    }

    /// Update task result
    pub async fn update_task_result(
        &self,
        task_id: &str,
        result: Option<TaskResult>,
        error: Option<TaskError>,
    ) -> anyhow::Result<Task> {
        let mut store = self.task_store.write().await;
        let task = store
            .get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

        task.updated_at = time::OffsetDateTime::now_utc();

        match (&result, &error) {
            (Some(_), None) => task.status = TaskStatus::Completed,
            (None, Some(_)) => task.status = TaskStatus::Failed,
            _ => {}
        }

        task.result = result;
        task.error = error;

        Ok(task.clone())
    }

    /// Execute a task (mark as running - actual execution is done by agents)
    pub async fn execute_task(&self, task_id: &str) -> anyhow::Result<Task> {
        let mut store = self.task_store.write().await;
        let task = store
            .get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

        task.status = TaskStatus::Running;
        task.updated_at = time::OffsetDateTime::now_utc();

        Ok(task.clone())
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> anyhow::Result<Task> {
        let mut store = self.task_store.write().await;
        let task = store
            .get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

        task.status = TaskStatus::Cancelled;
        task.updated_at = time::OffsetDateTime::now_utc();

        Ok(task.clone())
    }

    /// List available agents
    pub async fn list_agents(&self) -> Vec<Agent> {
        let registry = self.agents.read().await;
        registry.list()
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<Task> {
        let store = self.task_store.read().await;
        store.list()
    }

    /// Poll for tasks assigned to a specific agent (Pull Model)
    /// Updates agent's last_poll timestamp and returns pending tasks
    pub async fn poll_tasks(&self, agent_id: &str) -> anyhow::Result<Vec<Task>> {
        // Update agent's last_poll timestamp
        {
            let mut registry = self.agents.write().await;
            registry
                .update_poll_time(agent_id)
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        // Get tasks assigned to this agent that are in Accepted status
        let store = self.task_store.read().await;
        let tasks: Vec<Task> = store
            .list()
            .into_iter()
            .filter(|t| t.agent_id == agent_id && t.status == TaskStatus::Accepted)
            .collect();

        Ok(tasks)
    }

    /// Cleanup stale agents that haven't polled within timeout
    pub async fn cleanup_stale_agents(&self, timeout: time::Duration) -> Vec<String> {
        let mut registry = self.agents.write().await;
        registry.cleanup_stale_agents(timeout)
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export for convenience
pub use client::HubClient;
pub use time;

// HTTP client module
pub mod client;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_hub_new() {
        let hub = Hub::new();
        assert_eq!(hub.list_agents().await.len(), 0);
        assert_eq!(hub.list_tasks().await.len(), 0);
    }

    #[tokio::test]
    async fn test_register_agent() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;
        let agents = hub.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "test-agent");
    }

    #[tokio::test]
    async fn test_unregister_agent() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;
        assert_eq!(hub.list_agents().await.len(), 1);

        let removed = hub.unregister_agent("test-agent").await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().agent_id, "test-agent");
        assert_eq!(hub.list_agents().await.len(), 0);

        let removed_again = hub.unregister_agent("test-agent").await;
        assert!(removed_again.is_none());
    }

    #[tokio::test]
    async fn test_create_task_with_agent_id() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string(), "test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        let task = hub
            .create_task(
                "test.intent".to_string(),
                json!({"key": "value"}),
                Some("test-agent".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(task.intent, "test.intent");
        assert_eq!(task.agent_id, "test-agent");
        assert_eq!(task.status, TaskStatus::Accepted);
    }

    #[tokio::test]
    async fn test_create_task_auto_select_agent() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        let task = hub
            .create_task("test.capability".to_string(), json!({"key": "value"}), None)
            .await
            .unwrap();

        assert_eq!(task.agent_id, "test-agent");
    }

    #[tokio::test]
    async fn test_create_task_agent_not_found() {
        let hub = Hub::new();
        let result = hub
            .create_task(
                "test.intent".to_string(),
                json!({"key": "value"}),
                Some("non-existent".to_string()),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_task() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string(), "test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        let task = hub
            .create_task("test.intent".to_string(), json!({"key": "value"}), None)
            .await
            .unwrap();

        let retrieved = hub.get_task(&task.task_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().task_id, task.task_id);

        let not_found = hub.get_task("non-existent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_task_result() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string(), "test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        let task = hub
            .create_task("test.intent".to_string(), json!({"key": "value"}), None)
            .await
            .unwrap();

        // Update with result
        let updated = hub
            .update_task_result(&task.task_id, Some(json!({"result": "success"})), None)
            .await
            .unwrap();

        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.result, Some(json!({"result": "success"})));

        // Update with error
        let error = TaskError {
            code: "test.error".to_string(),
            message: "Test error message".to_string(),
        };

        let task2 = hub
            .create_task("test.capability".to_string(), json!({}), None)
            .await
            .unwrap();

        let updated = hub
            .update_task_result(&task2.task_id, None, Some(error))
            .await
            .unwrap();

        assert_eq!(updated.status, TaskStatus::Failed);
        assert!(updated.error.is_some());
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string(), "test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        let task = hub
            .create_task("test.intent".to_string(), json!({}), None)
            .await
            .unwrap();

        let cancelled = hub.cancel_task(&task.task_id).await.unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);

        let retrieved = hub.get_task(&task.task_id).await.unwrap();
        assert_eq!(retrieved.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_execute_task() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string(), "test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        let task = hub
            .create_task("test.intent".to_string(), json!({}), None)
            .await
            .unwrap();

        let executed = hub.execute_task(&task.task_id).await.unwrap();
        assert_eq!(executed.status, TaskStatus::Running);
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let hub = Hub::new();
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec![
                "test.capability".to_string(),
                "test.intent1".to_string(),
                "test.intent2".to_string(),
            ],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        hub.create_task("test.intent1".to_string(), json!({}), None)
            .await
            .unwrap();

        hub.create_task("test.intent2".to_string(), json!({}), None)
            .await
            .unwrap();

        let tasks = hub.list_tasks().await;
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_poll_tasks_empty() {
        let hub = Hub::new();

        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        // Poll with no tasks
        let tasks = hub.poll_tasks("test-agent").await.unwrap();
        assert_eq!(tasks.len(), 0);

        // last_poll should be updated
        let agents = hub.list_agents().await;
        assert!(agents[0].last_poll.is_some());
    }

    #[tokio::test]
    async fn test_poll_tasks_with_pending() {
        let hub = Hub::new();

        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        // Create tasks
        hub.create_task("test.intent".to_string(), json!({"data": "test1"}), None)
            .await
            .unwrap();

        hub.create_task("test.intent".to_string(), json!({"data": "test2"}), None)
            .await
            .unwrap();

        // Poll for tasks
        let tasks = hub.poll_tasks("test-agent").await.unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].status, TaskStatus::Accepted);
        assert_eq!(tasks[1].status, TaskStatus::Accepted);
    }

    #[tokio::test]
    async fn test_poll_tasks_only_returns_accepted() {
        let hub = Hub::new();

        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        hub.register_agent(agent).await;

        // Create and complete a task
        let task1 = hub
            .create_task("test.intent".to_string(), json!({}), None)
            .await
            .unwrap();
        hub.update_task_result(&task1.task_id, Some(json!({"done": true})), None)
            .await
            .unwrap();

        // Create another task (still Accepted)
        hub.create_task("test.intent".to_string(), json!({}), None)
            .await
            .unwrap();

        // Poll should only return Accepted tasks
        let tasks = hub.poll_tasks("test-agent").await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Accepted);
    }

    #[tokio::test]
    async fn test_cleanup_stale_agents() {
        let hub = Hub::new();

        let agent1 = Agent {
            agent_id: "active-agent".to_string(),
            display_name: "Active Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: Some(time::OffsetDateTime::now_utc()),
        };

        let agent2 = Agent {
            agent_id: "stale-agent".to_string(),
            display_name: "Stale Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: Some(time::OffsetDateTime::now_utc() - time::Duration::seconds(400)),
        };

        hub.register_agent(agent1).await;
        hub.register_agent(agent2).await;

        // Cleanup agents that haven't polled in 5 minutes
        let timeout = time::Duration::seconds(300);
        let stale_agents = hub.cleanup_stale_agents(timeout).await;

        assert_eq!(stale_agents.len(), 1);
        assert_eq!(stale_agents[0], "stale-agent");

        // Verify only active agent remains
        let agents = hub.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "active-agent");
    }

    #[tokio::test]
    async fn test_poll_nonexistent_agent() {
        let hub = Hub::new();

        let result = hub.poll_tasks("nonexistent").await;
        assert!(result.is_err());
    }
}

// HTTP client tests
// These tests require axum to run a test HTTP server
#[cfg(test)]
mod client_tests {
    use super::*;
    use axum::extract::State;
    use serde_json::json;
    use std::sync::Arc;

    /// Helper to spawn a test server on a given port
    async fn spawn_test_server(port: u16) -> anyhow::Result<tokio::task::JoinHandle<()>> {
        let hub = Hub::new();

        // Build axum app manually to avoid dependency on hub-server
        let app = axum::Router::new()
            .route(
                "/tasks",
                axum::routing::post(create_task_handler).get(list_tasks_handler),
            )
            .route("/tasks/{id}", axum::routing::get(get_task_handler))
            .route(
                "/tasks/{id}/cancel",
                axum::routing::post(cancel_task_handler),
            )
            .route(
                "/agents",
                axum::routing::get(list_agents_handler).post(register_agent_handler),
            )
            .route(
                "/agents/{id}",
                axum::routing::delete(unregister_agent_handler),
            )
            .with_state(Arc::new(hub));

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(handle)
    }

    // HTTP handlers (simplified versions)
    async fn list_agents_handler(
        State(hub): State<Arc<Hub>>,
    ) -> axum::response::Json<serde_json::Value> {
        let agents = hub.list_agents().await;
        axum::response::Json(serde_json::to_value(agents).unwrap())
    }

    async fn register_agent_handler(
        State(hub): State<Arc<Hub>>,
        axum::Json(agent): axum::Json<Agent>,
    ) -> axum::response::Json<serde_json::Value> {
        hub.register_agent(agent).await;
        axum::response::Json(serde_json::json!({"status": "registered"}))
    }

    async fn unregister_agent_handler(
        State(hub): State<Arc<Hub>>,
        axum::extract::Path(id): axum::extract::Path<String>,
    ) -> axum::http::StatusCode {
        hub.unregister_agent(&id)
            .await
            .map_or(axum::http::StatusCode::NOT_FOUND, |_| {
                axum::http::StatusCode::OK
            })
    }

    async fn create_task_handler(
        State(hub): State<Arc<Hub>>,
        axum::Json(req): axum::Json<serde_json::Value>,
    ) -> Result<axum::response::Json<serde_json::Value>, axum::http::StatusCode> {
        let intent = req["intent"]
            .as_str()
            .ok_or(axum::http::StatusCode::BAD_REQUEST)?
            .to_string();
        let input = req
            .get("input")
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default()));
        let agent_id = req
            .get("agentId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let task = hub
            .create_task(intent, input, agent_id)
            .await
            .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

        // Auto-execute
        let _ = hub.execute_task(&task.task_id).await;
        let result = hub.get_task(&task.task_id).await;

        result
            .map(|t| axum::response::Json(serde_json::to_value(t).unwrap()))
            .ok_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    async fn list_tasks_handler(
        State(hub): State<Arc<Hub>>,
    ) -> axum::response::Json<serde_json::Value> {
        let tasks = hub.list_tasks().await;
        axum::response::Json(serde_json::to_value(tasks).unwrap())
    }

    async fn get_task_handler(
        State(hub): State<Arc<Hub>>,
        axum::extract::Path(id): axum::extract::Path<String>,
    ) -> Result<axum::response::Json<serde_json::Value>, axum::http::StatusCode> {
        hub.get_task(&id)
            .await
            .map(|t| axum::response::Json(serde_json::to_value(t).unwrap()))
            .ok_or(axum::http::StatusCode::NOT_FOUND)
    }

    async fn cancel_task_handler(
        State(hub): State<Arc<Hub>>,
        axum::extract::Path(id): axum::extract::Path<String>,
    ) -> Result<axum::response::Json<serde_json::Value>, axum::http::StatusCode> {
        hub.cancel_task(&id)
            .await
            .map(|t| axum::response::Json(serde_json::to_value(t).unwrap()))
            .map_err(|_| axum::http::StatusCode::NOT_FOUND)
    }

    #[tokio::test]
    async fn test_client_list_agents_empty() {
        let _handle = spawn_test_server(3901).await.unwrap();

        let client = HubClient::new("http://127.0.0.1:3901".to_string());
        let agents = client.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_client_list_agents() {
        let _handle = spawn_test_server(3902).await.unwrap();

        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.capability".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        let reqwest_client = reqwest::Client::new();
        let response = reqwest_client
            .post("http://127.0.0.1:3902/agents")
            .json(&agent)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());

        let client = HubClient::new("http://127.0.0.1:3902".to_string());
        let agents = client.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "test-agent");
    }

    #[tokio::test]
    async fn test_client_create_task() {
        let _handle = spawn_test_server(3903).await.unwrap();

        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        let reqwest_client = reqwest::Client::new();
        let response = reqwest_client
            .post("http://127.0.0.1:3903/agents")
            .json(&agent)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());

        let client = HubClient::new("http://127.0.0.1:3903".to_string());
        let task = client
            .create_task("test.intent".to_string(), json!({"key": "value"}), None)
            .await
            .unwrap();

        assert_eq!(task.intent, "test.intent");
        assert_eq!(task.agent_id, "test-agent");
    }

    #[tokio::test]
    async fn test_client_create_task_with_agent_id() {
        let _handle = spawn_test_server(3904).await.unwrap();

        let agent = Agent {
            agent_id: "my-agent".to_string(),
            display_name: "My Agent".to_string(),
            capabilities: vec!["other.capability".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        let reqwest_client = reqwest::Client::new();
        let response = reqwest_client
            .post("http://127.0.0.1:3904/agents")
            .json(&agent)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());

        let client = HubClient::new("http://127.0.0.1:3904".to_string());
        let task = client
            .create_task(
                "any.intent".to_string(),
                json!({"key": "value"}),
                Some("my-agent".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(task.agent_id, "my-agent");
    }

    #[tokio::test]
    async fn test_client_get_task() {
        let _handle = spawn_test_server(3905).await.unwrap();

        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        let reqwest_client = reqwest::Client::new();
        let _ = reqwest_client
            .post("http://127.0.0.1:3905/agents")
            .json(&agent)
            .send()
            .await
            .unwrap();

        let response = reqwest_client
            .post("http://127.0.0.1:3905/tasks")
            .json(&serde_json::json!({
                "intent": "test.intent",
                "input": {"data": "test"}
            }))
            .send()
            .await
            .unwrap();

        let created_task: Task = response.json().await.unwrap();

        let client = HubClient::new("http://127.0.0.1:3905".to_string());
        let retrieved = client.get_task(&created_task.task_id).await.unwrap();

        assert_eq!(retrieved.task_id, created_task.task_id);
        assert_eq!(retrieved.intent, "test.intent");
    }

    #[tokio::test]
    async fn test_client_cancel_task() {
        let _handle = spawn_test_server(3906).await.unwrap();

        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        let reqwest_client = reqwest::Client::new();
        let _ = reqwest_client
            .post("http://127.0.0.1:3906/agents")
            .json(&agent)
            .send()
            .await
            .unwrap();

        let response = reqwest_client
            .post("http://127.0.0.1:3906/tasks")
            .json(&serde_json::json!({
                "intent": "test.intent",
                "input": {}
            }))
            .send()
            .await
            .unwrap();

        let created_task: Task = response.json().await.unwrap();

        let client = HubClient::new("http://127.0.0.1:3906".to_string());
        client.cancel_task(&created_task.task_id).await.unwrap();

        let cancelled = client.get_task(&created_task.task_id).await.unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_client_create_task_no_agent() {
        let _handle = spawn_test_server(3907).await.unwrap();

        let client = HubClient::new("http://127.0.0.1:3907".to_string());
        let result = client
            .create_task("unknown.intent".to_string(), json!({}), None)
            .await;

        assert!(result.is_err());
    }
}

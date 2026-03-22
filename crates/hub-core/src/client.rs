// HTTP client for connecting to remote Hub server

use crate::{Agent, Task};
use reqwest::Client;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HubClientError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// HTTP client for the Hub server
pub struct HubClient {
    client: Client,
    base_url: String,
}

impl HubClient {
    /// Create a new Hub client
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Create a new task
    pub async fn create_task(
        &self,
        intent: String,
        input: serde_json::Value,
        agent_id: Option<String>,
    ) -> Result<Task, HubClientError> {
        #[derive(Serialize)]
        struct CreateTaskRequest {
            intent: String,
            input: serde_json::Value,
            #[serde(rename = "agentId")]
            agent_id: Option<String>,
        }

        let req = CreateTaskRequest {
            intent,
            input,
            agent_id,
        };

        let response = self
            .client
            .post(format!("{}/tasks", self.base_url))
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            let task: Task = response.json().await?;
            Ok(task)
        } else {
            Err(HubClientError::InvalidResponse(format!(
                "Status: {}, Body: {:?}",
                response.status(),
                response.text().await
            )))
        }
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Result<Task, HubClientError> {
        let response = self
            .client
            .get(format!("{}/tasks/{}", self.base_url, task_id))
            .send()
            .await?;

        if response.status().is_success() {
            let task: Task = response.json().await?;
            Ok(task)
        } else if response.status().as_u16() == 404 {
            Err(HubClientError::TaskNotFound(task_id.to_string()))
        } else {
            Err(HubClientError::InvalidResponse(format!(
                "Status: {}",
                response.status()
            )))
        }
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<Task, HubClientError> {
        let response = self
            .client
            .post(format!("{}/tasks/{}/cancel", self.base_url, task_id))
            .send()
            .await?;

        if response.status().is_success() {
            let task: Task = response.json().await?;
            Ok(task)
        } else if response.status().as_u16() == 404 {
            Err(HubClientError::TaskNotFound(task_id.to_string()))
        } else {
            Err(HubClientError::InvalidResponse(format!(
                "Status: {}",
                response.status()
            )))
        }
    }

    /// List available agents
    pub async fn list_agents(&self) -> Result<Vec<Agent>, HubClientError> {
        let response = self
            .client
            .get(format!("{}/agents", self.base_url))
            .send()
            .await?;

        if response.status().is_success() {
            let agents: Vec<Agent> = response.json().await?;
            Ok(agents)
        } else {
            Err(HubClientError::InvalidResponse(format!(
                "Status: {}",
                response.status()
            )))
        }
    }
}

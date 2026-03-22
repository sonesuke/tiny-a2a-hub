// Hub CLI

use clap::{Parser, Subcommand};
use hub_core::{Agent, HubClient};
use rmcp::{
    ServiceExt,
    handler::server::ServerHandler,
    model::{CallToolResult, Content, Implementation, ServerInfo, Tool},
    transport::stdio,
};
use std::sync::Arc;
use tracing_subscriber::prelude::*;

#[derive(Parser, Debug)]
#[command(name = "hub-cli")]
#[command(about = "CLI for tiny-a2a-hub", long_about = None)]
struct Cli {
    /// Hub server URL
    #[arg(short, long, default_value = "http://127.0.0.1:3000")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run as MCP server
    Mcp,

    /// Task operations
    #[command(subcommand)]
    Task(TaskCommands),

    /// Agent operations
    #[command(subcommand)]
    Agent(AgentCommands),
}

#[derive(Subcommand, Debug)]
enum TaskCommands {
    /// List all tasks
    List,

    /// Get a task by ID
    Get {
        /// Task ID
        id: String,
    },

    /// Create a new task
    Post {
        /// Task intent (e.g., chat.general, code.review)
        intent: String,
        /// Task input as JSON string
        input: String,
        /// Agent ID (optional)
        #[arg(short, long)]
        agent: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AgentCommands {
    /// List all agents
    List,

    /// Register a new agent
    Register {
        /// Agent ID
        id: String,
        /// Display name
        #[arg(short, long)]
        name: String,
        /// Agent endpoint URL
        #[arg(short, long)]
        endpoint: String,
        /// Capabilities (comma-separated)
        #[arg(short, long)]
        capabilities: String,
    },

    /// Remove an agent
    Remove {
        /// Agent ID
        id: String,
    },
}

struct McpServer {
    client: Arc<HubClient>,
}

impl McpServer {
    fn new(client: Arc<HubClient>) -> Self {
        Self { client }
    }

    fn list_all_tools() -> Vec<Tool> {
        vec![
            Tool::new(
                "create_task",
                "Create a new task in the hub",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "intent": {
                            "type": "string",
                            "description": "The intent of the task (e.g., code.review, chat.general)"
                        },
                        "input": {
                            "type": "object",
                            "description": "Input data for the task"
                        },
                        "agentId": {
                            "type": "string",
                            "description": "Optional agent ID to route the task to"
                        }
                    },
                    "required": ["intent", "input"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            Tool::new(
                "get_task",
                "Get the status and result of a task",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "taskId": {
                            "type": "string",
                            "description": "The ID of the task to retrieve"
                        }
                    },
                    "required": ["taskId"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            Tool::new(
                "cancel_task",
                "Cancel a running task",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "taskId": {
                            "type": "string",
                            "description": "The ID of the task to cancel"
                        }
                    },
                    "required": ["taskId"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            Tool::new(
                "list_agents",
                "List all available agents and their capabilities",
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        ]
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(Default::default())
            .with_server_info(Implementation::new("hub-cli", "0.1.0"))
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListToolsResult, rmcp::ErrorData>>
    + Send
    + '_ {
        std::future::ready(Ok(rmcp::model::ListToolsResult {
            tools: Self::list_all_tools(),
            ..Default::default()
        }))
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        let client = self.client.clone();
        let tool_name = request.name.to_string();
        let arguments = request.arguments.clone();

        async move {
            match tool_name.as_str() {
                "create_task" => {
                    let intent = arguments
                        .as_ref()
                        .and_then(|args| args.get("intent"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| rmcp::ErrorData::invalid_params("Missing intent", None))?
                        .to_string();

                    let input = arguments
                        .as_ref()
                        .and_then(|args| args.get("input"))
                        .cloned()
                        .ok_or_else(|| rmcp::ErrorData::invalid_params("Missing input", None))?;

                    let agent_id = arguments
                        .as_ref()
                        .and_then(|args| args.get("agentId"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let task = client
                        .create_task(intent, input, agent_id)
                        .await
                        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "{{\"taskId\":\"{}\",\"status\":\"{:?}\"}}",
                        task.task_id, task.status
                    ))]))
                }
                "get_task" => {
                    let task_id = arguments
                        .as_ref()
                        .and_then(|args| args.get("taskId"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| rmcp::ErrorData::invalid_params("Missing taskId", None))?;

                    let task = client
                        .get_task(task_id)
                        .await
                        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&task)
                            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?,
                    )]))
                }
                "cancel_task" => {
                    let task_id = arguments
                        .as_ref()
                        .and_then(|args| args.get("taskId"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| rmcp::ErrorData::invalid_params("Missing taskId", None))?;

                    client
                        .cancel_task(task_id)
                        .await
                        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "{{\"cancelled\":\"{}\"}}",
                        task_id
                    ))]))
                }
                "list_agents" => {
                    let agents = client
                        .list_agents()
                        .await
                        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&agents)
                            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?,
                    )]))
                }
                _ => Err(rmcp::ErrorData::method_not_found::<
                    rmcp::model::CallToolRequestMethod,
                >()),
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tiny_a2a_hub=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let client = HubClient::new(cli.server.clone());

    match cli.command {
        Commands::Mcp => {
            tracing::info!("Connecting to Hub server at: {}", cli.server);
            let client = Arc::new(client);
            let server = McpServer::new(client);

            tracing::info!("MCP server starting on stdin/stdout");
            server.serve(stdio()).await?.waiting().await?;
        }
        Commands::Task(TaskCommands::List) => {
            let reqwest_client = reqwest::Client::new();
            let response = reqwest_client
                .get(format!("{}/tasks", cli.server))
                .send()
                .await?;

            if response.status().is_success() {
                let tasks: Vec<hub_core::Task> = response.json().await?;
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            } else {
                eprintln!("Error: {}", response.status());
            }
        }
        Commands::Task(TaskCommands::Get { id }) => {
            let task = client.get_task(&id).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        Commands::Task(TaskCommands::Post {
            intent,
            input,
            agent,
        }) => {
            let input_value: serde_json::Value = serde_json::from_str(&input)?;
            let task = client.create_task(intent, input_value, agent).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        Commands::Agent(AgentCommands::List) => {
            let agents = client.list_agents().await?;
            println!("{}", serde_json::to_string_pretty(&agents)?);
        }
        Commands::Agent(AgentCommands::Register {
            id,
            name,
            endpoint,
            capabilities,
        }) => {
            let agent = Agent {
                agent_id: id,
                display_name: name,
                endpoint,
                capabilities: capabilities
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                last_poll: None,
            };

            let reqwest_client = reqwest::Client::new();
            let response = reqwest_client
                .post(format!("{}/agents", cli.server))
                .json(&agent)
                .send()
                .await?;

            if response.status().is_success() {
                println!("Agent registered successfully");
            } else {
                eprintln!("Error: {}", response.status());
            }
        }
        Commands::Agent(AgentCommands::Remove { id }) => {
            let reqwest_client = reqwest::Client::new();
            let response = reqwest_client
                .delete(format!("{}/agents/{}", cli.server, id))
                .send()
                .await?;

            if response.status().is_success() {
                println!("Agent removed successfully");
            } else {
                eprintln!("Error: {}", response.status());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_core::Agent;
    use std::sync::Arc;

    /// Helper to spawn a test server
    async fn spawn_test_server(port: u16) -> tokio::task::JoinHandle<()> {
        let hub = hub_core::Hub::new();
        let state = hub_server::AppState { hub: Arc::new(hub) };

        let app = axum::Router::new()
            .route(
                "/tasks",
                axum::routing::post(hub_server::create_task).get(hub_server::list_tasks),
            )
            .route("/tasks/{id}", axum::routing::get(hub_server::get_task))
            .route(
                "/tasks/{id}/cancel",
                axum::routing::post(hub_server::cancel_task),
            )
            .route(
                "/agents",
                axum::routing::get(hub_server::list_agents).post(hub_server::register_agent),
            )
            .route(
                "/agents/{id}",
                axum::routing::delete(hub_server::unregister_agent),
            )
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        handle
    }

    #[tokio::test]
    async fn test_hubclient_list_agents() {
        let _handle = spawn_test_server(4001).await;

        let client = HubClient::new("http://127.0.0.1:4001".to_string());
        let agents = client.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_hubclient_register_and_create_task() {
        let _handle = spawn_test_server(4002).await;

        // Register agent first
        let agent = Agent {
            agent_id: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            capabilities: vec!["test.intent".to_string()],
            endpoint: "http://localhost:8000".to_string(),
            last_poll: None,
        };

        let reqwest_client = reqwest::Client::new();
        let response = reqwest_client
            .post("http://127.0.0.1:4002/agents")
            .json(&agent)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());

        let client = HubClient::new("http://127.0.0.1:4002".to_string());
        let task = client
            .create_task(
                "test.intent".to_string(),
                serde_json::json!({"key": "value"}),
                None,
            )
            .await
            .unwrap();

        assert_eq!(task.intent, "test.intent");
        assert_eq!(task.agent_id, "test-agent");
    }
}

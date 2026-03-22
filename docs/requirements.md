# tiny A2A Hub Requirements Specification

**Version:** v0.2
**Date:** 2026-03-22

---

## 1. Overview

tiny A2A Hub is an agent-to-agent (A2A) task hub for local environments. It provides a central task management system where agents can register, receive tasks, and update results.

**Architecture:**
- **Hub (hub-server)**: Central HTTP server for task management
- **Agents**: Independent processes that connect to the Hub
- **hub-cli**: Agent SDK / CLI tool for Hub communication (includes MCP server)

---

## 2. Use Cases (UC)

| ID | Use Case | Description | Priority |
|----|-----------|-------------|----------|
| UC-01 | Agent registers with Hub | Agent announces its capabilities to the Hub | P0 |
| UC-02 | Client creates task | Client creates task via Hub API (intent + input) | P0 |
| UC-03 | Agent polls for tasks | Agent retrieves assigned tasks via Hub API | P0 |
| UC-04 | Agent updates task result | Agent reports completion or error via Hub API | P0 |
| UC-05 | Client checks task status | Client queries task progress | P0 |
| UC-06 | Client cancels task | Client cancels a running task | P1 |
| UC-07 | MCP server integration | Claude Code interacts via MCP (hub-cli) | P0 |

---

## 3. Functional Requirements (FR)

### 3.1 Hub Server (hub-server)

| ID | Requirement | Description | Priority | Phase |
|----|-------------|-------------|----------|-------|
| FR-01 | Server startup | HTTP server starts on configurable port (default: 3000) | P0 | 1 |
| FR-02 | Agent registration | Agents can register with ID, name, capabilities, endpoint | P0 | 1 |
| FR-03 | Agent listing | Clients can list registered agents | P0 | 1 |
| FR-04 | Agent removal | Agents can be removed from the Hub | P1 | 1 |
| FR-05 | Task creation | Clients create tasks with intent, input, optional agent_id | P0 | 1 |
| FR-06 | Task routing | Hub auto-selects agent by intent if agent_id not specified | P0 | 1 |
| FR-07 | Task state management | Hub maintains task states: accepted, running, completed, failed, cancelled | P0 | 1 |
| FR-08 | Task retrieval | Clients can retrieve task by task_id | P0 | 1 |
| FR-09 | Task listing | Clients can list all tasks | P1 | 1 |
| FR-10 | Task result update | Agents can update task result (success or error) | P0 | 1 |
| FR-11 | Task cancellation | Clients can cancel running tasks | P1 | 1 |
| FR-12 | In-memory storage | Tasks stored in memory (extensible to DB later) | P0 | 1 |
| FR-13 | Agent task poll | Agents can poll for their assigned tasks via GET /agents/{id}/tasks | P0 | 1 |
| FR-14 | Agent heartbeat | Agent tracks last_poll timestamp on task poll | P0 | 1 |
| FR-15 | Stale agent cleanup | Hub removes agents that haven't polled within timeout (default: 5min) | P1 | 1 |
| FR-16 | Task timeout | Tasks not polled within timeout are marked failed/abandoned | P1 | 1 |

### 3.2 Hub CLI (hub-cli)

| ID | Requirement | Description | Priority | Phase |
|----|-------------|-------------|----------|-------|
| FR-13 | MCP server mode | Runs as MCP server for Claude Code integration | P0 | 1 |
| FR-14 | CLI commands | task list/get/post, agent list/register/remove | P0 | 1 |
| FR-15 | HTTP client | Communicates with Hub server via HTTP | P0 | 1 |
| FR-16 | Configurable server URL | Hub server URL configurable via --server flag | P0 | 1 |

---

## 4. Non-Functional Requirements (NFR)

| ID | Requirement | Description | Priority |
|----|-------------|-------------|----------|
| NFR-01 | Language | Implementation language is Rust | P0 |
| NFR-02 | Environment | Runs in local development environment | P0 |
| NFR-03 | No authentication | Localhost-only access, no auth/authz | P0 |
| NFR-04 | Modularity | Separate hub-core, hub-server, hub-cli | P0 |
| NFR-05 | Extensibility | Easy to add persistence, streaming, discovery | P0 |
| NFR-06 | Observability | Structured logging with task_id correlation | P1 |
| NFR-07 | Small footprint | Minimal dependencies, understandable codebase | P0 |

---

## 5. Interface Requirements

### 5.1 Hub HTTP API

| Method | Path | Description | Priority |
|--------|------|-------------|----------|
| GET | /tasks | List all tasks | P1 |
| POST | /tasks | Create task | P0 |
| GET | /tasks/{id} | Get task | P0 |
| POST | /tasks/{id}/cancel | Cancel task | P1 |
| PUT | /tasks/{id}/result | Update task result | P0 |
| GET | /agents | List agents | P0 |
| POST | /agents | Register agent | P0 |
| DELETE | /agents/{id} | Remove agent | P1 |
| GET | /agents/{id}/tasks | Poll for agent's assigned tasks | P0 |

### 5.2 MCP Tools (via hub-cli)

| Tool Name | Description | Priority |
|-----------|-------------|----------|
| create_task | Create task | P0 |
| get_task | Get task status | P0 |
| cancel_task | Cancel task | P1 |
| list_agents | List agent capabilities | P0 |

### 5.3 CLI Commands

| Command | Description | Priority |
|---------|-------------|----------|
| hub-cli mcp | Run as MCP server | P0 |
| hub-cli task list | List tasks | P0 |
| hub-cli task get <id> | Get task | P0 |
| hub-cli task post | Create task | P0 |
| hub-cli agent list | List agents | P0 |
| hub-cli agent register | Register agent | P0 |
| hub-cli agent remove <id> | Remove agent | P0 |

---

## 6. Data Models

### 6.1 Task

| Field | Type | Description | Required |
|-------|------|-------------|----------|
| task_id | string | Unique task identifier (UUID) | Yes |
| status | enum | Pending, Accepted, Running, Completed, Failed, Cancelled | Yes |
| intent | string | Task purpose/capability (e.g., "chat.general") | Yes |
| input | object | Task input data | Yes |
| result | object \| null | Execution result | - |
| error | object \| null | Error with code/message | - |
| agent_id | string | Assigned agent ID | Yes |
| created_at | timestamp | Creation time | Yes |
| updated_at | timestamp | Last update time | Yes |

### 6.2 Agent

| Field | Type | Description | Required |
|-------|------|-------------|----------|
| agent_id | string | Unique agent identifier | Yes |
| display_name | string | Human-readable name | Yes |
| capabilities | array<string> | List of supported intents/capabilities | Yes |
| endpoint | string | Agent's HTTP endpoint (for future use) | Yes |
| last_poll | timestamp | Last time agent polled for tasks | - |

---

## 7. Acceptance Criteria (AC)

| ID | Criterion | Priority |
|----|-----------|----------|
| AC-01 | Agents can register with Hub and appear in agent list | P0 |
| AC-02 | Clients can create tasks with auto-selected agent by intent | P0 |
| AC-03 | Agents can update task results (success/error) | P0 |
| AC-04 | Hub maintains task state transitions correctly | P0 |
| AC-05 | Claude Code can create tasks via MCP (hub-cli mcp) | P0 |
| AC-06 | CLI commands work for all Hub operations | P0 |
| AC-07 | Agents can poll for their assigned tasks via GET /agents/{id}/tasks | P0 |
| AC-08 | Hub tracks last_poll timestamp and cleans up stale agents | P1 |
| AC-09 | Tasks not polled within timeout are marked failed | P1 |

---

## 8. Architecture

### 8.1 Module Structure

```
tiny-a2a-hub/
├── crates/
│   ├── hub-core/      # Core logic (Hub, Task, Agent models)
│   ├── hub-server/    # HTTP server (Hub API)
│   └── hub-cli/       # CLI tool + MCP server + HTTP client
└── e2e/              # End-to-end tests
```

### 8.2 Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                hub-server (Hub)                      │
│              HTTP: http://localhost:3000           │
│  ┌───────────────────────────────────────────────┐  │
│  │  Hub Core (task store, agent registry)      │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
         ▲
         │ HTTP (task operations, agent management)
         │
┌─────────────────────────────────────────────────────┐
│              Agent Process                          │
│  ┌──────────────────────────────────────────────┐  │
│  │  Agent Core (Business Logic)                 │  │
│  │                                              │  │
│  │   hub-cli mcp -s http://hub:3000           │  │
│  └──────────────────────────────────────────────┘  │
│         ▲ stdio (MCP protocol)                      │
│  ┌─────┴──────────────────────────────────────┐    │
│  │     hub-cli (MCP Server)                    │    │
│  └────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
         ▲
         │ Claude Desktop / MCP Client
         │
┌─────────────────────────────────────────────────────┐
│              Claude Desktop                        │
└─────────────────────────────────────────────────────┘
```

### 8.3 Agent Workflow

1. **Agent Registration**: Agent calls `POST /agents` to register
2. **Task Creation**: Client creates task via `POST /tasks` (with intent)
3. **Task Assignment**: Hub assigns task to agent based on intent/capabilities
4. **Task Polling**: Agent polls for assigned tasks via `GET /tasks`
5. **Result Update**: Agent updates result via `PUT /tasks/{id}/result`

---

## 9. Implementation Phases

### Phase 1: Core (Current)

- Hub server with in-memory storage
- HTTP API for tasks and agents
- hub-cli with MCP server mode
- Basic CLI commands

### Phase 2: Agent Integration

- **Pull Model task polling** (Agent polls for assigned tasks)
- **Agent heartbeat tracking** (last_poll timestamp, stale agent cleanup)
- **Task timeout** (abandon tasks not processed within timeout)
- Task queue per agent
- Streaming/partial results
- SQLite persistence

### Phase 3: Production Features

- Authentication
- Agent discovery
- Monitoring & metrics
- Task scheduling & priorities

---

## 10. Out of Scope

- Authentication / Authorization (Phase 3+)
- Distributed deployment
- High availability
- Persistent storage (Phase 2+)
- WebSocket/SSE (Phase 2+)

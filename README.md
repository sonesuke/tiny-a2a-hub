# tiny A2A Hub

A minimal agent-to-agent (A2A) task hub for local environments.

**Status: Early Development**

## Overview

tiny A2A Hub is a lightweight, local-first agent orchestration hub. Independent agents register with the Hub, receive tasks via polling (Pull Model), and report results back.

## Architecture

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
│  Polls for tasks → Processes → Updates result      │
└─────────────────────────────────────────────────────┘
```

## Features

- **Pull Model**: Agents poll for assigned tasks
- **HTTP API**: Simple REST interface for task management
- **Agent Registry**: Capability-based agent discovery
- **Task State Management**: Pending, Accepted, Running, Completed, Failed, Cancelled
- **CLI Tool**: `hub-cli` for operations and MCP server mode
- **No Authentication**: Localhost-only access

## Quick Start

```bash
# Build the project
cargo build --workspace

# Start the hub server
cargo run -p hub-server

# In another terminal, register an agent
hub-cli agent register my-agent --name "My Agent" \
  --endpoint http://localhost:8000 \
  --capabilities "chat.general,code.review"

# Create a task
hub-cli task post chat.general '{"message": "hello"}'

# Check task status
hub-cli task get <task_id>

# Poll for tasks (agent-side)
curl http://127.0.0.1:3000/agents/my-agent/tasks
```

## API Endpoints

### Tasks

| Method | Path | Description |
|--------|------|-------------|
| POST | /tasks | Create a task |
| GET | /tasks | List all tasks |
| GET | /tasks/{id} | Get task status |
| POST | /tasks/{id}/cancel | Cancel a task |
| PUT | /tasks/{id}/result | Update task result |

### Agents

| Method | Path | Description |
|--------|------|-------------|
| GET | /agents | List all agents |
| POST | /agents | Register an agent |
| DELETE | /agents/{id} | Remove an agent |
| GET | /agents/{id}/tasks | Poll for agent's tasks (Pull Model) |

## CLI Commands

```bash
# Hub server
mise run serve    # or: cargo run -p hub-server

# CLI operations
hub-cli agent list
hub-cli agent register <id> --name <name> --endpoint <url> --capabilities <caps>
hub-cli agent remove <id>

hub-cli task list
hub-cli task get <id>
hub-cli task post <intent> <input-json> [--agent <id>]

# MCP server mode (for Claude Desktop)
hub-cli mcp --server http://localhost:3000
```

## Development

```bash
# Run tests
cargo test --workspace

# Run with coverage
cargo llvm-cov --workspace

# Format code
cargo fmt

# Run linter
cargo clippy --workspace -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup.

## Documentation

- [Requirements](docs/requirements.md) - Detailed requirements specification
- [Contributing](CONTRIBUTING.md) - Development guide

## License

MIT

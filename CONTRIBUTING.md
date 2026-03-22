# Contributing to tiny A2A Hub

Thank you for your interest in contributing!

## Development Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/tiny-a2a-hub.git
cd tiny-a2a-hub

# Check that the workspace compiles
cargo check --workspace

# Run tests
cargo test --workspace

# Run hub server
cargo run -p hub-server

# Run MCP server
cargo run -p mcp-server
```

## Code Style

- Use English for all code, documentation, and comments
- Follow Rust naming conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` to catch common issues

## Project Structure

```
tiny-a2a-hub/
├── crates/
│   ├── shared-types/     # Shared data models
│   ├── agent-adapter/    # Adapter trait interface
│   ├── adapter-claude/   # Claude-family adapter
│   ├── hub-core/         # Hub business logic
│   ├── hub-server/       # HTTP server
│   └── mcp-server/       # MCP server
├── docs/                 # Documentation
└── AGENTS.md            # Language policy
```

## Submitting Changes

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and formatting
5. Submit a pull request

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

# xds-mcp

A Rust-based MCP (Model Context Protocol) server for the nebucloud-xds project.

This server provides tools for understanding and navigating the xDS control plane codebase, making it easier for AI coding assistants to work with the project.

## Features

| Tool | Description |
|------|-------------|
| `get_workspace_info` | Get overview of the nebucloud-xds workspace |
| `get_crate_structure` | Get module hierarchy for a specific crate |
| `get_xds_resources` | List all xDS resource types and handlers |
| `get_service_endpoints` | Get gRPC service definitions and methods |
| `analyze_dependencies` | Show crate dependency graph |
| `get_milestone_progress` | Track M1-M5 implementation progress |
| `search_code` | Search for patterns in the codebase |
| `get_public_api` | Get public API surface of a crate |
| `run_cargo` | Execute cargo commands (build, test, check, clippy) |
| `get_github_issues` | Get GitHub issues related to milestones |

## Installation

The MCP server is included in the nebucloud-xds workspace. Build it with:

```bash
cargo build -p xds-mcp --release
```

## Usage

### Command Line

```bash
# Run with default workspace (current directory)
cargo run -p xds-mcp

# Run with specific workspace path
cargo run -p xds-mcp -- --workspace /path/to/nebucloud-xds

# Enable debug logging
cargo run -p xds-mcp -- --debug
```

### VS Code Integration

The project includes a `.vscode/mcp.json` configuration. To use it:

1. Make sure you have VS Code with GitHub Copilot installed
2. Open the nebucloud-xds workspace
3. The MCP server will be automatically available to Copilot

### Manual MCP Testing

You can test the server manually by piping JSON-RPC requests:

```bash
# Initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  cargo run -p xds-mcp --quiet -- --workspace .

# List tools
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | \
  cargo run -p xds-mcp --quiet -- --workspace .

# Call a tool
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_workspace_info","arguments":{}}}' | \
  cargo run -p xds-mcp --quiet -- --workspace .

# Get crate structure
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_crate_structure","arguments":{"crate_name":"xds-server"}}}' | \
  cargo run -p xds-mcp --quiet -- --workspace .
```

## Tool Examples

### Get Workspace Info

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_workspace_info",
    "arguments": {}
  }
}
```

### Get Crate Structure

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "get_crate_structure",
    "arguments": {
      "crate_name": "xds-server"
    }
  }
}
```

### Search Code

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "search_code",
    "arguments": {
      "pattern": "impl.*Discovery",
      "file_type": "rs"
    }
  }
}
```

### Analyze Dependencies

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "analyze_dependencies",
    "arguments": {
      "crate_name": "xds-server"
    }
  }
}
```

### Run Cargo Commands

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "run_cargo",
    "arguments": {
      "command": "test",
      "package": "xds-core"
    }
  }
}
```

## Architecture

```
xds-mcp/
├── src/
│   ├── main.rs          # Entry point and MCP server loop
│   ├── protocol.rs      # JSON-RPC types (request, response, error)
│   └── tools/
│       ├── mod.rs       # ToolRegistry and tool definitions
│       ├── analyzer.rs  # Dependency analysis
│       ├── cargo.rs     # Cargo command runner
│       ├── crate_structure.rs  # Crate and module analysis
│       ├── milestones.rs       # Milestone tracking
│       └── search.rs    # Code search
```

## Protocol

The server implements the MCP specification (protocol version `2024-11-05`):

- **Transport**: stdio (stdin/stdout)
- **Capabilities**: Tools (no resources or prompts currently)
- **Methods**: `initialize`, `initialized`, `tools/list`, `tools/call`, `ping`

## License

Apache-2.0

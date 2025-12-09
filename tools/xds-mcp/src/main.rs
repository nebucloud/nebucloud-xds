//! # xds-mcp
//!
//! MCP (Model Context Protocol) server for the nebucloud-xds project.
//!
//! This server provides tools for understanding and navigating the xDS control plane codebase:
//!
//! - `get_crate_structure` - Get module hierarchy for any crate
//! - `get_xds_resources` - List xDS resource types and handlers
//! - `get_service_endpoints` - Get gRPC service definitions
//! - `analyze_dependencies` - Show crate dependency graph
//! - `get_milestone_progress` - Track implementation progress
//! - `search_code` - Search for patterns in the codebase
//! - `run_cargo` - Execute cargo commands

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing::{debug, error, info};

mod protocol;
mod tools;

use protocol::{JsonRpcRequest, JsonRpcResponse, McpError};
use tools::ToolRegistry;

/// MCP Server for nebucloud-xds project understanding
#[derive(Parser, Debug)]
#[command(name = "xds-mcp")]
#[command(about = "MCP server for nebucloud-xds project", long_about = None)]
struct Args {
    /// Path to the nebucloud-xds workspace root
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing (to stderr so it doesn't interfere with MCP stdio)
    let level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(level)
        .with_writer(io::stderr)
        .init();

    info!("Starting xds-mcp server");
    debug!("Workspace: {:?}", args.workspace);

    let workspace = args.workspace.canonicalize().unwrap_or(args.workspace);
    let registry = ToolRegistry::new(workspace);

    // Run the MCP server loop
    run_mcp_server(registry)
}

fn run_mcp_server(registry: ToolRegistry) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                error!("Error reading stdin: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        debug!("Received: {}", line);

        let response = handle_request(&line, &registry);
        let response_json = serde_json::to_string(&response)?;

        debug!("Sending: {}", response_json);
        writeln!(stdout, "{}", response_json)?;
        stdout.flush()?;
    }

    info!("MCP server shutting down");
    Ok(())
}

fn handle_request(input: &str, registry: &ToolRegistry) -> JsonRpcResponse {
    // Parse the JSON-RPC request
    let request: JsonRpcRequest = match serde_json::from_str(input) {
        Ok(req) => req,
        Err(e) => {
            return JsonRpcResponse::error(
                serde_json::Value::Null,
                McpError::parse_error(format!("Invalid JSON: {}", e)),
            );
        }
    };

    let id = request.id.clone();

    // Handle the request based on method
    match request.method.as_str() {
        "initialize" => handle_initialize(id, &request),
        "initialized" => JsonRpcResponse::success(id, serde_json::json!({})),
        "tools/list" => handle_tools_list(id, registry),
        "tools/call" => handle_tools_call(id, &request, registry),
        "resources/list" => handle_resources_list(id),
        "prompts/list" => handle_prompts_list(id),
        "ping" => JsonRpcResponse::success(id, serde_json::json!({})),
        _ => JsonRpcResponse::error(
            id,
            McpError::method_not_found(format!("Unknown method: {}", request.method)),
        ),
    }
}

fn handle_initialize(id: serde_json::Value, _request: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": false
                },
                "resources": {
                    "subscribe": false,
                    "listChanged": false
                },
                "prompts": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "xds-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tools_list(id: serde_json::Value, registry: &ToolRegistry) -> JsonRpcResponse {
    let tools = registry.list_tools();
    JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
}

fn handle_tools_call(
    id: serde_json::Value,
    request: &JsonRpcRequest,
    registry: &ToolRegistry,
) -> JsonRpcResponse {
    let params = match &request.params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::error(
                id,
                McpError::invalid_params("Missing params for tools/call"),
            );
        }
    };

    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => {
            return JsonRpcResponse::error(
                id,
                McpError::invalid_params("Missing 'name' in params"),
            );
        }
    };

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    match registry.call_tool(tool_name, arguments) {
        Ok(result) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": result
                }]
            }),
        ),
        Err(e) => JsonRpcResponse::error(id, McpError::internal_error(e.to_string())),
    }
}

fn handle_resources_list(id: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse::success(id, serde_json::json!({ "resources": [] }))
}

fn handle_prompts_list(id: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse::success(id, serde_json::json!({ "prompts": [] }))
}

//! MCP Tools for nebucloud-xds project understanding.

mod analyzer;
mod cargo;
mod crate_structure;
mod milestones;
mod search;

use std::path::PathBuf;

use anyhow::Result;
use serde_json::json;

use crate::protocol::ToolDefinition;

pub use analyzer::DependencyAnalyzer;
pub use cargo::CargoRunner;
pub use crate_structure::CrateAnalyzer;
pub use milestones::MilestoneTracker;
pub use search::CodeSearcher;

/// Registry of available tools.
pub struct ToolRegistry {
    #[allow(dead_code)]
    workspace: PathBuf,
    crate_analyzer: CrateAnalyzer,
    dependency_analyzer: DependencyAnalyzer,
    milestone_tracker: MilestoneTracker,
    code_searcher: CodeSearcher,
    cargo_runner: CargoRunner,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            crate_analyzer: CrateAnalyzer::new(workspace.clone()),
            dependency_analyzer: DependencyAnalyzer::new(workspace.clone()),
            milestone_tracker: MilestoneTracker::new(workspace.clone()),
            code_searcher: CodeSearcher::new(workspace.clone()),
            cargo_runner: CargoRunner::new(workspace.clone()),
            workspace,
        }
    }

    /// List all available tools.
    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::new(
                "get_workspace_info",
                "Get overview of the nebucloud-xds workspace including all crates and their purposes",
                json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            ToolDefinition::new(
                "get_crate_structure",
                "Get the module hierarchy and structure of a specific crate",
                json!({
                    "type": "object",
                    "properties": {
                        "crate_name": {
                            "type": "string",
                            "description": "Name of the crate (e.g., 'xds-core', 'xds-server')"
                        }
                    },
                    "required": ["crate_name"]
                }),
            ),
            ToolDefinition::new(
                "get_xds_resources",
                "List all xDS resource types defined in the codebase with their handlers",
                json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            ToolDefinition::new(
                "get_service_endpoints",
                "Get all gRPC service definitions and their methods",
                json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "Optional: specific service to query (e.g., 'ADS', 'CDS', 'LDS')"
                        }
                    },
                    "required": []
                }),
            ),
            ToolDefinition::new(
                "analyze_dependencies",
                "Show the dependency graph between crates in the workspace",
                json!({
                    "type": "object",
                    "properties": {
                        "crate_name": {
                            "type": "string",
                            "description": "Optional: specific crate to analyze dependencies for"
                        }
                    },
                    "required": []
                }),
            ),
            ToolDefinition::new(
                "get_milestone_progress",
                "Get the current implementation progress for milestones M1-M5",
                json!({
                    "type": "object",
                    "properties": {
                        "milestone": {
                            "type": "string",
                            "description": "Optional: specific milestone (M1, M2, M3, M4, M5)"
                        }
                    },
                    "required": []
                }),
            ),
            ToolDefinition::new(
                "search_code",
                "Search for patterns, types, or functions in the codebase",
                json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (supports regex)"
                        },
                        "file_type": {
                            "type": "string",
                            "description": "Optional: file extension filter (e.g., 'rs', 'toml')"
                        }
                    },
                    "required": ["pattern"]
                }),
            ),
            ToolDefinition::new(
                "get_public_api",
                "Get the public API surface of a crate (exported types, functions, traits)",
                json!({
                    "type": "object",
                    "properties": {
                        "crate_name": {
                            "type": "string",
                            "description": "Name of the crate to analyze"
                        }
                    },
                    "required": ["crate_name"]
                }),
            ),
            ToolDefinition::new(
                "run_cargo",
                "Run cargo commands (build, test, check, clippy)",
                json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "enum": ["build", "test", "check", "clippy", "doc"],
                            "description": "Cargo command to run"
                        },
                        "package": {
                            "type": "string",
                            "description": "Optional: specific package to target"
                        }
                    },
                    "required": ["command"]
                }),
            ),
            ToolDefinition::new(
                "get_github_issues",
                "Get GitHub issues related to a milestone or feature",
                json!({
                    "type": "object",
                    "properties": {
                        "milestone": {
                            "type": "string",
                            "description": "Optional: filter by milestone (M1, M2, M3, M4, M5)"
                        },
                        "state": {
                            "type": "string",
                            "enum": ["open", "closed", "all"],
                            "description": "Issue state filter"
                        }
                    },
                    "required": []
                }),
            ),
        ]
    }

    /// Call a tool by name with arguments.
    pub fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<String> {
        match name {
            "get_workspace_info" => self.get_workspace_info(),
            "get_crate_structure" => {
                let crate_name = args
                    .get("crate_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing crate_name argument"))?;
                self.crate_analyzer.get_crate_structure(crate_name)
            }
            "get_xds_resources" => self.crate_analyzer.get_xds_resources(),
            "get_service_endpoints" => {
                let service = args.get("service").and_then(|v| v.as_str());
                self.crate_analyzer.get_service_endpoints(service)
            }
            "analyze_dependencies" => {
                let crate_name = args.get("crate_name").and_then(|v| v.as_str());
                self.dependency_analyzer.analyze(crate_name)
            }
            "get_milestone_progress" => {
                let milestone = args.get("milestone").and_then(|v| v.as_str());
                self.milestone_tracker.get_progress(milestone)
            }
            "search_code" => {
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing pattern argument"))?;
                let file_type = args.get("file_type").and_then(|v| v.as_str());
                self.code_searcher.search(pattern, file_type)
            }
            "get_public_api" => {
                let crate_name = args
                    .get("crate_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing crate_name argument"))?;
                self.crate_analyzer.get_public_api(crate_name)
            }
            "run_cargo" => {
                let command = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing command argument"))?;
                let package = args.get("package").and_then(|v| v.as_str());
                self.cargo_runner.run(command, package)
            }
            "get_github_issues" => {
                let milestone = args.get("milestone").and_then(|v| v.as_str());
                let state = args.get("state").and_then(|v| v.as_str());
                self.milestone_tracker.get_github_issues(milestone, state)
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    fn get_workspace_info(&self) -> Result<String> {
        let mut output = String::new();
        output.push_str("# nebucloud-xds Workspace\n\n");
        output.push_str("A Rust-based xDS (Envoy Discovery Service) control plane implementation.\n\n");

        output.push_str("## Crates\n\n");
        output.push_str("| Crate | Description |\n");
        output.push_str("|-------|-------------|\n");
        output.push_str("| `xds-core` | Core types, traits, and error handling |\n");
        output.push_str("| `xds-types` | xDS resource type definitions (Cluster, Listener, Route, etc.) |\n");
        output.push_str("| `xds-cache` | Snapshot cache and resource versioning |\n");
        output.push_str("| `xds-server` | gRPC server with ADS, SotW, and Delta protocols |\n");
        output.push_str("| `nebucloud-xds` | Main facade crate that re-exports everything |\n\n");

        output.push_str("## Architecture\n\n");
        output.push_str("```\n");
        output.push_str("┌─────────────────────────────────────────────────────────────┐\n");
        output.push_str("│                      nebucloud-xds                          │\n");
        output.push_str("│  (facade crate - re-exports all public APIs)                │\n");
        output.push_str("└───────────────────────────┬─────────────────────────────────┘\n");
        output.push_str("                            │\n");
        output.push_str("    ┌───────────────────────┼───────────────────────┐\n");
        output.push_str("    │                       │                       │\n");
        output.push_str("    ▼                       ▼                       ▼\n");
        output.push_str("┌─────────┐           ┌──────────┐           ┌───────────┐\n");
        output.push_str("│xds-core │◄──────────│xds-cache │◄──────────│xds-server │\n");
        output.push_str("└────┬────┘           └────┬─────┘           └───────────┘\n");
        output.push_str("     │                     │\n");
        output.push_str("     ▼                     ▼\n");
        output.push_str("┌─────────┐           ┌──────────┐\n");
        output.push_str("│xds-types│           │xds-types │\n");
        output.push_str("└─────────┘           └──────────┘\n");
        output.push_str("```\n\n");

        output.push_str("## Key Features\n\n");
        output.push_str("- **State-of-the-World (SotW)** - Full resource updates\n");
        output.push_str("- **Delta xDS** - Incremental resource updates\n");
        output.push_str("- **ADS (Aggregated Discovery Service)** - Single stream for all resource types\n");
        output.push_str("- **Health Checking** - gRPC health protocol\n");
        output.push_str("- **Prometheus Metrics** - Observability\n");
        output.push_str("- **Graceful Shutdown** - Connection draining\n");

        Ok(output)
    }
}

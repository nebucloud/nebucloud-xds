//! Cargo command runner tool.

use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

/// Runner for cargo commands.
pub struct CargoRunner {
    workspace: PathBuf,
}

impl CargoRunner {
    /// Create a new cargo runner.
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    /// Run a cargo command.
    pub fn run(&self, command: &str, package: Option<&str>) -> Result<String> {
        let mut output = String::new();
        
        // Validate command
        let valid_commands = ["build", "test", "check", "clippy", "doc"];
        if !valid_commands.contains(&command) {
            return Err(anyhow::anyhow!(
                "Invalid command: {}. Valid commands: {:?}",
                command,
                valid_commands
            ));
        }

        output.push_str(&format!("# Running: cargo {}", command));
        if let Some(pkg) = package {
            output.push_str(&format!(" -p {}", pkg));
        }
        output.push_str("\n\n");

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&self.workspace);
        cmd.arg(command);

        if let Some(pkg) = package {
            cmd.arg("-p").arg(pkg);
        }

        // Add common flags
        match command {
            "clippy" => {
                cmd.arg("--").arg("-D").arg("warnings");
            }
            "doc" => {
                cmd.arg("--no-deps");
            }
            _ => {}
        }

        match cmd.output() {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                if result.status.success() {
                    output.push_str("## ✅ Success\n\n");
                } else {
                    output.push_str("## ❌ Failed\n\n");
                }

                if !stdout.is_empty() {
                    output.push_str("### stdout\n\n```\n");
                    // Truncate if too long
                    let stdout_str = stdout.to_string();
                    if stdout_str.len() > 5000 {
                        output.push_str(&stdout_str[..5000]);
                        output.push_str("\n... (truncated)");
                    } else {
                        output.push_str(&stdout_str);
                    }
                    output.push_str("\n```\n\n");
                }

                if !stderr.is_empty() {
                    output.push_str("### stderr\n\n```\n");
                    let stderr_str = stderr.to_string();
                    if stderr_str.len() > 5000 {
                        output.push_str(&stderr_str[..5000]);
                        output.push_str("\n... (truncated)");
                    } else {
                        output.push_str(&stderr_str);
                    }
                    output.push_str("\n```\n\n");
                }
            }
            Err(e) => {
                output.push_str(&format!("## ❌ Error\n\nFailed to run cargo: {}\n", e));
            }
        }

        Ok(output)
    }
}

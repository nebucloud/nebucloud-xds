//! Milestone tracking tool.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

/// Tracker for project milestones.
pub struct MilestoneTracker {
    workspace: PathBuf,
}

impl MilestoneTracker {
    /// Create a new milestone tracker.
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    /// Get milestone progress.
    pub fn get_progress(&self, milestone: Option<&str>) -> Result<String> {
        let mut output = String::new();

        // Read MILESTONES.md if it exists
        let milestones_file = self.workspace.join("MILESTONES.md");
        
        if milestones_file.exists() {
            let content = fs::read_to_string(&milestones_file)?;
            
            if let Some(m) = milestone {
                // Extract specific milestone section
                let marker = format!("## {}", m.to_uppercase());
                if let Some(start) = content.find(&marker) {
                    let section = &content[start..];
                    if let Some(end) = section[3..].find("\n## ") {
                        output.push_str(&section[..end + 3]);
                    } else {
                        output.push_str(section);
                    }
                } else {
                    output.push_str(&format!("Milestone {} not found in MILESTONES.md\n", m));
                }
            } else {
                // Return summary of all milestones
                output.push_str("# Milestone Progress\n\n");
                output.push_str(&self.extract_milestone_summary(&content));
            }
        } else {
            output.push_str(&self.get_hardcoded_milestones(milestone));
        }

        Ok(output)
    }

    /// Get GitHub issues related to milestones.
    pub fn get_github_issues(&self, milestone: Option<&str>, state: Option<&str>) -> Result<String> {
        let mut output = String::new();
        output.push_str("# GitHub Issues\n\n");

        // Try to use gh CLI
        let state_arg = state.unwrap_or("all");
        
        let mut cmd = Command::new("gh");
        cmd.current_dir(&self.workspace)
            .arg("issue")
            .arg("list")
            .arg("--state")
            .arg(state_arg)
            .arg("--json")
            .arg("number,title,state,labels");

        match cmd.output() {
            Ok(result) => {
                if result.status.success() {
                    let json_output = String::from_utf8_lossy(&result.stdout);
                    if let Ok(issues) = serde_json::from_str::<Vec<serde_json::Value>>(&json_output) {
                        // Filter by milestone if specified
                        let filtered: Vec<&serde_json::Value> = if let Some(m) = milestone {
                            let milestone_label = m.to_lowercase();
                            issues.iter()
                                .filter(|issue| {
                                    issue.get("labels")
                                        .and_then(|l| l.as_array())
                                        .map_or(false, |labels| {
                                            labels.iter().any(|label| {
                                                label.get("name")
                                                    .and_then(|n| n.as_str())
                                                    .map_or(false, |n| n.to_lowercase().contains(&milestone_label))
                                            })
                                        })
                                })
                                .collect()
                        } else {
                            issues.iter().collect()
                        };

                        output.push_str("| # | Title | State |\n");
                        output.push_str("|---|-------|-------|\n");
                        
                        for issue in filtered {
                            let number = issue.get("number").and_then(|n| n.as_u64()).unwrap_or(0);
                            let title = issue.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown");
                            let state = issue.get("state").and_then(|s| s.as_str()).unwrap_or("unknown");
                            output.push_str(&format!("| #{} | {} | {} |\n", number, title, state));
                        }
                    } else {
                        output.push_str("Failed to parse GitHub issues JSON\n");
                    }
                } else {
                    output.push_str("GitHub CLI returned an error. Make sure you're authenticated with `gh auth login`.\n");
                }
            }
            Err(_) => {
                output.push_str("GitHub CLI (`gh`) not available. Install it to fetch issues.\n\n");
                output.push_str("## Known Issue Ranges\n\n");
                output.push_str("- M1 Foundation: #1-8\n");
                output.push_str("- M2 Core Implementation: #9-16\n");
                output.push_str("- M3 Protocol Handlers: #17-23\n");
                output.push_str("- M4 Production Readiness: #24-31\n");
                output.push_str("- M5 Documentation & Examples: #32-39\n");
            }
        }

        Ok(output)
    }

    fn extract_milestone_summary(&self, content: &str) -> String {
        let mut summary = String::new();
        summary.push_str("| Milestone | Status | Issues |\n");
        summary.push_str("|-----------|--------|--------|\n");

        let milestones = [
            ("M1", "Foundation", "#1-8"),
            ("M2", "Core Implementation", "#9-16"),
            ("M3", "Protocol Handlers", "#17-23"),
            ("M4", "Production Readiness", "#24-31"),
            ("M5", "Documentation & Examples", "#32-39"),
        ];

        for (id, name, issues) in milestones {
            let status = if content.contains(&format!("{}: ✅", id)) || content.contains(&format!("{} ✅", id)) {
                "✅ Complete"
            } else if content.contains(&format!("{}: 🟡", id)) || content.contains(&format!("{} 🟡", id)) {
                "🟡 In Progress"
            } else {
                "⬜ Not Started"
            };
            summary.push_str(&format!("| {} | {} - {} | {} |\n", id, id, name, issues));
            summary.push_str(&format!("|   | {} |   |\n", status));
        }

        summary
    }

    fn get_hardcoded_milestones(&self, milestone: Option<&str>) -> String {
        let milestones = r#"# Milestones

## M1 - Foundation ✅
Issues: #1-8
- [x] Workspace structure
- [x] Core types (Resource, TypeUrl, Version)
- [x] Error handling
- [x] Basic traits

## M2 - Core Implementation ✅
Issues: #9-16
- [x] xds-types crate (Cluster, Listener, Route, Endpoint, Secret)
- [x] Resource registry
- [x] Snapshot cache
- [x] Version tracking

## M3 - Protocol Handlers ✅
Issues: #17-23
- [x] SotW protocol
- [x] Delta protocol
- [x] ADS service
- [x] Individual discovery services (CDS, LDS, RDS, EDS, SDS)

## M4 - Production Readiness 🟡
Issues: #24-31
- [x] XdsServerBuilder
- [x] Prometheus metrics
- [x] Health checking
- [x] gRPC reflection
- [x] Graceful shutdown
- [x] Connection tracking
- [ ] Performance benchmarks
- [ ] Load testing

## M5 - Documentation & Examples
Issues: #32-39
- [ ] API documentation
- [ ] Architecture guide
- [ ] Simple server example
- [ ] Kubernetes controller example
- [ ] Integration tests
"#;

        if let Some(m) = milestone {
            let marker = format!("## {}", m.to_uppercase());
            if let Some(start) = milestones.find(&marker) {
                let section = &milestones[start..];
                if let Some(end) = section[3..].find("\n## ") {
                    return section[..end + 3].to_string();
                } else {
                    return section.to_string();
                }
            }
        }

        milestones.to_string()
    }
}

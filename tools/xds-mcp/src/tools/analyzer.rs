//! Dependency analysis tool.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

/// Analyzer for crate dependencies.
pub struct DependencyAnalyzer {
    workspace: PathBuf,
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer.
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    /// Analyze dependencies for all crates or a specific crate.
    pub fn analyze(&self, crate_name: Option<&str>) -> Result<String> {
        let mut output = String::new();
        
        output.push_str("# Dependency Analysis\n\n");

        // Get all crates in the workspace
        let crates_dir = self.workspace.join("crates");
        let mut crates: Vec<String> = Vec::new();

        if let Ok(entries) = fs::read_dir(&crates_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        crates.push(name.to_string());
                    }
                }
            }
        }

        crates.sort();

        // Build dependency map
        let mut dep_map: HashMap<String, Vec<String>> = HashMap::new();
        
        for crate_name_item in &crates {
            let cargo_toml = crates_dir.join(crate_name_item).join("Cargo.toml");
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                let deps = self.extract_workspace_deps(&content, &crates);
                dep_map.insert(crate_name_item.clone(), deps);
            }
        }

        // Filter if specific crate requested
        if let Some(filter) = crate_name {
            if !dep_map.contains_key(filter) {
                return Err(anyhow::anyhow!("Crate not found: {}", filter));
            }
            
            output.push_str(&format!("## {} Dependencies\n\n", filter));
            
            // Direct dependencies
            output.push_str("### Direct Dependencies\n\n");
            if let Some(deps) = dep_map.get(filter) {
                if deps.is_empty() {
                    output.push_str("_None_\n\n");
                } else {
                    for dep in deps {
                        output.push_str(&format!("- `{}`\n", dep));
                    }
                    output.push('\n');
                }
            }

            // Reverse dependencies (what depends on this crate)
            output.push_str("### Dependents (crates that depend on this)\n\n");
            let mut dependents: Vec<&String> = dep_map
                .iter()
                .filter(|(_, deps)| deps.contains(&filter.to_string()))
                .map(|(name, _)| name)
                .collect();
            dependents.sort();
            
            if dependents.is_empty() {
                output.push_str("_None_\n\n");
            } else {
                for dep in dependents {
                    output.push_str(&format!("- `{}`\n", dep));
                }
                output.push('\n');
            }
        } else {
            // Show all dependencies
            output.push_str("## Dependency Graph\n\n");
            output.push_str("```mermaid\n");
            output.push_str("graph TD\n");
            
            for (crate_name_item, deps) in &dep_map {
                for dep in deps {
                    output.push_str(&format!("    {} --> {}\n", 
                        crate_name_item.replace('-', "_"), 
                        dep.replace('-', "_")));
                }
            }
            output.push_str("```\n\n");

            // Summary table
            output.push_str("## Summary\n\n");
            output.push_str("| Crate | Dependencies | Dependents |\n");
            output.push_str("|-------|--------------|------------|\n");
            
            for crate_name_item in &crates {
                let deps = dep_map.get(crate_name_item).map_or(0, |d| d.len());
                let dependents = dep_map
                    .iter()
                    .filter(|(_, d)| d.contains(crate_name_item))
                    .count();
                output.push_str(&format!("| `{}` | {} | {} |\n", crate_name_item, deps, dependents));
            }
        }

        Ok(output)
    }

    fn extract_workspace_deps(&self, cargo_toml: &str, workspace_crates: &[String]) -> Vec<String> {
        let mut deps = Vec::new();
        
        for crate_name in workspace_crates {
            // Look for patterns like:
            // xds-core = { workspace = true }
            // xds-core.workspace = true
            if cargo_toml.contains(&format!("{} = {{ workspace", crate_name))
                || cargo_toml.contains(&format!("{}.workspace", crate_name))
                || cargo_toml.contains(&format!("{} = {{ version", crate_name))
            {
                deps.push(crate_name.clone());
            }
        }
        
        deps
    }
}

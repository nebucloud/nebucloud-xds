//! Crate structure analysis tool.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use regex::Regex;
use walkdir::WalkDir;

/// Analyzer for crate structure and xDS resources.
pub struct CrateAnalyzer {
    workspace: PathBuf,
}

impl CrateAnalyzer {
    /// Create a new crate analyzer.
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    /// Get the module structure of a crate.
    pub fn get_crate_structure(&self, crate_name: &str) -> Result<String> {
        let crate_path = self.workspace.join("crates").join(crate_name);
        
        if !crate_path.exists() {
            return Err(anyhow::anyhow!("Crate not found: {}", crate_name));
        }

        let src_path = crate_path.join("src");
        let mut output = String::new();
        output.push_str(&format!("# Crate: {}\n\n", crate_name));

        // Read Cargo.toml for description
        let cargo_toml = crate_path.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if let Some(desc) = self.extract_description(&content) {
                output.push_str(&format!("**Description:** {}\n\n", desc));
            }
        }

        output.push_str("## Module Structure\n\n");
        output.push_str("```\n");
        self.build_tree(&src_path, "", &mut output)?;
        output.push_str("```\n\n");

        // Analyze key types
        output.push_str("## Key Types\n\n");
        let types = self.find_public_types(&src_path)?;
        for (category, items) in types {
            output.push_str(&format!("### {}\n\n", category));
            for item in items {
                output.push_str(&format!("- `{}`\n", item));
            }
            output.push('\n');
        }

        Ok(output)
    }

    /// Get all xDS resource types.
    pub fn get_xds_resources(&self) -> Result<String> {
        let _types_path = self.workspace.join("crates").join("xds-types").join("src");
        let mut output = String::new();
        
        output.push_str("# xDS Resource Types\n\n");
        output.push_str("| Resource | Type URL | Module |\n");
        output.push_str("|----------|----------|--------|\n");

        let resources = [
            ("Cluster", "type.googleapis.com/envoy.config.cluster.v3.Cluster", "cluster"),
            ("Listener", "type.googleapis.com/envoy.config.listener.v3.Listener", "listener"),
            ("RouteConfiguration", "type.googleapis.com/envoy.config.route.v3.RouteConfiguration", "route"),
            ("ClusterLoadAssignment", "type.googleapis.com/envoy.config.endpoint.v3.ClusterLoadAssignment", "endpoint"),
            ("Secret", "type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.Secret", "secret"),
            ("Runtime", "type.googleapis.com/envoy.service.runtime.v3.Runtime", "runtime"),
            ("VirtualHost", "type.googleapis.com/envoy.config.route.v3.VirtualHost", "route"),
            ("ScopedRouteConfiguration", "type.googleapis.com/envoy.config.route.v3.ScopedRouteConfiguration", "route"),
        ];

        for (name, type_url, module) in resources {
            output.push_str(&format!("| {} | `{}` | `{}` |\n", name, type_url, module));
        }

        output.push_str("\n## Service Mapping\n\n");
        output.push_str("| Service | Resource Types |\n");
        output.push_str("|---------|----------------|\n");
        output.push_str("| CDS (Cluster Discovery) | Cluster |\n");
        output.push_str("| LDS (Listener Discovery) | Listener |\n");
        output.push_str("| RDS (Route Discovery) | RouteConfiguration, VirtualHost |\n");
        output.push_str("| EDS (Endpoint Discovery) | ClusterLoadAssignment |\n");
        output.push_str("| SDS (Secret Discovery) | Secret |\n");
        output.push_str("| ADS (Aggregated Discovery) | All of the above |\n");

        Ok(output)
    }

    /// Get gRPC service endpoints.
    pub fn get_service_endpoints(&self, service_filter: Option<&str>) -> Result<String> {
        let _server_path = self.workspace.join("crates").join("xds-server").join("src").join("services");
        let mut output = String::new();

        output.push_str("# gRPC Service Endpoints\n\n");

        let services = [
            ("ADS", "Aggregated Discovery Service", "ads.rs", &[
                ("StreamAggregatedResources", "Bidirectional stream for all resource types"),
                ("DeltaAggregatedResources", "Delta updates for all resource types"),
            ][..]),
            ("CDS", "Cluster Discovery Service", "cds.rs", &[
                ("StreamClusters", "Stream cluster configurations"),
                ("DeltaClusters", "Delta cluster updates"),
                ("FetchClusters", "Unary cluster fetch"),
            ][..]),
            ("LDS", "Listener Discovery Service", "lds.rs", &[
                ("StreamListeners", "Stream listener configurations"),
                ("DeltaListeners", "Delta listener updates"),
                ("FetchListeners", "Unary listener fetch"),
            ][..]),
            ("RDS", "Route Discovery Service", "rds.rs", &[
                ("StreamRoutes", "Stream route configurations"),
                ("DeltaRoutes", "Delta route updates"),
                ("FetchRoutes", "Unary route fetch"),
            ][..]),
            ("EDS", "Endpoint Discovery Service", "eds.rs", &[
                ("StreamEndpoints", "Stream endpoint configurations"),
                ("DeltaEndpoints", "Delta endpoint updates"),
                ("FetchEndpoints", "Unary endpoint fetch"),
            ][..]),
            ("SDS", "Secret Discovery Service", "sds.rs", &[
                ("StreamSecrets", "Stream secret configurations"),
                ("DeltaSecrets", "Delta secret updates"),
                ("FetchSecrets", "Unary secret fetch"),
            ][..]),
        ];

        for (name, description, file, methods) in services {
            if let Some(filter) = service_filter {
                if !name.eq_ignore_ascii_case(filter) {
                    continue;
                }
            }

            output.push_str(&format!("## {} - {}\n\n", name, description));
            output.push_str(&format!("**File:** `crates/xds-server/src/services/{}`\n\n", file));
            output.push_str("| Method | Description |\n");
            output.push_str("|--------|-------------|\n");
            for (method, desc) in methods {
                output.push_str(&format!("| `{}` | {} |\n", method, desc));
            }
            output.push('\n');
        }

        Ok(output)
    }

    /// Get public API of a crate.
    pub fn get_public_api(&self, crate_name: &str) -> Result<String> {
        let crate_path = self.workspace.join("crates").join(crate_name);
        let lib_path = crate_path.join("src").join("lib.rs");

        if !lib_path.exists() {
            return Err(anyhow::anyhow!("Crate lib.rs not found: {}", crate_name));
        }

        let content = fs::read_to_string(&lib_path)?;
        let mut output = String::new();

        output.push_str(&format!("# Public API: {}\n\n", crate_name));

        // Find pub use statements
        output.push_str("## Re-exports\n\n");
        let re_exports: Vec<&str> = content
            .lines()
            .filter(|line| line.starts_with("pub use "))
            .collect();
        
        if re_exports.is_empty() {
            output.push_str("_No direct re-exports_\n\n");
        } else {
            for export in re_exports {
                output.push_str(&format!("- `{}`\n", export.trim()));
            }
            output.push('\n');
        }

        // Find pub mod statements
        output.push_str("## Public Modules\n\n");
        let pub_mods: Vec<&str> = content
            .lines()
            .filter(|line| line.starts_with("pub mod "))
            .collect();
        
        for module in pub_mods {
            output.push_str(&format!("- `{}`\n", module.trim()));
        }
        output.push('\n');

        // Find pub struct/enum/trait
        output.push_str("## Types (from lib.rs)\n\n");
        let type_re = Regex::new(r"pub (struct|enum|trait|type) (\w+)")?;
        for cap in type_re.captures_iter(&content) {
            let kind = &cap[1];
            let name = &cap[2];
            output.push_str(&format!("- `{}` ({})\n", name, kind));
        }

        Ok(output)
    }

    fn extract_description(&self, cargo_toml: &str) -> Option<String> {
        for line in cargo_toml.lines() {
            if line.starts_with("description") {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    return Some(parts[1].trim().trim_matches('"').to_string());
                }
            }
        }
        None
    }

    fn build_tree(&self, path: &PathBuf, prefix: &str, output: &mut String) -> Result<()> {
        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                !name_str.starts_with('.') && name_str.ends_with(".rs") || e.path().is_dir()
            })
            .collect();

        entries.sort_by_key(|e| e.file_name());

        let count = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            let is_last = i == count - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if entry.path().is_dir() {
                output.push_str(&format!("{}{}{}/\n", prefix, connector, name_str));
                let new_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
                self.build_tree(&entry.path(), &new_prefix, output)?;
            } else {
                output.push_str(&format!("{}{}{}\n", prefix, connector, name_str));
            }
        }

        Ok(())
    }

    fn find_public_types(&self, src_path: &PathBuf) -> Result<HashMap<String, Vec<String>>> {
        let mut types: HashMap<String, Vec<String>> = HashMap::new();
        types.insert("Structs".to_string(), Vec::new());
        types.insert("Enums".to_string(), Vec::new());
        types.insert("Traits".to_string(), Vec::new());

        let struct_re = Regex::new(r"pub struct (\w+)")?;
        let enum_re = Regex::new(r"pub enum (\w+)")?;
        let trait_re = Regex::new(r"pub trait (\w+)")?;

        for entry in WalkDir::new(src_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let content = fs::read_to_string(entry.path())?;
            
            for cap in struct_re.captures_iter(&content) {
                types.get_mut("Structs").unwrap().push(cap[1].to_string());
            }
            for cap in enum_re.captures_iter(&content) {
                types.get_mut("Enums").unwrap().push(cap[1].to_string());
            }
            for cap in trait_re.captures_iter(&content) {
                types.get_mut("Traits").unwrap().push(cap[1].to_string());
            }
        }

        // Remove empty categories
        types.retain(|_, v| !v.is_empty());

        Ok(types)
    }
}

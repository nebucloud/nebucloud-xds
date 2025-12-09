//! Code search tool.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use regex::Regex;
use walkdir::WalkDir;

/// Searcher for code patterns.
pub struct CodeSearcher {
    workspace: PathBuf,
}

impl CodeSearcher {
    /// Create a new code searcher.
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    /// Search for a pattern in the codebase.
    pub fn search(&self, pattern: &str, file_type: Option<&str>) -> Result<String> {
        let mut output = String::new();
        let regex = Regex::new(pattern)?;
        
        output.push_str(&format!("# Search Results: `{}`\n\n", pattern));

        let extension = file_type.unwrap_or("rs");
        let crates_dir = self.workspace.join("crates");

        let mut matches: Vec<(String, usize, String)> = Vec::new();

        for entry in WalkDir::new(&crates_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == extension)
            })
        {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        let relative_path = entry
                            .path()
                            .strip_prefix(&self.workspace)
                            .unwrap_or(entry.path())
                            .to_string_lossy()
                            .to_string();
                        matches.push((relative_path, line_num + 1, line.trim().to_string()));
                    }
                }
            }
        }

        if matches.is_empty() {
            output.push_str("_No matches found_\n");
        } else {
            output.push_str(&format!("Found {} matches:\n\n", matches.len()));
            
            // Group by file
            let mut current_file = String::new();
            for (file, line, content) in &matches {
                if file != &current_file {
                    output.push_str(&format!("\n## `{}`\n\n", file));
                    current_file = file.clone();
                }
                // Truncate long lines
                let display_content = if content.len() > 100 {
                    format!("{}...", &content[..100])
                } else {
                    content.clone()
                };
                output.push_str(&format!("- **L{}**: `{}`\n", line, display_content));
            }
        }

        Ok(output)
    }
}

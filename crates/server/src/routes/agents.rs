//! Agent discovery API endpoints
//!
//! Scans `.claude/agents` directories for agent definition files and returns
//! their metadata parsed from YAML frontmatter.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use axum::{
    Router,
    extract::{Path as AxumPath, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::project_repo::ProjectRepo;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::DeploymentImpl;
use utils::response::ApiResponse;

/// Metadata extracted from an agent definition file
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentMetadata {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub path: String,
    pub avatar_letter: String,
}

/// Response containing a list of discovered agents
#[derive(Debug, Serialize, TS)]
pub struct AgentListResponse {
    pub agents: Vec<AgentMetadata>,
}

/// YAML frontmatter structure for agent files
#[derive(Debug, Deserialize)]
struct AgentFrontmatter {
    name: Option<String>,
    description: Option<String>,
    tools: Option<String>,
    model: Option<String>,
}

/// Get all agents from the global `.claude/agents` directory
pub async fn get_global_agents() -> ResponseJson<ApiResponse<AgentListResponse>> {
    let agents_dir = PathBuf::from(".claude/agents");
    let agents = scan_agents_directory(&agents_dir).await;
    ResponseJson(ApiResponse::success(AgentListResponse { agents }))
}

/// Get agents from a project-specific `.claude/agents` directory
pub async fn get_project_agents(
    State(deployment): State<DeploymentImpl>,
    AxumPath(project_id): AxumPath<Uuid>,
) -> ResponseJson<ApiResponse<AgentListResponse>> {
    // Get the project's repositories to find agent directories
    let repos = match ProjectRepo::find_repos_for_project(&deployment.db().pool, project_id).await {
        Ok(repos) => repos,
        Err(e) => {
            tracing::warn!("Failed to get project repositories for {}: {}", project_id, e);
            return ResponseJson(ApiResponse::success(AgentListResponse { agents: vec![] }));
        }
    };

    let mut all_agents = Vec::new();
    let mut seen_names = HashSet::new();

    // Scan each repository's .claude/agents directory
    for repo in repos {
        let agents_dir = repo.path.join(".claude/agents");
        let agents = scan_agents_directory(&agents_dir).await;

        // Deduplicate by name (keep first occurrence)
        for agent in agents {
            if seen_names.insert(agent.name.clone()) {
                all_agents.push(agent);
            }
        }
    }

    ResponseJson(ApiResponse::success(AgentListResponse { agents: all_agents }))
}

/// Scan a directory for agent definition files and parse their metadata
async fn scan_agents_directory(dir: &Path) -> Vec<AgentMetadata> {
    let mut agents = Vec::new();
    let mut seen_names = HashSet::new();

    // Check if directory exists
    let dir_path = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        // For relative paths, use current working directory
        std::env::current_dir()
            .map(|cwd| cwd.join(dir))
            .unwrap_or_else(|_| dir.to_path_buf())
    };

    let entries = match tokio::fs::read_dir(&dir_path).await {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Failed to read agents directory {:?}: {}", dir_path, e);
            }
            return agents;
        }
    };

    // Collect entries
    let mut entries_vec = Vec::new();
    let mut entries = entries;
    while let Ok(Some(entry)) = entries.next_entry().await {
        entries_vec.push(entry);
    }

    for entry in entries_vec {
        let path = entry.path();

        // Only process .md files
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        match parse_agent_file(&path).await {
            Ok(Some(agent)) => {
                // Deduplicate by name
                if seen_names.insert(agent.name.clone()) {
                    agents.push(agent);
                } else {
                    tracing::debug!("Skipping duplicate agent: {}", agent.name);
                }
            }
            Ok(None) => {
                tracing::debug!("No valid frontmatter in {:?}", path);
            }
            Err(e) => {
                tracing::warn!("Failed to parse agent file {:?}: {}", path, e);
            }
        }
    }

    // Sort by name for consistent ordering
    agents.sort_by(|a, b| a.name.cmp(&b.name));

    agents
}

/// Parse an agent definition file and extract metadata from YAML frontmatter
async fn parse_agent_file(path: &Path) -> Result<Option<AgentMetadata>, std::io::Error> {
    let content = tokio::fs::read_to_string(path).await?;

    // Extract YAML frontmatter (between --- markers)
    let frontmatter = extract_frontmatter(&content);
    let yaml_str = match frontmatter {
        Some(s) => s,
        None => return Ok(None),
    };

    // Parse YAML
    let frontmatter: AgentFrontmatter = match serde_yaml::from_str(yaml_str) {
        Ok(fm) => fm,
        Err(e) => {
            tracing::warn!("Invalid YAML in {:?}: {}", path, e);
            return Ok(None);
        }
    };

    // Extract name (required)
    let name = match frontmatter.name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => {
            // Fall back to filename without extension
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        }
    };

    // Extract description
    let description = frontmatter
        .description
        .map(|d| d.trim().to_string())
        .unwrap_or_default();

    // Parse tools (comma-separated string to Vec)
    let tools = frontmatter.tools.map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });

    // Extract model
    let model = frontmatter.model.map(|m| m.trim().to_string());

    // Generate avatar letter from first character of name
    let avatar_letter = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    // Use relative path for display
    let display_path = path.to_string_lossy().to_string();

    Ok(Some(AgentMetadata {
        name,
        description,
        tools,
        model,
        path: display_path,
        avatar_letter,
    }))
}

/// Extract YAML frontmatter from markdown content
fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim_start();

    // Must start with ---
    if !content.starts_with("---") {
        return None;
    }

    // Find the closing ---
    let after_first = &content[3..];
    let end_pos = after_first.find("\n---")?;

    Some(&after_first[..end_pos])
}

/// Create the router for agent discovery endpoints
pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/agents/global", get(get_global_agents))
        .route("/agents/project/{project_id}", get(get_project_agents))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let content = r#"---
name: scout
description: Code exploration specialist
tools: Glob, Grep, Read
model: haiku
---

# Scout Agent

Some content here.
"#;

        let frontmatter = extract_frontmatter(content);
        assert!(frontmatter.is_some());
        let fm = frontmatter.unwrap();
        assert!(fm.contains("name: scout"));
        assert!(fm.contains("description: Code exploration specialist"));
    }

    #[test]
    fn test_extract_frontmatter_no_frontmatter() {
        let content = "# Just a markdown file\n\nNo frontmatter here.";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn test_extract_frontmatter_whitespace() {
        let content = r#"
---
name: test
---

Content
"#;

        let frontmatter = extract_frontmatter(content);
        assert!(frontmatter.is_some());
    }

    #[test]
    fn test_parse_tools_string() {
        let tools_str = "Glob, Grep, Read, Bash";
        let tools: Vec<String> = tools_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert_eq!(tools, vec!["Glob", "Grep", "Read", "Bash"]);
    }
}

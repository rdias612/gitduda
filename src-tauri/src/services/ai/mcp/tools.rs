//! MCP tool definitions and handlers
//!
//! Each tool maps to git2 operations for querying repository state.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An MCP tool definition
#[derive(Debug, Clone, Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Get the list of all available MCP tools
pub fn get_tool_list() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "get_commit_history".to_string(),
            description: "Get recent commit history for a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "count": {
                        "type": "integer",
                        "description": "Number of commits to return (default 20)"
                    },
                    "branch": {
                        "type": "string",
                        "description": "Branch name to get history for (default: HEAD)"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_branches".to_string(),
            description: "List all branches in a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_status".to_string(),
            description: "Get working directory status (staged, modified, untracked files)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_diff".to_string(),
            description: "Get diff for working directory or between two refs".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "from_ref": {
                        "type": "string",
                        "description": "Starting reference (commit SHA, branch, tag)"
                    },
                    "to_ref": {
                        "type": "string",
                        "description": "Ending reference (commit SHA, branch, tag)"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Limit the diff to a single file (relative path)"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_file_blame".to_string(),
            description: "Get blame information for a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Relative path to the file within the repository"
                    }
                },
                "required": ["repo_path", "file_path"]
            }),
        },
        McpTool {
            name: "search_commits".to_string(),
            description: "Search commits by message content".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query to match against commit messages"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results (default 20)"
                    }
                },
                "required": ["repo_path", "query"]
            }),
        },
        McpTool {
            name: "get_open_repositories".to_string(),
            description: "Get list of repositories currently open in Gitduda".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_commit".to_string(),
            description:
                "Get full details of a single commit: metadata, parents and the files it changed"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "sha": {
                        "type": "string",
                        "description": "Commit SHA"
                    }
                },
                "required": ["repo_path", "sha"]
            }),
        },
        McpTool {
            name: "get_file_history".to_string(),
            description: "Get the commits that touched a file, following renames by default"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Relative path to the file within the repository"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results (default 100, max 500)"
                    },
                    "follow_renames": {
                        "type": "boolean",
                        "description": "Follow the file across renames (default true)"
                    }
                },
                "required": ["repo_path", "file_path"]
            }),
        },
        McpTool {
            name: "get_tags".to_string(),
            description:
                "List all tags in a repository (annotated tags include message and tagger)"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_stashes".to_string(),
            description: "List all stashes in a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_worktrees".to_string(),
            description: "List all worktrees of a repository".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_remotes".to_string(),
            description: "List configured remotes with their fetch and push URLs".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    }
                },
                "required": ["repo_path"]
            }),
        },
        McpTool {
            name: "get_commit_stats".to_string(),
            description:
                "Get addition/deletion statistics for the changes between two refs (to_ref defaults to HEAD)"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "from_ref": {
                        "type": "string",
                        "description": "Starting reference (commit SHA, branch, tag)"
                    },
                    "to_ref": {
                        "type": "string",
                        "description": "Ending reference (default HEAD)"
                    }
                },
                "required": ["repo_path", "from_ref"]
            }),
        },
        McpTool {
            name: "get_file_content".to_string(),
            description:
                "Read a file's content from the working directory (truncated when larger than max_bytes)"
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Path to the git repository"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Relative path to the file within the repository"
                    },
                    "max_bytes": {
                        "type": "integer",
                        "description": "Maximum bytes of content to return (default 262144, max 2097152)"
                    }
                },
                "required": ["repo_path", "file_path"]
            }),
        },
    ]
}

/// Parameters for get_commit_history
#[derive(Debug, Deserialize)]
struct CommitHistoryParams {
    repo_path: String,
    count: Option<u32>,
    branch: Option<String>,
}

/// Parameters for get_branches
#[derive(Debug, Deserialize)]
struct BranchesParams {
    repo_path: String,
}

/// Parameters for get_status
#[derive(Debug, Deserialize)]
struct StatusParams {
    repo_path: String,
}

/// Parameters for get_diff
#[derive(Debug, Deserialize)]
struct DiffParams {
    repo_path: String,
    from_ref: Option<String>,
    to_ref: Option<String>,
    file_path: Option<String>,
}

/// Parameters for get_file_blame
#[derive(Debug, Deserialize)]
struct BlameParams {
    repo_path: String,
    file_path: String,
}

/// Parameters for search_commits
#[derive(Debug, Deserialize)]
struct SearchCommitsParams {
    repo_path: String,
    query: String,
    max_results: Option<u32>,
}

/// Parameters for get_commit
#[derive(Debug, Deserialize)]
struct CommitParams {
    repo_path: String,
    sha: String,
}

/// Parameters for get_file_history
#[derive(Debug, Deserialize)]
struct FileHistoryParams {
    repo_path: String,
    file_path: String,
    max_results: Option<u32>,
    follow_renames: Option<bool>,
}

/// Parameters for the tools that only need the repository (get_tags,
/// get_stashes, get_worktrees, get_remotes)
#[derive(Debug, Deserialize)]
struct RepoParams {
    repo_path: String,
}

/// Parameters for get_commit_stats
#[derive(Debug, Deserialize)]
struct CommitStatsParams {
    repo_path: String,
    from_ref: Option<String>,
    to_ref: Option<String>,
}

/// Parameters for get_file_content
#[derive(Debug, Deserialize)]
struct FileContentParams {
    repo_path: String,
    file_path: String,
    max_bytes: Option<usize>,
}

/// Cap on how much content get_file_content returns in one answer.
const DEFAULT_MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_FILE_BYTES_CAP: usize = 2 * 1024 * 1024;

/// Same ceiling the commit-history command uses before it gives up walking.
const MAX_FILE_HISTORY_COMMITS: usize = 25_000;

/// Validate that a repo_path is an actual git repository and is not a path traversal attempt.
///
/// Only allows access to repositories whose paths are in the `open_repos` list,
/// or any valid git repository if the list is empty (for backwards compatibility
/// during initial setup before repos are registered).
pub(crate) fn validate_repo_path(repo_path: &str, open_repos: &[String]) -> Result<(), String> {
    let path = std::path::Path::new(repo_path);

    // Must be an absolute path
    if !path.is_absolute() {
        return Err("Repository path must be absolute".to_string());
    }

    // Canonicalize to resolve symlinks and ../ traversals
    let canonical = path
        .canonicalize()
        .map_err(|_| format!("Repository path does not exist: {}", repo_path))?;

    // If we have open repos registered, only allow access to those
    if !open_repos.is_empty() {
        let canonical_str = canonical.to_string_lossy();
        let allowed = open_repos.iter().any(|r| {
            if let Ok(c) = std::path::Path::new(r).canonicalize() {
                canonical_str.starts_with(&*c.to_string_lossy())
            } else {
                false
            }
        });
        if !allowed {
            return Err(format!(
                "Access denied: '{}' is not an open repository",
                repo_path
            ));
        }
    }

    // Verify it's actually a git repository
    git2::Repository::open(&canonical)
        .map_err(|_| format!("Not a valid git repository: {}", repo_path))?;

    Ok(())
}

/// Call a tool by name with the given arguments
pub async fn call_tool(
    name: &str,
    arguments: &Value,
    open_repos: &[String],
) -> Result<Value, String> {
    let name = name.to_string();
    let arguments = arguments.clone();
    let open_repos = open_repos.to_vec();

    tokio::task::spawn_blocking(move || execute_tool(&name, &arguments, &open_repos))
        .await
        .map_err(|e| format!("Tool execution task failed: {}", e))?
}

fn execute_tool(name: &str, arguments: &Value, open_repos: &[String]) -> Result<Value, String> {
    match name {
        "get_commit_history" => {
            let params: CommitHistoryParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_commit_history(params)
        }
        "get_branches" => {
            let params: BranchesParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_branches(params)
        }
        "get_status" => {
            let params: StatusParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_status(params)
        }
        "get_diff" => {
            let params: DiffParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_diff(params)
        }
        "get_file_blame" => {
            let params: BlameParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_file_blame(params)
        }
        "search_commits" => {
            let params: SearchCommitsParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_search_commits(params)
        }
        "get_open_repositories" => tool_get_open_repositories(open_repos),
        "get_commit" => {
            let params: CommitParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_commit(params)
        }
        "get_file_history" => {
            let params: FileHistoryParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_file_history(params)
        }
        "get_tags" | "get_stashes" | "get_worktrees" | "get_remotes" => {
            let params: RepoParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            match name {
                "get_tags" => tool_get_tags(params),
                "get_stashes" => tool_get_stashes(params),
                "get_worktrees" => tool_get_worktrees(params),
                _ => tool_get_remotes(params),
            }
        }
        "get_commit_stats" => {
            let params: CommitStatsParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_commit_stats(params)
        }
        "get_file_content" => {
            let params: FileContentParams = serde_json::from_value(arguments.clone())
                .map_err(|e| format!("Invalid arguments: {}", e))?;
            validate_repo_path(&params.repo_path, open_repos)?;
            tool_get_file_content(params)
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

/// Get commit history for a repository
fn tool_get_commit_history(params: CommitHistoryParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let count = params.count.unwrap_or(20) as usize;

    let mut revwalk = repo
        .revwalk()
        .map_err(|e| format!("Failed to create revwalk: {}", e))?;

    // Start from the specified branch or HEAD
    if let Some(ref branch) = params.branch {
        let reference = repo
            .resolve_reference_from_short_name(branch)
            .map_err(|e| format!("Failed to resolve branch '{}': {}", branch, e))?;
        let oid = reference
            .peel_to_commit()
            .map_err(|e| format!("Failed to resolve '{}' to commit: {}", branch, e))?
            .id();
        revwalk
            .push(oid)
            .map_err(|e| format!("Failed to push to revwalk: {}", e))?;
    } else {
        revwalk
            .push_head()
            .map_err(|e| format!("Failed to push HEAD: {}", e))?;
    }

    revwalk.set_sorting(git2::Sort::TIME).ok();

    let mut commits = Vec::new();
    for oid_result in revwalk.take(count) {
        let oid = oid_result.map_err(|e| format!("Revwalk error: {}", e))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| format!("Failed to find commit: {}", e))?;

        let author = commit.author();
        commits.push(serde_json::json!({
            "sha": oid.to_string(),
            "message": commit.message().unwrap_or("").to_string(),
            "author": author.name().unwrap_or("Unknown").to_string(),
            "date": commit.time().seconds()
        }));
    }

    Ok(Value::Array(commits))
}

/// List all branches in a repository
fn tool_get_branches(params: BranchesParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let head = repo.head().ok();
    let head_name = head
        .as_ref()
        .and_then(|h| h.shorthand().ok().map(String::from));

    let mut branches = Vec::new();

    for branch_result in repo
        .branches(None)
        .map_err(|e| format!("Failed to list branches: {}", e))?
    {
        let (branch, branch_type) =
            branch_result.map_err(|e| format!("Failed to get branch: {}", e))?;

        let name = branch
            .name()
            .map_err(|e| format!("Failed to get branch name: {}", e))?
            .unwrap_or("unknown")
            .to_string();

        let is_remote = branch_type == git2::BranchType::Remote;
        let is_current = !is_remote && head_name.as_deref() == Some(&name);

        branches.push(serde_json::json!({
            "name": name,
            "isCurrent": is_current,
            "isRemote": is_remote
        }));
    }

    Ok(Value::Array(branches))
}

/// Get working directory status
fn tool_get_status(params: StatusParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let statuses = repo
        .statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(true),
        ))
        .map_err(|e| format!("Failed to get status: {}", e))?;

    let mut staged = Vec::new();
    let mut modified = Vec::new();
    let mut untracked = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status = entry.status();

        if status.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        ) {
            staged.push(path.clone());
        }

        if status.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        ) {
            modified.push(path.clone());
        }

        if status.contains(git2::Status::WT_NEW) {
            untracked.push(path);
        }
    }

    Ok(serde_json::json!({
        "staged": staged,
        "modified": modified,
        "untracked": untracked
    }))
}

/// Get diff for working directory or between two refs
fn tool_get_diff(params: DiffParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    // Optional per-file filter. Pathspecs are escaped literals so a filename
    // containing `*`, `?` or `[` only ever matches itself.
    let mut diff_opts = params.file_path.as_ref().map(|fp| {
        let mut o = git2::DiffOptions::new();
        o.pathspec(crate::commands::path_utils::literal_git_pathspec(fp));
        o
    });

    let diff = if let (Some(from_ref), Some(to_ref)) = (&params.from_ref, &params.to_ref) {
        // Diff between two refs
        let from_obj = repo
            .revparse_single(from_ref)
            .map_err(|e| format!("Failed to resolve '{}': {}", from_ref, e))?;
        let to_obj = repo
            .revparse_single(to_ref)
            .map_err(|e| format!("Failed to resolve '{}': {}", to_ref, e))?;

        let from_tree = from_obj
            .peel_to_tree()
            .map_err(|e| format!("Failed to get tree for '{}': {}", from_ref, e))?;
        let to_tree = to_obj
            .peel_to_tree()
            .map_err(|e| format!("Failed to get tree for '{}': {}", to_ref, e))?;

        repo.diff_tree_to_tree(
            Some(&from_tree),
            Some(&to_tree),
            diff_opts.as_mut(),
        )
        .map_err(|e| format!("Failed to create diff: {}", e))?
    } else if let Some(from_ref) = &params.from_ref {
        // Diff from a ref to working directory
        let from_obj = repo
            .revparse_single(from_ref)
            .map_err(|e| format!("Failed to resolve '{}': {}", from_ref, e))?;
        let from_tree = from_obj
            .peel_to_tree()
            .map_err(|e| format!("Failed to get tree for '{}': {}", from_ref, e))?;

        repo.diff_tree_to_workdir_with_index(Some(&from_tree), diff_opts.as_mut())
            .map_err(|e| format!("Failed to create diff: {}", e))?
    } else {
        // Working directory diff (unstaged changes)
        let head = repo.head().ok();
        let head_tree = head.and_then(|h| h.peel_to_tree().ok());

        repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), diff_opts.as_mut())
            .map_err(|e| format!("Failed to create diff: {}", e))?
    };

    let mut diff_str = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        if let Ok(content) = std::str::from_utf8(line.content()) {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            diff_str.push_str(prefix);
            diff_str.push_str(content);
        }
        true
    })
    .map_err(|e| format!("Failed to print diff: {}", e))?;

    Ok(serde_json::json!({
        "diff": diff_str
    }))
}

/// Get blame information for a file
fn tool_get_file_blame(params: BlameParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let blame = repo
        .blame_file(std::path::Path::new(&params.file_path), None)
        .map_err(|e| format!("Failed to get blame for '{}': {}", params.file_path, e))?;

    let mut hunks = Vec::new();
    for i in 0..blame.len() {
        if let Some(hunk) = blame.get_index(i) {
            // `final_signature()` returns `Option<Signature>` (git2 0.21), so a hunk
            // without a recoverable signature falls back to "Unknown"/epoch 0 rather
            // than panicking.
            let sig = hunk.final_signature();
            let author_name = sig
                .as_ref()
                .and_then(|s| s.name().ok())
                .unwrap_or("Unknown")
                .to_string();
            let date = sig.as_ref().map(|s| s.when().seconds()).unwrap_or(0);
            hunks.push(serde_json::json!({
                "lineStart": hunk.final_start_line(),
                "lineEnd": hunk.final_start_line() + hunk.lines_in_hunk() - 1,
                "author": author_name,
                "commitSha": hunk.final_commit_id().to_string(),
                "date": date
            }));
        }
    }

    Ok(Value::Array(hunks))
}

/// Search commits by message content
fn tool_search_commits(params: SearchCommitsParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let max_results = params.max_results.unwrap_or(20) as usize;
    let query_lower = params.query.to_lowercase();

    let mut revwalk = repo
        .revwalk()
        .map_err(|e| format!("Failed to create revwalk: {}", e))?;

    if revwalk.push_head().is_err() {
        return Ok(Value::Array(Vec::new()));
    }

    revwalk.set_sorting(git2::Sort::TIME).ok();

    let mut results = Vec::new();
    for oid_result in revwalk.take(50_000) {
        if results.len() >= max_results {
            break;
        }

        let oid = match oid_result {
            Ok(oid) => oid,
            Err(_) => continue,
        };

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let message = commit.message().unwrap_or("");
        if message.to_lowercase().contains(&query_lower) {
            let author = commit.author();
            results.push(serde_json::json!({
                "sha": oid.to_string(),
                "message": message.to_string(),
                "author": author.name().unwrap_or("Unknown").to_string(),
                "date": commit.time().seconds()
            }));
        }
    }

    Ok(Value::Array(results))
}

/// Get the list of open repositories
fn tool_get_open_repositories(open_repos: &[String]) -> Result<Value, String> {
    let repos: Vec<Value> = open_repos
        .iter()
        .map(|path| {
            serde_json::json!({
                "path": path
            })
        })
        .collect();

    Ok(Value::Array(repos))
}

/// Human-readable label for a diff delta status
fn delta_status_label(status: git2::Delta) -> &'static str {
    match status {
        git2::Delta::Added => "added",
        git2::Delta::Deleted => "deleted",
        git2::Delta::Modified => "modified",
        git2::Delta::Renamed => "renamed",
        git2::Delta::Copied => "copied",
        git2::Delta::Typechange => "typechange",
        git2::Delta::Untracked => "untracked",
        git2::Delta::Ignored => "ignored",
        _ => "unmodified",
    }
}

/// Build the per-file list (path, oldPath, status, additions, deletions) from a
/// diff with rename/copy detection already applied.
fn diff_files_with_stats(diff: &git2::Diff) -> Result<Vec<Value>, String> {
    let mut files: Vec<Value> = diff
        .deltas()
        .map(|delta| {
            let status = delta_status_label(delta.status());
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let old_path = match delta.status() {
                git2::Delta::Renamed | git2::Delta::Copied => delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().into_owned()),
                _ => None,
            };
            serde_json::json!({ "path": path, "oldPath": old_path, "status": status })
        })
        .collect();

    for i in 0..files.len() {
        if let Ok(Some(patch)) = git2::Patch::from_diff(diff, i) {
            if let Ok((_, additions, deletions)) = patch.line_stats() {
                files[i]["additions"] = serde_json::json!(additions);
                files[i]["deletions"] = serde_json::json!(deletions);
            }
        }
    }
    Ok(files)
}

/// Get full details of a single commit
fn tool_get_commit(params: CommitParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;
    let oid = git2::Oid::from_str(&params.sha)
        .map_err(|_| format!("Invalid commit SHA: {}", params.sha))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|_| format!("Commit not found: {}", params.sha))?;

    // A root commit diffs against the empty tree: `parent(0)` failing is
    // expected there, not an error.
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let commit_tree = commit
        .tree()
        .map_err(|e| format!("Failed to read tree: {}", e))?;

    let mut diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)
        .map_err(|e| format!("Failed to create diff: {}", e))?;
    // Detect renames/copies like `git show` does, so a moved file is one
    // "renamed" entry instead of a delete + add pair.
    let mut find_opts = git2::DiffFindOptions::new();
    find_opts.renames(true).copies(true);
    diff.find_similar(Some(&mut find_opts))
        .map_err(|e| format!("Failed to detect renames: {}", e))?;

    let files = diff_files_with_stats(&diff)?;

    let author = commit.author();
    let committer = commit.committer();
    let parents: Vec<String> = commit.parent_ids().map(|p| p.to_string()).collect();

    Ok(serde_json::json!({
        "sha": oid.to_string(),
        "message": commit.message().unwrap_or(""),
        "author": {
            "name": author.name().ok().unwrap_or("Unknown"),
            "email": author.email().ok().unwrap_or(""),
            "date": author.when().seconds()
        },
        "committer": {
            "name": committer.name().ok().unwrap_or("Unknown"),
            "email": committer.email().ok().unwrap_or(""),
            "date": committer.when().seconds()
        },
        "parents": parents,
        "files": files
    }))
}

/// Get the commits that touched a file
fn tool_get_file_history(params: FileHistoryParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let limit = params.max_results.unwrap_or(100).clamp(1, 500) as usize;
    let should_follow = params.follow_renames.unwrap_or(true);

    let mut revwalk = repo
        .revwalk()
        .map_err(|e| format!("Failed to create revwalk: {}", e))?;
    // TOPOLOGICAL as well as TIME, mirroring the commit-history command: a
    // rename must be visited before the pre-rename history is tested against
    // the new name, and time alone does not guarantee that for same-second
    // commits.
    revwalk
        .set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
        .ok();

    let head = match repo.head() {
        Ok(h) => h,
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => return Ok(Value::Array(Vec::new())),
        Err(e) => return Err(format!("Failed to read HEAD: {}", e)),
    };
    let Some(head_oid) = head.target() else {
        return Ok(Value::Array(Vec::new()));
    };
    revwalk
        .push(head_oid)
        .map_err(|e| format!("Failed to push HEAD: {}", e))?;

    let mut entries: Vec<Value> = Vec::new();
    let mut current_path = params.file_path.clone();

    for oid_result in revwalk.take(MAX_FILE_HISTORY_COMMITS) {
        if entries.len() >= limit {
            break;
        }

        let oid = match oid_result {
            Ok(oid) => oid,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if commit.parent_count() > 1 {
            continue;
        }
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let file_in_commit = tree
            .get_path(std::path::Path::new(&current_path))
            .is_ok();
        if !file_in_commit && !should_follow {
            continue;
        }
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.disable_pathspec_match(true).pathspec(&current_path);
        let diff = match repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            Some(&mut diff_opts),
        ) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let mut file_modified;
        let mut renamed_from: Option<String> = None;

        if should_follow {
            // The filtered diff only holds the Added(current) half at the
            // commit that renamed old -> current, so detect the rename from an
            // UNFILTERED diff, paid once per rename, mirroring the app.
            let introduced_here = diff
                .deltas()
                .any(|d| matches!(d.status(), git2::Delta::Added | git2::Delta::Renamed));
            file_modified = diff.deltas().count() > 0;

            if introduced_here {
                if let Ok(mut unfiltered) =
                    repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
                {
                    let mut find_opts = git2::DiffFindOptions::new();
                    find_opts.renames(true).copies(false);
                    let _ = unfiltered.find_similar(Some(&mut find_opts));
                    for delta in unfiltered.deltas() {
                        if delta.status() != git2::Delta::Renamed {
                            continue;
                        }
                        let is_ours = delta
                            .new_file()
                            .path()
                            .is_some_and(|p| p.to_string_lossy() == current_path);
                        if is_ours {
                            file_modified = true;
                            if let Some(old_file) = delta.old_file().path() {
                                renamed_from = Some(old_file.to_string_lossy().into_owned());
                            }
                            break;
                        }
                    }
                }
            }
        } else {
            file_modified = diff.deltas().count() > 0;
        }

        if file_modified {
            let author = commit.author();
            // Recorded BEFORE the rename is followed: at the renaming commit
            // the file already exists under the new name there.
            entries.push(serde_json::json!({
                "sha": oid.to_string(),
                "path": current_path.clone(),
                "message": commit.summary().ok().flatten().unwrap_or(""),
                "author": author.name().ok().unwrap_or("Unknown"),
                "date": commit.time().seconds()
            }));
            if let Some(old) = renamed_from {
                current_path = old;
            }
        }
    }

    Ok(Value::Array(entries))
}

/// List all tags
fn tool_get_tags(params: RepoParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let mut tags: Vec<Value> = Vec::new();
    repo.tag_foreach(|oid, name| {
        let name_str = String::from_utf8_lossy(name).to_string();
        let short_name = name_str
            .strip_prefix("refs/tags/")
            .unwrap_or(&name_str)
            .to_string();

        let (message, tagger_name, tagger_date, is_annotated, target_oid) =
            if let Ok(tag) = repo.find_tag(oid) {
                let tagger = tag.tagger();
                (
                    tag.message().ok().flatten().map(str::to_string),
                    tagger.as_ref().and_then(|s| s.name().ok()).map(str::to_string),
                    tagger.as_ref().map(|s| s.when().seconds()),
                    true,
                    tag.target_id().to_string(),
                )
            } else {
                (None, None, None, false, oid.to_string())
            };

        tags.push(serde_json::json!({
            "name": short_name,
            "targetOid": target_oid,
            "message": message,
            "tagger": tagger_name,
            "taggerDate": tagger_date,
            "isAnnotated": is_annotated
        }));
        true
    })
    .map_err(|e| format!("Failed to list tags: {}", e))?;

    tags.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    Ok(Value::Array(tags))
}

/// List all stashes
fn tool_get_stashes(params: RepoParams) -> Result<Value, String> {
    let mut repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let mut stashes: Vec<Value> = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        stashes.push(serde_json::json!({
            "index": index,
            "message": message,
            "oid": oid.to_string()
        }));
        true
    })
    .map_err(|e| format!("Failed to list stashes: {}", e))?;

    Ok(Value::Array(stashes))
}

/// List all worktrees, mirroring the `git worktree list --porcelain` parse the
/// worktree command uses.
fn tool_get_worktrees(params: RepoParams) -> Result<Value, String> {
    let output = std::process::Command::new("git")
        .current_dir(&params.repo_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to run git worktree list: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "git worktree list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut worktrees: Vec<Value> = Vec::new();
    let mut current: Option<serde_json::Map<String, Value>> = None;

    for line in text.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(wt) = current.take() {
                worktrees.push(Value::Object(wt));
            }
            let mut wt = serde_json::Map::new();
            wt.insert("path".into(), serde_json::json!(path));
            wt.insert(
                "isMain".into(),
                serde_json::json!(worktrees.is_empty()),
            );
            wt.insert("headOid".into(), Value::Null);
            wt.insert("branch".into(), Value::Null);
            wt.insert("isBare".into(), serde_json::json!(false));
            wt.insert("isDetached".into(), serde_json::json!(false));
            wt.insert("isLocked".into(), serde_json::json!(false));
            wt.insert("lockReason".into(), Value::Null);
            current = Some(wt);
        } else if let Some(wt) = current.as_mut() {
            if let Some(oid) = line.strip_prefix("HEAD ") {
                wt.insert("headOid".into(), serde_json::json!(oid));
            } else if let Some(branch_ref) = line.strip_prefix("branch ") {
                let short = branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref);
                wt.insert("branch".into(), serde_json::json!(short));
            } else if line == "bare" {
                wt.insert("isBare".into(), serde_json::json!(true));
            } else if line == "detached" {
                wt.insert("isDetached".into(), serde_json::json!(true));
            } else if line.starts_with("locked") {
                wt.insert("isLocked".into(), serde_json::json!(true));
                if let Some(reason) = line.strip_prefix("locked ") {
                    wt.insert("lockReason".into(), serde_json::json!(reason));
                }
            }
        }
    }
    if let Some(wt) = current.take() {
        worktrees.push(Value::Object(wt));
    }

    Ok(Value::Array(worktrees))
}

/// List configured remotes
fn tool_get_remotes(params: RepoParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;
    let remotes = repo
        .remotes()
        .map_err(|e| format!("Failed to list remotes: {}", e))?;

    let mut result = Vec::new();
    for name in remotes.iter().filter_map(|s| s.ok().flatten()) {
        if let Ok(remote) = repo.find_remote(name) {
            result.push(serde_json::json!({
                "name": name,
                "url": remote.url().unwrap_or(""),
                "pushUrl": remote.pushurl().ok().flatten().map(str::to_string)
            }));
        }
    }

    Ok(Value::Array(result))
}

/// Get addition/deletion statistics between two refs
fn tool_get_commit_stats(params: CommitStatsParams) -> Result<Value, String> {
    let repo = git2::Repository::open(&params.repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let from_ref = params
        .from_ref
        .as_deref()
        .ok_or("Missing required argument: from_ref")?;
    let to_ref = params.to_ref.as_deref().unwrap_or("HEAD");

    let from_tree = repo
        .revparse_single(from_ref)
        .map_err(|e| format!("Failed to resolve '{}': {}", from_ref, e))?
        .peel_to_tree()
        .map_err(|e| format!("Failed to get tree for '{}': {}", from_ref, e))?;
    let to_tree = repo
        .revparse_single(to_ref)
        .map_err(|e| format!("Failed to resolve '{}': {}", to_ref, e))?
        .peel_to_tree()
        .map_err(|e| format!("Failed to get tree for '{}': {}", to_ref, e))?;

    let diff = repo
        .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)
        .map_err(|e| format!("Failed to create diff: {}", e))?;
    let stats = diff
        .stats()
        .map_err(|e| format!("Failed to read diff stats: {}", e))?;

    Ok(serde_json::json!({
        "from": from_ref,
        "to": to_ref,
        "filesChanged": stats.files_changed(),
        "insertions": stats.insertions(),
        "deletions": stats.deletions()
    }))
}

/// Read a file's content from the working directory
fn tool_get_file_content(params: FileContentParams) -> Result<Value, String> {
    let full_path = crate::commands::path_utils::validate_path_within_repo(
        std::path::Path::new(&params.repo_path),
        &params.file_path,
    )
    .map_err(|e| e.to_string())?;

    let bytes = std::fs::read(&full_path)
        .map_err(|e| format!("Cannot read '{}': {}", params.file_path, e))?;
    let max = params
        .max_bytes
        .unwrap_or(DEFAULT_MAX_FILE_BYTES)
        .clamp(1, MAX_FILE_BYTES_CAP);
    let truncated = bytes.len() > max;
    let content = String::from_utf8_lossy(&bytes[..max.min(bytes.len())]).into_owned();

    Ok(serde_json::json!({
        "path": params.file_path,
        "size": bytes.len(),
        "truncated": truncated,
        "content": content
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestRepo;

    // ========================================================================
    // Tool list tests
    // ========================================================================

    #[test]
    fn test_get_tool_list_returns_all_tools() {
        let tools = get_tool_list();
        assert_eq!(tools.len(), 15);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"get_commit_history"));
        assert!(names.contains(&"get_branches"));
        assert!(names.contains(&"get_status"));
        assert!(names.contains(&"get_diff"));
        assert!(names.contains(&"get_file_blame"));
        assert!(names.contains(&"search_commits"));
        assert!(names.contains(&"get_open_repositories"));
        assert!(names.contains(&"get_commit"));
        assert!(names.contains(&"get_file_history"));
        assert!(names.contains(&"get_tags"));
        assert!(names.contains(&"get_stashes"));
        assert!(names.contains(&"get_worktrees"));
        assert!(names.contains(&"get_remotes"));
        assert!(names.contains(&"get_commit_stats"));
        assert!(names.contains(&"get_file_content"));
    }

    #[test]
    fn test_tool_has_required_fields() {
        let tools = get_tool_list();
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.input_schema.is_object());
            assert_eq!(tool.input_schema["type"], "object");
        }
    }

    #[test]
    fn test_tool_serialization() {
        let tools = get_tool_list();
        let json = serde_json::to_string(&tools).expect("Failed to serialize tools");
        assert!(json.contains("get_commit_history"));
        assert!(json.contains("inputSchema"));
    }

    // ========================================================================
    // get_commit tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_commit_details_with_files() {
        let t = TestRepo::with_initial_commit();
        let first = t.head_oid();
        let second = t.create_commit("Add feature", &[("src/main.rs", "fn main() {}\n")]);

        let result = call_tool(
            "get_commit",
            &serde_json::json!({ "repo_path": t.path_str(), "sha": second.to_string() }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        assert_eq!(result["sha"], second.to_string());
        assert_eq!(result["message"], "Add feature");
        assert_eq!(result["parents"][0], first.to_string());
        assert_eq!(result["files"][0]["path"], "src/main.rs");
        assert_eq!(result["files"][0]["status"], "added");
        assert!(result["files"][0]["additions"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn test_get_commit_unknown_sha() {
        let t = TestRepo::with_initial_commit();
        let err = call_tool(
            "get_commit",
            &serde_json::json!({ "repo_path": t.path_str(), "sha": "0".repeat(40) }),
            &[t.path_str()],
        )
        .await
        .unwrap_err();
        assert!(err.contains("Commit not found"));
    }

    // ========================================================================
    // get_file_history tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_file_history() {
        let t = TestRepo::with_initial_commit();
        t.create_commit("First edit", &[("notes.md", "one\n")]);
        t.create_commit("Second edit", &[("notes.md", "two\n")]);

        let result = call_tool(
            "get_file_history",
            &serde_json::json!({ "repo_path": t.path_str(), "file_path": "notes.md" }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["path"], "notes.md");
        assert_eq!(arr[0]["message"], "Second edit");
        assert_eq!(arr[1]["message"], "First edit");
    }

    #[tokio::test]
    async fn test_get_file_history_follows_rename() {
        let t = TestRepo::with_initial_commit();
        t.create_commit("Create file", &[("old-name.md", "content\n")]);

        // A real rename: the old path leaves the tree and the new one enters it
        // in the same commit, so rename detection has both halves to pair.
        let repo = t.repo();
        t.create_file("new-name.md", "content\n");
        std::fs::remove_file(t.path.join("old-name.md")).expect("Failed to remove old file");
        let mut index = repo.index().expect("Failed to read index");
        index
            .remove_path(std::path::Path::new("old-name.md"))
            .expect("Failed to remove from index");
        index
            .add_path(std::path::Path::new("new-name.md"))
            .expect("Failed to add to index");
        index.write().expect("Failed to write index");
        let tree_oid = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_oid).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");
        let parent = repo
            .head()
            .expect("Failed to read HEAD")
            .peel_to_commit()
            .expect("Failed to peel HEAD");
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Rename file",
            &tree,
            &[&parent],
        )
        .expect("Failed to commit rename");

        let result = call_tool(
            "get_file_history",
            &serde_json::json!({ "repo_path": t.path_str(), "file_path": "new-name.md" }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["path"], "new-name.md");
        assert_eq!(arr[1]["path"], "old-name.md");
    }

    // ========================================================================
    // get_tags / get_stashes / get_remotes tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_tags_annotated_and_lightweight() {
        let t = TestRepo::with_initial_commit();
        t.create_tag("v1.0");
        t.create_lightweight_tag("v0.9");

        let result = call_tool(
            "get_tags",
            &serde_json::json!({ "repo_path": t.path_str() }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        // Sorted by name: v0.9 before v1.0
        assert_eq!(arr[0]["name"], "v0.9");
        assert_eq!(arr[0]["isAnnotated"], false);
        assert_eq!(arr[1]["name"], "v1.0");
        assert_eq!(arr[1]["isAnnotated"], true);
        assert!(arr[1]["message"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_get_stashes() {
        let t = TestRepo::with_initial_commit();
        t.create_file("wip.txt", "work in progress\n");
        // Stash does not include untracked files by default, so stage first.
        t.stage_file("wip.txt");
        let mut repo = t.repo();
        let sig = repo.signature().expect("Failed to get signature");
        repo.stash_save(&sig, "WIP from test", None)
            .expect("Failed to stash");
        drop(repo);

        let result = call_tool(
            "get_stashes",
            &serde_json::json!({ "repo_path": t.path_str() }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["index"], 0);
        // git prefixes the message with "On <branch>: "
        assert!(arr[0]["message"].as_str().unwrap().contains("WIP from test"));
    }

    #[tokio::test]
    async fn test_get_remotes() {
        let t = TestRepo::with_initial_commit();
        t.add_remote("origin", "https://example.com/repo.git");

        let result = call_tool(
            "get_remotes",
            &serde_json::json!({ "repo_path": t.path_str() }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "origin");
        assert_eq!(arr[0]["url"], "https://example.com/repo.git");
    }

    // ========================================================================
    // get_worktrees tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_worktrees_main_and_linked() {
        let t = TestRepo::with_initial_commit();
        t.create_branch("feature");
        let output = std::process::Command::new("git")
            .current_dir(t.path_str())
            .args(["worktree", "add", "wt-feature", "feature"])
            .output()
            .expect("Failed to run git worktree add");
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let result = call_tool(
            "get_worktrees",
            &serde_json::json!({ "repo_path": t.path_str() }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let main = arr.iter().find(|w| w["isMain"] == true).unwrap();
        assert_eq!(main["headOid"], t.head_oid().to_string());
        assert_eq!(main["branch"], "main");
        let linked = arr.iter().find(|w| w["isMain"] == false).unwrap();
        assert_eq!(linked["branch"], "feature");
    }

    // ========================================================================
    // get_commit_stats tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_commit_stats_between_refs() {
        let t = TestRepo::with_initial_commit();
        let first = t.head_oid();
        let second = t.create_commit("Add feature", &[("f.txt", "hello\n")]);

        let result = call_tool(
            "get_commit_stats",
            &serde_json::json!({
                "repo_path": t.path_str(),
                "from_ref": first.to_string(),
                "to_ref": second.to_string()
            }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        assert_eq!(result["filesChanged"], 1);
        assert_eq!(result["insertions"], 1);
        assert_eq!(result["from"], first.to_string());
        assert_eq!(result["to"], second.to_string());
    }

    #[tokio::test]
    async fn test_get_commit_stats_requires_from_ref() {
        let t = TestRepo::with_initial_commit();
        let err = call_tool(
            "get_commit_stats",
            &serde_json::json!({ "repo_path": t.path_str() }),
            &[t.path_str()],
        )
        .await
        .unwrap_err();
        assert!(err.contains("from_ref"));
    }

    // ========================================================================
    // get_file_content tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_file_content() {
        let t = TestRepo::with_initial_commit();
        t.create_file("notes.txt", "hello world\n");

        let result = call_tool(
            "get_file_content",
            &serde_json::json!({ "repo_path": t.path_str(), "file_path": "notes.txt" }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        assert_eq!(result["content"], "hello world\n");
        assert_eq!(result["truncated"], false);
        assert_eq!(result["size"], 12);
    }

    #[tokio::test]
    async fn test_get_file_content_truncates_at_max_bytes() {
        let t = TestRepo::with_initial_commit();
        t.create_file("big.txt", "abcdefghij\n");

        let result = call_tool(
            "get_file_content",
            &serde_json::json!({ "repo_path": t.path_str(), "file_path": "big.txt", "max_bytes": 4 }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        assert_eq!(result["content"], "abcd");
        assert_eq!(result["truncated"], true);
        assert_eq!(result["size"], 11);
    }

    #[tokio::test]
    async fn test_get_file_content_rejects_traversal() {
        let t = TestRepo::with_initial_commit();
        let err = call_tool(
            "get_file_content",
            &serde_json::json!({ "repo_path": t.path_str(), "file_path": "../../outside.txt" }),
            &[t.path_str()],
        )
        .await
        .unwrap_err();
        assert!(err.contains("relative") || err.contains("outside"));
    }

    #[tokio::test]
    async fn test_get_file_content_missing_file() {
        let t = TestRepo::with_initial_commit();
        let err = call_tool(
            "get_file_content",
            &serde_json::json!({ "repo_path": t.path_str(), "file_path": "nope.txt" }),
            &[t.path_str()],
        )
        .await
        .unwrap_err();
        assert!(err.contains("Cannot read"));
    }

    // ========================================================================
    // get_diff with file_path tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_diff_with_file_path() {
        let t = TestRepo::with_initial_commit();
        t.create_commit("Add files", &[("a.txt", "alpha\n"), ("b.txt", "beta\n")]);
        t.create_file("a.txt", "alpha changed\n");

        let result = call_tool(
            "get_diff",
            &serde_json::json!({ "repo_path": t.path_str(), "file_path": "a.txt" }),
            &[t.path_str()],
        )
        .await
        .unwrap();
        let diff = result["diff"].as_str().unwrap();
        assert!(diff.contains("a.txt"));
        assert!(!diff.contains("b.txt"));
    }

    // ========================================================================
    // get_open_repositories tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_open_repositories_empty() {
        let result = call_tool("get_open_repositories", &serde_json::json!({}), &[]).await;
        assert!(result.is_ok());
        let repos = result.unwrap();
        assert!(repos.is_array());
        assert!(repos.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_open_repositories_with_repos() {
        let open = vec!["/path/to/repo1".to_string(), "/path/to/repo2".to_string()];
        let result = call_tool("get_open_repositories", &serde_json::json!({}), &open).await;
        assert!(result.is_ok());
        let repos = result.unwrap();
        let arr = repos.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["path"], "/path/to/repo1");
        assert_eq!(arr[1]["path"], "/path/to/repo2");
    }

    // ========================================================================
    // Unknown tool tests
    // ========================================================================

    #[tokio::test]
    async fn test_unknown_tool() {
        let result = call_tool("nonexistent_tool", &serde_json::json!({}), &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    // ========================================================================
    // get_commit_history tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_commit_history() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_commit("Second commit", &[("file.txt", "content")]);
        test_repo.create_commit("Third commit", &[("file2.txt", "content2")]);

        let result = call_tool(
            "get_commit_history",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "count": 10
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let commits = result.unwrap();
        let arr = commits.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        // Most recent first
        assert!(arr[0]["message"].as_str().unwrap().contains("Third commit"));
    }

    #[tokio::test]
    async fn test_get_commit_history_with_limit() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_commit("Second commit", &[("file.txt", "content")]);
        test_repo.create_commit("Third commit", &[("file2.txt", "content2")]);

        let result = call_tool(
            "get_commit_history",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "count": 1
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let commits = result.unwrap();
        assert_eq!(commits.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_commit_history_invalid_repo() {
        let result = call_tool(
            "get_commit_history",
            &serde_json::json!({
                "repo_path": "/nonexistent/repo"
            }),
            &[],
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_commit_history_with_branch() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_branch("feature");
        test_repo.checkout_branch("feature");
        test_repo.create_commit("Feature commit", &[("feature.txt", "feature content")]);

        let result = call_tool(
            "get_commit_history",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "branch": "feature"
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let commits = result.unwrap();
        let arr = commits.as_array().unwrap();
        assert_eq!(arr.len(), 2); // feature commit + initial
        assert!(arr[0]["message"]
            .as_str()
            .unwrap()
            .contains("Feature commit"));
    }

    // ========================================================================
    // get_branches tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_branches() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_branch("feature");
        test_repo.create_branch("develop");

        let result = call_tool(
            "get_branches",
            &serde_json::json!({
                "repo_path": test_repo.path_str()
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let branches = result.unwrap();
        let arr = branches.as_array().unwrap();
        assert!(arr.len() >= 3); // main, feature, develop

        // Find main and check it's current
        let main_branch = arr.iter().find(|b| b["name"] == "main");
        assert!(main_branch.is_some());
        assert!(main_branch.unwrap()["isCurrent"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_get_branches_invalid_repo() {
        let result = call_tool(
            "get_branches",
            &serde_json::json!({
                "repo_path": "/nonexistent/repo"
            }),
            &[],
        )
        .await;

        assert!(result.is_err());
    }

    // ========================================================================
    // get_status tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_status_clean() {
        let test_repo = TestRepo::with_initial_commit();

        let result = call_tool(
            "get_status",
            &serde_json::json!({
                "repo_path": test_repo.path_str()
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status["staged"].as_array().unwrap().is_empty());
        assert!(status["modified"].as_array().unwrap().is_empty());
        assert!(status["untracked"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_status_with_changes() {
        let test_repo = TestRepo::with_initial_commit();

        // Create an untracked file
        test_repo.create_file("untracked.txt", "untracked content");

        // Create and stage a file
        test_repo.create_file("staged.txt", "staged content");
        test_repo.stage_file("staged.txt");

        // Modify an existing file
        test_repo.create_file("README.md", "modified content");

        let result = call_tool(
            "get_status",
            &serde_json::json!({
                "repo_path": test_repo.path_str()
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let status = result.unwrap();

        let staged: Vec<&str> = status["staged"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(staged.contains(&"staged.txt"));

        let untracked: Vec<&str> = status["untracked"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(untracked.contains(&"untracked.txt"));
    }

    // ========================================================================
    // get_diff tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_diff_working_directory() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_file("README.md", "modified content");

        let result = call_tool(
            "get_diff",
            &serde_json::json!({
                "repo_path": test_repo.path_str()
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let diff_result = result.unwrap();
        let diff_str = diff_result["diff"].as_str().unwrap();
        assert!(diff_str.contains("-# Test Repo"));
        assert!(diff_str.contains("+modified content"));
    }

    #[tokio::test]
    async fn test_get_diff_no_changes() {
        let test_repo = TestRepo::with_initial_commit();

        let result = call_tool(
            "get_diff",
            &serde_json::json!({
                "repo_path": test_repo.path_str()
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let diff_result = result.unwrap();
        let diff_str = diff_result["diff"].as_str().unwrap();
        assert!(diff_str.is_empty());
    }

    #[tokio::test]
    async fn test_get_diff_between_refs() {
        let test_repo = TestRepo::with_initial_commit();
        let first_sha = test_repo.head_oid().to_string();

        test_repo.create_commit("Second commit", &[("new_file.txt", "new content")]);
        let second_sha = test_repo.head_oid().to_string();

        let result = call_tool(
            "get_diff",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "from_ref": first_sha,
                "to_ref": second_sha
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let diff_result = result.unwrap();
        let diff_str = diff_result["diff"].as_str().unwrap();
        assert!(diff_str.contains("+new content"));
    }

    // ========================================================================
    // get_file_blame tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_file_blame() {
        let test_repo = TestRepo::with_initial_commit();

        let result = call_tool(
            "get_file_blame",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "file_path": "README.md"
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let hunks = result.unwrap();
        let arr = hunks.as_array().unwrap();
        assert!(!arr.is_empty());
        assert_eq!(arr[0]["author"], "Test User");
        assert!(arr[0]["commitSha"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_get_file_blame_nonexistent_file() {
        let test_repo = TestRepo::with_initial_commit();

        let result = call_tool(
            "get_file_blame",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "file_path": "nonexistent.txt"
            }),
            &[],
        )
        .await;

        assert!(result.is_err());
    }

    // ========================================================================
    // search_commits tests
    // ========================================================================

    #[tokio::test]
    async fn test_search_commits() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_commit("feat: add login page", &[("login.txt", "login")]);
        test_repo.create_commit("fix: resolve crash on startup", &[("fix.txt", "fix")]);
        test_repo.create_commit("feat: add dashboard", &[("dash.txt", "dashboard")]);

        let result = call_tool(
            "search_commits",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "query": "feat"
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let results = result.unwrap();
        let arr = results.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[tokio::test]
    async fn test_search_commits_case_insensitive() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_commit("FEAT: add feature", &[("f.txt", "content")]);

        let result = call_tool(
            "search_commits",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "query": "feat"
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_search_commits_no_results() {
        let test_repo = TestRepo::with_initial_commit();

        let result = call_tool(
            "search_commits",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "query": "nonexistent_query_xyz"
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert!(results.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_search_commits_with_max_results() {
        let test_repo = TestRepo::with_initial_commit();
        test_repo.create_commit("feat: one", &[("a.txt", "a")]);
        test_repo.create_commit("feat: two", &[("b.txt", "b")]);
        test_repo.create_commit("feat: three", &[("c.txt", "c")]);

        let result = call_tool(
            "search_commits",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "query": "feat",
                "max_results": 2
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.as_array().unwrap().len(), 2);
    }

    // ========================================================================
    // Invalid arguments tests
    // ========================================================================

    // ========================================================================
    // Path validation tests
    // ========================================================================

    #[test]
    fn test_validate_repo_path_relative_rejected() {
        let result = validate_repo_path("../some/repo", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute"));
    }

    #[test]
    fn test_validate_repo_path_nonexistent_rejected() {
        // Built from the platform's temp dir rather than hardcoded as
        // "/nonexistent/...": a leading-slash-only path has no drive prefix, so
        // `Path::is_absolute` is false for it on Windows and the earlier
        // "must be absolute" arm answered instead of the one under test.
        let missing = std::env::temp_dir().join("gitduda-definitely-nonexistent-repo");
        let result = validate_repo_path(&missing.to_string_lossy(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_validate_repo_path_not_in_open_repos() {
        let test_repo = TestRepo::with_initial_commit();
        let open_repos = vec!["/some/other/repo".to_string()];
        let result = validate_repo_path(&test_repo.path_str(), &open_repos);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));
    }

    #[test]
    fn test_validate_repo_path_in_open_repos() {
        let test_repo = TestRepo::with_initial_commit();
        let path = test_repo.path_str();
        let open_repos = vec![path.clone()];
        let result = validate_repo_path(&path, &open_repos);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_repo_path_empty_open_repos_allows_valid_repo() {
        let test_repo = TestRepo::with_initial_commit();
        let result = validate_repo_path(&test_repo.path_str(), &[]);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Invalid arguments tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_commit_history_with_symbolic_head_and_annotated_tag() {
        let test_repo = TestRepo::with_initial_commit();
        let commit_oid = test_repo.create_commit("Tagged commit", &[("tag.txt", "tagged")]);

        // Create an annotated tag pointing at the commit
        {
            let repo = test_repo.repo();
            let obj = repo.find_object(commit_oid, None).unwrap();
            let sig = repo.signature().unwrap();
            repo.tag("v1.0.0", &obj, &sig, "Release v1.0.0", false)
                .unwrap();
        }

        // Resolving symbolic ref "HEAD"
        let head_result = call_tool(
            "get_commit_history",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "branch": "HEAD"
            }),
            &[],
        )
        .await;
        assert!(head_result.is_ok(), "{:?}", head_result);
        assert_eq!(head_result.unwrap().as_array().unwrap().len(), 2);

        // Resolving annotated tag "v1.0.0"
        let tag_result = call_tool(
            "get_commit_history",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "branch": "v1.0.0"
            }),
            &[],
        )
        .await;
        assert!(tag_result.is_ok(), "{:?}", tag_result);
        assert_eq!(tag_result.unwrap().as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_search_commits_unborn_head_returns_empty() {
        let test_repo = TestRepo::new();

        let result = call_tool(
            "search_commits",
            &serde_json::json!({
                "repo_path": test_repo.path_str(),
                "query": "feat"
            }),
            &[],
        )
        .await;

        assert!(result.is_ok());
        assert!(result.unwrap().as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_call_tool_invalid_arguments() {
        let result = call_tool(
            "get_commit_history",
            &serde_json::json!({"wrong_field": 123}),
            &[],
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid arguments"));
    }
}

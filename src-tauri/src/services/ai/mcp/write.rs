//! MCP write tools behind a consent gate (roadmap 4.2, issue #610).
//!
//! Agents connected over MCP may change a repository only through these
//! tools — `stage`, `commit`, `create_branch`, `create_worktree`, `stash`,
//! `checkout` — and every call must pass the gate:
//!
//! 1. Write tools are **off** until the user turns them on (they are not even
//!    listed by `tools/list` while off).
//! 2. A per-repository **policy** may pre-approve a tool on branches matching a
//!    glob ("agents may `commit` on `agent/*`"). The branch is the one the
//!    write lands on: the current branch for stage/commit/stash, the new or
//!    target branch for create_branch/create_worktree/checkout.
//! 3. Otherwise Gitduda shows the request with a preview and waits for the
//!    user. No answer within the timeout is a denial, as is having no UI to
//!    ask (e.g. headless tests).
//!
//! Every applied write is announced (`mcp-write-applied`) so the UI refreshes
//! and the user sees what the agent did.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::tools::McpTool;

const WRITE_SETTINGS_FILE: &str = "mcp-write.json";
/// How long a consent request waits for the user before it is denied.
pub const DEFAULT_CONSENT_TIMEOUT: Duration = Duration::from_secs(120);

pub const CONSENT_REQUEST_EVENT: &str = "mcp-consent-request";
pub const CONSENT_RESOLVED_EVENT: &str = "mcp-consent-resolved";
pub const WRITE_APPLIED_EVENT: &str = "mcp-write-applied";

pub const WRITE_TOOLS: [&str; 16] = [
    "stage",
    "commit",
    "create_branch",
    "create_worktree",
    "stash",
    "checkout",
    "unstage",
    "write_file_content",
    "discard_changes",
    "apply_stash",
    "drop_stash",
    "delete_branch",
    "create_tag",
    "delete_tag",
    "reset",
    "revert_commit",
];

pub fn is_write_tool(name: &str) -> bool {
    WRITE_TOOLS.contains(&name)
}

/// A standing permission: `tool` may run without asking in `repo_path` when
/// the affected branch matches `branch_pattern`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WritePolicy {
    pub repo_path: String,
    pub tool: String,
    pub branch_pattern: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WriteSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub policies: Vec<WritePolicy>,
}

/// What the user is asked to approve.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRequest {
    pub id: u64,
    pub tool: String,
    pub repo_path: String,
    /// The branch the write lands on, when there is one.
    pub branch: Option<String>,
    pub summary: String,
    /// Preview lines (files, message, paths).
    pub details: Vec<String>,
    /// Unix seconds.
    pub requested_at: u64,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Approved by the user.
    Approved,
    /// Allowed by a saved policy without asking.
    AllowedByPolicy,
    Denied,
    TimedOut,
}

/// Sends an event to the UI: (event name, payload).
pub type Emitter = Arc<dyn Fn(&str, Value) + Send + Sync>;

/// Minimal glob: `*` matches any run of characters (including `/`), `?` one.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn canonical_string(path: &str) -> String {
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
}

/// Whether a saved policy pre-approves this request.
pub fn policy_allows(
    policies: &[WritePolicy],
    tool: &str,
    repo_path: &str,
    branch: Option<&str>,
) -> bool {
    let Some(branch) = branch else {
        // Detached HEAD / unborn branch: nothing to match a pattern against.
        return false;
    };
    let repo = canonical_string(repo_path);
    policies.iter().any(|p| {
        p.tool == tool
            && canonical_string(&p.repo_path) == repo
            && glob_match(&p.branch_pattern, branch)
    })
}

struct Pending {
    request: ConsentRequest,
    reply: tokio::sync::oneshot::Sender<bool>,
}

/// Write-tool settings, policies and the consent broker, shared by the MCP
/// server and the Tauri commands.
pub struct WriteGate {
    config_dir: Option<PathBuf>,
    settings: StdRwLock<WriteSettings>,
    pending: Mutex<HashMap<u64, Pending>>,
    next_id: AtomicU64,
    emitter: StdRwLock<Option<Emitter>>,
    timeout: Duration,
}

impl WriteGate {
    /// Load persisted settings from `config_dir` (defaults when absent).
    pub fn load(config_dir: &Path) -> Self {
        let settings = std::fs::read_to_string(config_dir.join(WRITE_SETTINGS_FILE))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self::with(
            Some(config_dir.to_path_buf()),
            settings,
            DEFAULT_CONSENT_TIMEOUT,
        )
    }

    /// An in-memory gate (tests).
    pub fn in_memory(settings: WriteSettings, timeout: Duration) -> Self {
        Self::with(None, settings, timeout)
    }

    fn with(config_dir: Option<PathBuf>, settings: WriteSettings, timeout: Duration) -> Self {
        Self {
            config_dir,
            settings: StdRwLock::new(settings),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            emitter: StdRwLock::new(None),
            timeout,
        }
    }

    pub fn set_emitter(&self, emitter: Emitter) {
        *self.emitter.write().unwrap_or_else(|e| e.into_inner()) = Some(emitter);
    }

    fn emit(&self, event: &str, payload: Value) -> bool {
        let guard = self.emitter.read().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(emit) => {
                emit(event, payload);
                true
            }
            None => false,
        }
    }

    pub fn settings(&self) -> WriteSettings {
        self.settings
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn enabled(&self) -> bool {
        self.settings
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .enabled
    }

    /// Replace and persist the settings. Turning write tools off denies every
    /// request still waiting for an answer.
    pub fn set_settings(&self, settings: WriteSettings) -> Result<(), String> {
        if let Some(dir) = &self.config_dir {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
            let text = serde_json::to_string_pretty(&settings)
                .map_err(|e| format!("Failed to serialize MCP write settings: {e}"))?;
            std::fs::write(dir.join(WRITE_SETTINGS_FILE), text)
                .map_err(|e| format!("Failed to save MCP write settings: {e}"))?;
        }
        let disabled = !settings.enabled;
        *self.settings.write().unwrap_or_else(|e| e.into_inner()) = settings;
        if disabled {
            let ids: Vec<u64> = self.pending_requests().iter().map(|r| r.id).collect();
            for id in ids {
                let _ = self.respond(id, false, None);
            }
        }
        Ok(())
    }

    pub fn pending_requests(&self) -> Vec<ConsentRequest> {
        let mut out: Vec<ConsentRequest> = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(|p| p.request.clone())
            .collect();
        out.sort_by_key(|r| r.id);
        out
    }

    /// Answer a waiting request. With `always_allow_pattern`, approving also
    /// saves a policy for this tool and repository.
    pub fn respond(
        &self,
        id: u64,
        approve: bool,
        always_allow_pattern: Option<String>,
    ) -> Result<(), String> {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id)
            .ok_or_else(|| "This request is no longer waiting for an answer".to_string())?;
        if approve {
            if let Some(pattern) = always_allow_pattern
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
            {
                let mut settings = self.settings();
                let policy = WritePolicy {
                    repo_path: pending.request.repo_path.clone(),
                    tool: pending.request.tool.clone(),
                    branch_pattern: pattern,
                };
                if !settings.policies.contains(&policy) {
                    settings.policies.push(policy);
                    self.set_settings(settings)?;
                }
            }
        }
        let _ = pending.reply.send(approve);
        self.emit(
            CONSENT_RESOLVED_EVENT,
            json!({ "id": id, "approved": approve }),
        );
        Ok(())
    }

    /// Decide whether a write may proceed: policy, else ask the user.
    pub async fn authorize(&self, tool: &str, prepared: &PreparedWrite) -> Decision {
        let settings = self.settings();
        if policy_allows(
            &settings.policies,
            tool,
            &prepared.repo_path,
            prepared.branch.as_deref(),
        ) {
            return Decision::AllowedByPolicy;
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = ConsentRequest {
            id,
            tool: tool.to_string(),
            repo_path: prepared.repo_path.clone(),
            branch: prepared.branch.clone(),
            summary: prepared.summary.clone(),
            details: prepared.details.clone(),
            requested_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            timeout_secs: self.timeout.as_secs(),
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                id,
                Pending {
                    request: request.clone(),
                    reply: tx,
                },
            );
        let payload = serde_json::to_value(&request).unwrap_or(Value::Null);
        if !self.emit(CONSENT_REQUEST_EVENT, payload) {
            // Nobody can be asked: deny rather than wait out the timeout.
            self.pending
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id);
            return Decision::Denied;
        }
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(true)) => Decision::Approved,
            Ok(_) => Decision::Denied,
            Err(_) => {
                self.pending
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&id);
                self.emit(
                    CONSENT_RESOLVED_EVENT,
                    json!({ "id": id, "approved": false, "expired": true }),
                );
                Decision::TimedOut
            }
        }
    }

    pub fn notify_applied(&self, tool: &str, repo_path: &str, summary: &str, by_policy: bool) {
        self.emit(
            WRITE_APPLIED_EVENT,
            json!({ "tool": tool, "repoPath": repo_path, "summary": summary, "byPolicy": by_policy }),
        );
    }
}

/// A validated write, described for the consent prompt.
#[derive(Debug, Clone)]
pub struct PreparedWrite {
    pub repo_path: String,
    pub branch: Option<String>,
    pub summary: String,
    pub details: Vec<String>,
    action: Action,
}

#[derive(Debug, Clone)]
enum Action {
    Stage(Vec<String>),
    Commit(String),
    CreateBranch {
        name: String,
        start: Option<String>,
    },
    Checkout(String),
    Stash {
        message: Option<String>,
        include_untracked: bool,
    },
    CreateWorktree {
        path: PathBuf,
        branch: String,
        start: Option<String>,
    },
    Unstage(Vec<String>),
    WriteFile {
        file_path: String,
        content: String,
        stage_after: bool,
    },
    Discard(Vec<String>),
    ApplyStash {
        index: usize,
        drop_after: bool,
    },
    DropStash {
        index: usize,
    },
    DeleteBranch {
        name: String,
        force: bool,
    },
    CreateTag {
        name: String,
        target_oid: String,
        target_ref: Option<String>,
        message: Option<String>,
    },
    DeleteTag {
        name: String,
    },
    Reset {
        target_oid: String,
        mode: String,
    },
    RevertCommit {
        sha: String,
        mainline: Option<u32>,
    },
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    str_arg(args, key).ok_or_else(|| format!("Missing required argument: {key}"))
}

fn current_branch(repo: &git2::Repository) -> Option<String> {
    repo.head()
        .ok()
        .filter(|h| h.is_branch())
        .and_then(|h| h.shorthand().ok().map(str::to_string))
}

fn validate_branch_name(name: &str) -> Result<(), String> {
    if name.starts_with('-') || !git2::Branch::name_is_valid(name).unwrap_or(false) {
        return Err(format!("Invalid branch name: {name}"));
    }
    Ok(())
}

fn validate_rel_path(p: &str) -> Result<(), String> {
    let path = Path::new(p);
    if path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        return Err(format!(
            "Paths must be relative to the repository root: {p}"
        ));
    }
    if path
        .components()
        .next()
        .is_some_and(|c| c.as_os_str() == ".git")
    {
        return Err(format!("Refusing to touch the git directory: {p}"));
    }
    Ok(())
}

fn staged_files(repo: &git2::Repository) -> Result<Vec<String>, String> {
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let index = repo.index().map_err(|e| e.to_string())?;
    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), Some(&index), None)
        .map_err(|e| e.to_string())?;
    Ok(diff
        .deltas()
        .filter_map(|d| {
            d.new_file()
                .path()
                .or_else(|| d.old_file().path())
                .map(|p| p.to_string_lossy().into_owned())
        })
        .collect())
}

/// Extract a non-empty list of relative path strings from `args[key]`.
fn paths_arg(args: &Value, key: &str) -> Result<Vec<String>, String> {
    let list: Vec<String> = args
        .get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if list.is_empty() {
        Err(format!("Missing required argument: {key} (a non-empty list)"))
    } else {
        Ok(list)
    }
}

/// Extract an optional non-negative integer from `args[key]`.
fn usize_arg(args: &Value, key: &str, default: usize) -> usize {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(default)
}

/// The message of the stash at `index`, or an error naming the hole.
fn stash_message(repo: &mut git2::Repository, index: usize) -> Result<String, String> {
    let mut found = None;
    repo.stash_foreach(|i, message, _| {
        if i == index {
            found = Some(message.to_string());
            return false;
        }
        true
    })
    .map_err(|e| e.to_string())?;
    found.ok_or_else(|| format!("No stash at index {index}"))
}

/// Validate a write tool call and describe it, without changing anything.
pub fn prepare(tool: &str, args: &Value, repo_path: &str) -> Result<PreparedWrite, String> {
    let mut repo = git2::Repository::open(repo_path)
        .map_err(|_| format!("Not a valid git repository: {repo_path}"))?;
    if repo.is_bare() {
        return Err("Write tools need a working tree".to_string());
    }
    let current = current_branch(&repo);
    let (branch, summary, details, action) = match tool {
        "stage" => {
            let paths: Vec<String> = args
                .get("paths")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            if paths.is_empty() {
                return Err("Missing required argument: paths (a non-empty list)".to_string());
            }
            for p in &paths {
                validate_rel_path(p)?;
            }
            let summary = format!(
                "Stage {} file{}",
                paths.len(),
                if paths.len() == 1 { "" } else { "s" }
            );
            (
                current.clone(),
                summary,
                paths.clone(),
                Action::Stage(paths),
            )
        }
        "commit" => {
            let message = required(args, "message")?.to_string();
            let files = staged_files(&repo)?;
            if files.is_empty() {
                return Err("Nothing is staged to commit".to_string());
            }
            let first = message.lines().next().unwrap_or("").to_string();
            let mut details = vec![format!("Message: {first}")];
            details.extend(files.iter().map(|f| format!("  {f}")));
            let summary = format!(
                "Commit {} staged file{} on {}",
                files.len(),
                if files.len() == 1 { "" } else { "s" },
                current.as_deref().unwrap_or("a detached HEAD")
            );
            (current.clone(), summary, details, Action::Commit(message))
        }
        "create_branch" => {
            let name = required(args, "name")?.to_string();
            validate_branch_name(&name)?;
            if repo.find_branch(&name, git2::BranchType::Local).is_ok() {
                return Err(format!("Branch already exists: {name}"));
            }
            let start = str_arg(args, "start_point").map(str::to_string);
            let summary = format!(
                "Create branch {name} from {}",
                start.as_deref().unwrap_or("HEAD")
            );
            (
                Some(name.clone()),
                summary,
                Vec::new(),
                Action::CreateBranch { name, start },
            )
        }
        "checkout" => {
            let target = required(args, "branch")?.to_string();
            repo.find_branch(&target, git2::BranchType::Local)
                .map_err(|_| format!("No local branch named {target}"))?;
            let summary = format!("Switch the working tree to {target}");
            (
                Some(target.clone()),
                summary,
                Vec::new(),
                Action::Checkout(target),
            )
        }
        "stash" => {
            let message = str_arg(args, "message").map(str::to_string);
            let include_untracked = args
                .get("include_untracked")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut opts = git2::StatusOptions::new();
            opts.include_untracked(include_untracked)
                .recurse_untracked_dirs(true);
            let changed = repo.statuses(Some(&mut opts)).map(|s| s.len()).unwrap_or(0);
            if changed == 0 {
                return Err("There are no changes to stash".to_string());
            }
            let summary = format!(
                "Stash {changed} changed file{}",
                if changed == 1 { "" } else { "s" }
            );
            let details = message.iter().map(|m| format!("Message: {m}")).collect();
            (
                current.clone(),
                summary,
                details,
                Action::Stash {
                    message,
                    include_untracked,
                },
            )
        }
        "create_worktree" => {
            let path = PathBuf::from(required(args, "path")?);
            if !path.is_absolute() {
                return Err("Worktree path must be absolute".to_string());
            }
            if path.exists() {
                return Err(format!("Path already exists: {}", path.display()));
            }
            let branch = required(args, "branch")?.to_string();
            validate_branch_name(&branch)?;
            if repo.find_branch(&branch, git2::BranchType::Local).is_ok() {
                return Err(format!("Branch already exists: {branch}"));
            }
            let start = str_arg(args, "start_point").map(str::to_string);
            let summary = format!("Create worktree on new branch {branch}");
            let details = vec![
                format!("Path: {}", path.display()),
                format!("From: {}", start.as_deref().unwrap_or("HEAD")),
            ];
            (
                Some(branch.clone()),
                summary,
                details,
                Action::CreateWorktree {
                    path,
                    branch,
                    start,
                },
            )
        }
        "unstage" => {
            let paths = paths_arg(args, "paths")?;
            for p in &paths {
                validate_rel_path(p)?;
            }
            let summary = format!(
                "Unstage {} file{}",
                paths.len(),
                if paths.len() == 1 { "" } else { "s" }
            );
            (
                current.clone(),
                summary,
                paths.clone(),
                Action::Unstage(paths),
            )
        }
        "write_file_content" => {
            let file_path = required(args, "file_path")?.to_string();
            validate_rel_path(&file_path)?;
            let content = args
                .get("content")
                .and_then(Value::as_str)
                .ok_or("Missing required argument: content")?
                .to_string();
            let stage_after = args
                .get("stage_after")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let summary = format!("Write {} bytes to {file_path}", content.len());
            let mut details = vec![file_path.clone()];
            if stage_after {
                details.push("Stage the file after writing".to_string());
            }
            (
                current.clone(),
                summary,
                details,
                Action::WriteFile {
                    file_path,
                    content,
                    stage_after,
                },
            )
        }
        "discard_changes" => {
            let paths = paths_arg(args, "paths")?;
            for p in &paths {
                validate_rel_path(p)?;
            }
            // Classify up front so the consent prompt is concrete, and check
            // nested repositories before anything is described as doable —
            // the same all-or-nothing shape the discard command uses.
            let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
            let index = repo.index().map_err(|e| e.to_string())?;
            let mut details = Vec::new();
            for p in &paths {
                let path_obj = Path::new(p);
                let in_head = head_tree
                    .as_ref()
                    .map(|t| t.get_path(path_obj).is_ok())
                    .unwrap_or(false);
                let in_index = index.get_path(path_obj, 0).is_some();
                if in_index {
                    details.push(format!("Restore from the index: {p}"));
                } else if !in_head {
                    details.push(format!("Delete untracked: {p}"));
                    let full = crate::commands::path_utils::validate_path_within_repo(
                        Path::new(repo_path),
                        p,
                    )
                    .map_err(|e| e.to_string())?;
                    let is_dir = std::fs::symlink_metadata(&full)
                        .map(|m| m.file_type().is_dir())
                        .unwrap_or(false);
                    if is_dir && crate::utils::contains_nested_repo(&full) {
                        return Err(format!(
                            "'{}' contains a git repository — discarding it would destroy its \
                             history. Nothing was discarded.",
                            full.display()
                        ));
                    }
                } else {
                    details.push(format!("No working-tree change to discard: {p}"));
                }
            }
            let summary = format!(
                "Discard changes to {} file{} (uncommitted work is lost)",
                paths.len(),
                if paths.len() == 1 { "" } else { "s" }
            );
            (
                current.clone(),
                summary,
                details,
                Action::Discard(paths),
            )
        }
        "apply_stash" => {
            let index = usize_arg(args, "index", 0);
            let drop_after = args
                .get("drop_after")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let message = stash_message(&mut repo, index)?;
            let summary = format!(
                "Apply stash #{index}{}",
                if drop_after { " and drop it" } else { "" }
            );
            let details = vec![format!("Message: {message}")];
            (
                current.clone(),
                summary,
                details,
                Action::ApplyStash { index, drop_after },
            )
        }
        "drop_stash" => {
            let index = usize_arg(args, "index", 0);
            let message = stash_message(&mut repo, index)?;
            let summary = format!("Drop stash #{index} (the stashed changes are lost)");
            let details = vec![format!("Message: {message}")];
            (None, summary, details, Action::DropStash { index })
        }
        "delete_branch" => {
            let name = required(args, "name")?.to_string();
            validate_branch_name(&name)?;
            let branch = repo
                .find_branch(&name, git2::BranchType::Local)
                .map_err(|_| format!("No local branch named {name}"))?;
            let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);
            // Same rule as the app's delete command: a preventDeletion branch
            // rule holds even when force is asked for.
            let rules = crate::commands::branch_rules::load_rules(Path::new(repo_path))
                .map_err(|e| e.to_string())?;
            if crate::commands::branch_rules::is_deletion_prevented(&rules, &name) {
                return Err(format!(
                    "Branch \"{name}\" is protected by a branch rule and cannot be deleted. \
                     Remove the rule first."
                ));
            }
            let mut details = Vec::new();
            let merged = if force {
                details.push("Branch has unmerged commits; they become unreachable".to_string());
                true
            } else {
                // Mirrors `git branch -d`: an unborn HEAD is "not fully merged".
                let head = repo.head().map_err(|_| {
                    format!("Branch {name} is not fully merged. Use force to delete anyway.")
                })?;
                let head_oid = head.target().ok_or_else(|| {
                    format!("Branch {name} is not fully merged. Use force to delete anyway.")
                })?;
                let branch_oid = branch.get().target().ok_or_else(|| {
                    format!("Branch {name} is not fully merged. Use force to delete anyway.")
                })?;
                match crate::services::merge_detection::detect_merge_kind(
                    &repo, branch_oid, head_oid,
                ) {
                    Ok(Some(_)) => true,
                    Ok(None) => false,
                    Err(e) => return Err(e.to_string()),
                }
            };
            if !force && !merged {
                return Err(format!(
                    "Branch {name} is not fully merged. Use force to delete anyway."
                ));
            }
            let summary = format!(
                "{} branch {name}",
                if force { "Force delete" } else { "Delete" }
            );
            (Some(name.clone()), summary, details, Action::DeleteBranch { name, force })
        }
        "create_tag" => {
            let name = required(args, "name")?.to_string();
            // Tag names follow the same ref-name rules as branch names.
            if !git2::Branch::name_is_valid(&name).unwrap_or(false) {
                return Err(format!("Invalid tag name: {name}"));
            }
            if repo
                .find_reference(&format!("refs/tags/{name}"))
                .is_ok()
            {
                return Err(format!("Tag already exists: {name}"));
            }
            let target = str_arg(args, "target").map(str::to_string);
            let spec = target.clone().unwrap_or_else(|| "HEAD".to_string());
            let commit = resolve_commit(&repo, Some(&spec))?;
            let target_oid = commit.id().to_string();
            let message = str_arg(args, "message").map(str::to_string);
            let kind = if message.is_some() {
                "annotated"
            } else {
                "lightweight"
            };
            let summary = format!("Create {kind} tag {name} on {spec} @ {target_oid}");
            let details = vec![format!("Points at commit {target_oid}")];
            (
                None,
                summary,
                details,
                Action::CreateTag {
                    name,
                    target_oid,
                    target_ref: target,
                    message,
                },
            )
        }
        "delete_tag" => {
            let name = required(args, "name")?.to_string();
            repo.find_reference(&format!("refs/tags/{name}"))
                .map_err(|_| format!("No tag named {name}"))?;
            let summary = format!("Delete tag {name} (this cannot be undone)");
            (None, summary, Vec::new(), Action::DeleteTag { name })
        }
        "reset" => {
            let target_ref = required(args, "target_ref")?.to_string();
            // Same refusal as the app's reset: an in-progress merge, rebase,
            // cherry-pick or revert would be silently destroyed.
            crate::commands::reflog::ensure_resettable(&repo).map_err(|e| e.to_string())?;
            let commit = resolve_commit(&repo, Some(&target_ref))?;
            let mode = required(args, "mode")?.to_string();
            let summary = match mode.as_str() {
                "soft" => format!("Reset HEAD to {target_ref} @ {}", commit.id()),
                "mixed" => format!("Reset HEAD and index to {target_ref} @ {}", commit.id()),
                "hard" => format!(
                    "Reset HEAD, index and working tree to {target_ref} @ {} (uncommitted work is lost)",
                    commit.id()
                ),
                other => {
                    return Err(format!("Invalid reset mode: {other}. Use 'soft', 'mixed', or 'hard'"))
                }
            };
            let details = vec![format!("Mode: {mode}")];
            (
                current.clone(),
                summary,
                details,
                Action::Reset {
                    target_oid: commit.id().to_string(),
                    mode,
                },
            )
        }
        "revert_commit" => {
            if repo.state() != git2::RepositoryState::Clean {
                return Err("Another operation is in progress".to_string());
            }
            // Revert applies its result to the index: staged changes would be
            // silently folded into the revert commit.
            let head_tree = repo
                .head()
                .map_err(|_| "No commits to revert".to_string())?
                .peel_to_tree()
                .map_err(|e| e.to_string())?;
            let mut index = repo.index().map_err(|e| e.to_string())?;
            let index_tree_id = index.write_tree().map_err(|e| e.to_string())?;
            if index_tree_id != head_tree.id() {
                return Err(
                    "The index has staged changes. Commit or unstage them before reverting."
                        .to_string(),
                );
            }
            let sha = required(args, "sha")?.to_string();
            let oid = git2::Oid::from_str(&sha)
                .map_err(|_| format!("Invalid commit SHA: {sha}"))?;
            let commit = repo
                .find_commit(oid)
                .map_err(|_| format!("Commit not found: {sha}"))?;
            if commit.parent_count() == 0 {
                return Err("Cannot revert a root commit".to_string());
            }
            let mut mainline = args.get("mainline").and_then(Value::as_u64).map(|v| v as u32);
            if commit.parent_count() > 1 {
                match mainline {
                    Some(m) if m >= 1 && m <= commit.parent_count() as u32 => {}
                    Some(m) => {
                        return Err(format!(
                            "Mainline {m} is out of range: commit {sha} has {} parents.",
                            commit.parent_count()
                        ))
                    }
                    None => {
                        return Err(format!(
                            "Commit {sha} is a merge; pass mainline (1 or 2) to choose which parent to revert relative to."
                        ))
                    }
                }
            } else {
                mainline = None;
            }
            let summary = commit.summary().ok().flatten().unwrap_or("").to_string();
            let summary = format!("Revert \"{summary}\" ({sha})");
            let details = vec![format!("Creates a new commit undoing {sha}")];
            (
                current.clone(),
                summary,
                details,
                Action::RevertCommit { sha, mainline },
            )
        }
        other => return Err(format!("Unknown write tool: {other}")),
    };
    Ok(PreparedWrite {
        repo_path: repo_path.to_string(),
        branch,
        summary,
        details,
        action,
    })
}

fn resolve_commit<'r>(
    repo: &'r git2::Repository,
    start: Option<&str>,
) -> Result<git2::Commit<'r>, String> {
    let spec = start.unwrap_or("HEAD");
    repo.revparse_single(spec)
        .and_then(|o| o.peel_to_commit())
        .map_err(|_| format!("Cannot resolve start point: {spec}"))
}

/// Carry out an approved write.
pub fn execute(prepared: &PreparedWrite) -> Result<Value, String> {
    let mut repo = git2::Repository::open(&prepared.repo_path).map_err(|e| e.to_string())?;
    let err = |e: git2::Error| e.message().to_string();
    match &prepared.action {
        Action::Stage(paths) => {
            let workdir = repo.workdir().ok_or("No working tree")?.to_path_buf();
            let mut index = repo.index().map_err(err)?;
            for p in paths {
                if workdir.join(p).exists() {
                    index.add_path(Path::new(p)).map_err(err)?;
                } else {
                    index.remove_path(Path::new(p)).map_err(err)?;
                }
            }
            index.write().map_err(err)?;
            Ok(json!({ "staged": paths }))
        }
        Action::Commit(message) => {
            let sig = repo.signature().map_err(err)?;
            let mut index = repo.index().map_err(err)?;
            let tree = repo
                .find_tree(index.write_tree().map_err(err)?)
                .map_err(err)?;
            let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
            let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
            let oid = repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
                .map_err(err)?;
            Ok(json!({ "commit": oid.to_string() }))
        }
        Action::CreateBranch { name, start } => {
            let commit = resolve_commit(&repo, start.as_deref())?;
            repo.branch(name, &commit, false).map_err(err)?;
            Ok(json!({ "branch": name, "at": commit.id().to_string() }))
        }
        Action::Checkout(branch) => {
            let refname = format!("refs/heads/{branch}");
            let obj = repo.revparse_single(&refname).map_err(err)?;
            let mut opts = git2::build::CheckoutBuilder::new();
            opts.safe();
            repo.checkout_tree(&obj, Some(&mut opts))
                .map_err(|e| format!("Checkout would overwrite local changes: {}", e.message()))?;
            repo.set_head(&refname).map_err(err)?;
            Ok(json!({ "branch": branch }))
        }
        Action::Stash {
            message,
            include_untracked,
        } => {
            let sig = repo.signature().map_err(err)?;
            let mut flags = git2::StashFlags::DEFAULT;
            if *include_untracked {
                flags |= git2::StashFlags::INCLUDE_UNTRACKED;
            }
            let oid = repo
                .stash_save(
                    &sig,
                    message.as_deref().unwrap_or("Stashed by agent"),
                    Some(flags),
                )
                .map_err(err)?;
            Ok(json!({ "stash": oid.to_string() }))
        }
        Action::CreateWorktree {
            path,
            branch,
            start,
        } => {
            let commit = resolve_commit(&repo, start.as_deref())?;
            let new_branch = repo.branch(branch, &commit, false).map_err(err)?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| branch.replace('/', "-"));
            let mut opts = git2::WorktreeAddOptions::new();
            opts.reference(Some(new_branch.get()));
            if let Err(e) = repo.worktree(&name, path, Some(&opts)) {
                // Leave no orphan branch behind for a failed worktree.
                if let Ok(mut b) = repo.find_branch(branch, git2::BranchType::Local) {
                    let _ = b.delete();
                }
                return Err(e.message().to_string());
            }
            Ok(json!({ "worktree": path.to_string_lossy(), "branch": branch }))
        }
        Action::Unstage(paths) => {
            // Mirrors the unstage command: a path tracked in HEAD resets the
            // index entry to HEAD (`git reset -- <path>`); a path newly added
            // is dropped from the index. On an unborn HEAD there is no HEAD
            // version, so every path is dropped.
            let head_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
            let mut index = repo.index().map_err(err)?;
            for p in paths {
                let path_obj = Path::new(p);
                match &head_commit {
                    Some(commit) => {
                        let head_tree = commit.tree().map_err(err)?;
                        if head_tree.get_path(path_obj).is_ok() {
                            let literal =
                                crate::commands::path_utils::literal_git_pathspec(p);
                            repo.reset_default(
                                Some(commit.as_object()),
                                [Path::new(&literal)],
                            )
                            .map_err(err)?;
                            index = repo.index().map_err(err)?;
                        } else {
                            index.remove_path(path_obj).map_err(err)?;
                            index.write().map_err(err)?;
                            index = repo.index().map_err(err)?;
                        }
                    }
                    None => {
                        index.remove_path(path_obj).map_err(err)?;
                        index.write().map_err(err)?;
                        index = repo.index().map_err(err)?;
                    }
                }
            }
            index.write().map_err(err)?;
            Ok(json!({ "unstaged": paths }))
        }
        Action::WriteFile {
            file_path,
            content,
            stage_after,
        } => {
            let full = crate::commands::path_utils::validate_path_within_repo(
                Path::new(&prepared.repo_path),
                file_path,
            )
            .map_err(|e| e.to_string())?;
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&full, content)
                .map_err(|e| format!("Cannot write '{file_path}': {}", e))?;
            if *stage_after {
                let mut index = repo.index().map_err(err)?;
                index.add_path(Path::new(file_path)).map_err(err)?;
                index.write().map_err(err)?;
            }
            Ok(json!({ "file": file_path, "bytes": content.len(), "staged": stage_after }))
        }
        Action::Discard(paths) => {
            // Mirrors the discard command: tracked files are restored from the
            // INDEX (`git checkout -- <path>` semantics — never from HEAD, or a
            // staged version would be silently destroyed); untracked files are
            // deleted. The nested-repository guard ran in prepare, so nothing
            // reaches this point that would delete a repo's history.
            let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
            let index = repo.index().map_err(err)?;
            let mut index_paths: Vec<&str> = Vec::new();
            let mut untracked_paths: Vec<PathBuf> = Vec::new();
            for p in paths {
                let full = crate::commands::path_utils::validate_path_within_repo(
                    Path::new(&prepared.repo_path),
                    p,
                )
                .map_err(|e| e.to_string())?;
                let path_obj = Path::new(p);
                let in_head = head_tree
                    .as_ref()
                    .map(|t| t.get_path(path_obj).is_ok())
                    .unwrap_or(false);
                if index.get_path(path_obj, 0).is_some() {
                    index_paths.push(p.as_str());
                } else if !in_head {
                    untracked_paths.push(full);
                }
            }

            if !index_paths.is_empty() {
                let mut opts = git2::build::CheckoutBuilder::new();
                opts.force();
                opts.remove_untracked(false);
                // Paths are PATHSPECs to the builder: disable matching so a
                // filename with glob metacharacters only restores itself.
                opts.disable_pathspec_match(true);
                for p in &index_paths {
                    opts.path(p);
                }
                let mut fresh_index = repo.index().map_err(err)?;
                repo.checkout_index(Some(&mut fresh_index), Some(&mut opts))
                    .map_err(err)?;
            }

            for full in untracked_paths {
                // symlink_metadata, not is_dir(): the symlink itself must go,
                // never its target.
                if let Ok(meta) = std::fs::symlink_metadata(&full) {
                    if meta.file_type().is_dir() {
                        std::fs::remove_dir_all(&full).map_err(|e| e.to_string())?;
                    } else {
                        std::fs::remove_file(&full).map_err(|e| e.to_string())?;
                    }
                }
            }
            Ok(json!({ "discarded": paths }))
        }
        Action::ApplyStash { index, drop_after } => {
            repo.stash_apply(*index, None).map_err(err)?;
            if repo.index().map_err(err)?.has_conflicts() {
                return Err(format!(
                    "Stash #{index} applied with conflicts. Resolve them in Gitduda before continuing."
                ));
            }
            if *drop_after {
                repo.stash_drop(*index).map_err(err)?;
            }
            Ok(json!({ "applied": index, "dropped": drop_after }))
        }
        Action::DropStash { index } => {
            repo.stash_drop(*index).map_err(err)?;
            Ok(json!({ "dropped": index }))
        }
        Action::DeleteBranch { name, force } => {
            let mut branch = repo
                .find_branch(name, git2::BranchType::Local)
                .map_err(err)?;
            // The force flag is re-checked in execute: prepare refuses an
            // unmerged branch without force, and consent cannot upgrade it.
            if !*force {
                let head = repo.head().ok();
                let merged = head
                    .as_ref()
                    .and_then(|h| h.target())
                    .and_then(|head_oid| branch.get().target().map(|b| (b, head_oid)))
                    .and_then(|(branch_oid, head_oid)| {
                        crate::services::merge_detection::detect_merge_kind(
                            &repo, branch_oid, head_oid,
                        )
                        .ok()
                        .flatten()
                    })
                    .is_some();
                if head.is_none() || !merged {
                    return Err(format!(
                        "Branch {name} is not fully merged. Use force to delete anyway."
                    ));
                }
            }
            branch.delete().map_err(err)?;
            Ok(json!({ "deleted": name, "forced": force }))
        }
        Action::CreateTag {
            name,
            target_oid,
            target_ref: _,
            message,
        } => {
            // Re-resolve the target: prepare captured the oid for the preview,
            // and execute must point the tag at the commit that exists now.
            let oid = git2::Oid::from_str(target_oid)
                .map_err(|_| format!("Invalid target: {target_oid}"))?;
            let commit = repo.find_commit(oid).map_err(err)?;
            match message {
                Some(msg) => {
                    let sig = repo.signature().map_err(err)?;
                    let tag_oid = repo
                        .tag(name, commit.as_object(), &sig, msg, false)
                        .map_err(err)?;
                    Ok(json!({ "tag": name, "annotated": true, "oid": tag_oid.to_string(), "target": target_oid }))
                }
                None => {
                    let oid = repo
                        .tag_lightweight(name, commit.as_object(), false)
                        .map_err(err)?;
                    Ok(json!({ "tag": name, "annotated": false, "oid": oid.to_string(), "target": target_oid }))
                }
            }
        }
        Action::DeleteTag { name } => {
            repo.tag_delete(name).map_err(err)?;
            Ok(json!({ "deleted": name }))
        }
        Action::Reset { target_oid, mode } => {
            let reset_type = match mode.as_str() {
                "soft" => git2::ResetType::Soft,
                "mixed" => git2::ResetType::Mixed,
                "hard" => git2::ResetType::Hard,
                other => {
                    return Err(format!("Invalid reset mode: {other}. Use 'soft', 'mixed', or 'hard'"))
                }
            };
            let oid = git2::Oid::from_str(target_oid)
                .map_err(|_| format!("Invalid target: {target_oid}"))?;
            let commit = repo.find_commit(oid).map_err(err)?;
            repo.reset(commit.as_object(), reset_type, None).map_err(err)?;
            Ok(json!({ "reset": target_oid, "mode": mode }))
        }
        Action::RevertCommit { sha, mainline } => {
            // Defensive re-checks of the state prepare validated.
            if repo.state() != git2::RepositoryState::Clean {
                return Err("Another operation is in progress".to_string());
            }
            let head_tree = repo
                .head()
                .map_err(|_| "No commits to revert".to_string())?
                .peel_to_tree()
                .map_err(err)?;
            let mut index = repo.index().map_err(err)?;
            let index_tree_id = index.write_tree().map_err(err)?;
            if index_tree_id != head_tree.id() {
                return Err(
                    "The index has staged changes. Commit or unstage them before reverting."
                        .to_string(),
                );
            }
            let oid = git2::Oid::from_str(sha)
                .map_err(|_| format!("Invalid commit SHA: {sha}"))?;
            let commit = repo.find_commit(oid).map_err(err)?;

            let mut opts = git2::RevertOptions::new();
            if commit.parent_count() > 1 {
                // Merge commits need the caller to pick a mainline parent.
                let m = mainline.ok_or_else(|| {
                    format!("Commit {sha} is a merge; pass mainline (1 or 2).")
                })?;
                opts.mainline(m);
            }
            let mut checkout = git2::build::CheckoutBuilder::new();
            checkout
                .allow_conflicts(true)
                .conflict_style_merge(true);
            opts.checkout_builder(checkout);

            repo.revert(&commit, Some(&mut opts)).map_err(err)?;

            let mut index = repo.index().map_err(err)?;
            if index.has_conflicts() {
                // One path occupies up to three conflict stages; a
                // delete/modify conflict has no `our` or no `their` entry, so
                // all three have to be tried — the same naming the merge
                // command uses.
                let mut files: Vec<String> = Vec::new();
                for conflict in index.conflicts().map_err(err)? {
                    let conflict = conflict.map_err(err)?;
                    if let Some(entry) = conflict.our.or(conflict.their).or(conflict.ancestor) {
                        let p = String::from_utf8_lossy(&entry.path).into_owned();
                        if !files.contains(&p) {
                            files.push(p);
                        }
                    }
                }
                return Err(format!(
                    "Revert created conflicts in {}. Resolve them in Gitduda.",
                    files.join(", ")
                ));
            }

            let head = repo
                .head()
                .map_err(err)?
                .peel_to_commit()
                .map_err(err)?;
            let tree_oid = index.write_tree().map_err(err)?;
            // A revert that changes nothing is empty; stop like git does.
            if tree_oid == head.tree_id() {
                return Err(format!(
                    "Reverting {sha} would produce no changes (it is already reverted)"
                ));
            }
            let tree = repo.find_tree(tree_oid).map_err(err)?;
            let sig = repo.signature().map_err(err)?;
            let summary = commit.summary().ok().flatten().unwrap_or("").to_string();
            let revert_message = if commit.parent_count() > 1 {
                let m = mainline.unwrap_or(1);
                let parent_oid = commit
                    .parent((m - 1) as usize)
                    .map(|p| p.id().to_string())
                    .unwrap_or_default();
                format!(
                    "Revert \"{summary}\"\n\nThis reverts commit {sha}, reversing\nchanges made to {parent_oid}."
                )
            } else {
                format!("Revert \"{summary}\"\n\nThis reverts commit {sha}.")
            };
            let new_oid = repo
                .commit(Some("HEAD"), &sig, &sig, &revert_message, &tree, &[&head])
                .map_err(err)?;
            repo.cleanup_state().map_err(err)?;
            Ok(json!({ "commit": new_oid.to_string(), "reverted": sha }))
        }
    }
}

fn repo_prop() -> Value {
    json!({ "type": "string", "description": "Absolute path of the repository (must be open in Gitduda)" })
}

/// Write tools as advertised by `tools/list` while enabled.
pub fn write_tool_list() -> Vec<McpTool> {
    let tool = |name: &str, description: &str, properties: Value, required: Vec<&str>| McpTool {
        name: name.to_string(),
        description: format!(
            "{description} Requires approval in Gitduda unless a saved policy allows it."
        ),
        input_schema: json!({ "type": "object", "properties": properties, "required": required }),
    };
    vec![
        tool(
            "stage",
            "Stage files (paths relative to the repository root; deleted files are staged as deletions).",
            json!({ "repo_path": repo_prop(), "paths": { "type": "array", "items": { "type": "string" } } }),
            vec!["repo_path", "paths"],
        ),
        tool(
            "commit",
            "Commit the staged changes on the current branch.",
            json!({ "repo_path": repo_prop(), "message": { "type": "string" } }),
            vec!["repo_path", "message"],
        ),
        tool(
            "create_branch",
            "Create a branch (does not switch to it).",
            json!({ "repo_path": repo_prop(), "name": { "type": "string" }, "start_point": { "type": "string", "description": "Commit or ref (default HEAD)" } }),
            vec!["repo_path", "name"],
        ),
        tool(
            "create_worktree",
            "Create a new branch checked out in a new worktree at an absolute path.",
            json!({ "repo_path": repo_prop(), "path": { "type": "string" }, "branch": { "type": "string" }, "start_point": { "type": "string" } }),
            vec!["repo_path", "path", "branch"],
        ),
        tool(
            "stash",
            "Stash the working-tree changes.",
            json!({ "repo_path": repo_prop(), "message": { "type": "string" }, "include_untracked": { "type": "boolean" } }),
            vec!["repo_path"],
        ),
        tool(
            "checkout",
            "Switch to an existing local branch. Refuses to overwrite local changes.",
            json!({ "repo_path": repo_prop(), "branch": { "type": "string" } }),
            vec!["repo_path", "branch"],
        ),
        tool(
            "unstage",
            "Unstage files from the index (paths relative to the repository root).",
            json!({ "repo_path": repo_prop(), "paths": { "type": "array", "items": { "type": "string" } } }),
            vec!["repo_path", "paths"],
        ),
        tool(
            "write_file_content",
            "Write content to a file in the working tree (creates or overwrites it).",
            json!({ "repo_path": repo_prop(), "file_path": { "type": "string", "description": "Path relative to the repository root" }, "content": { "type": "string" }, "stage_after": { "type": "boolean", "description": "Stage the file after writing (default false)" } }),
            vec!["repo_path", "file_path", "content"],
        ),
        tool(
            "discard_changes",
            "Discard working-tree changes: tracked files are restored from the index, untracked files are deleted. Uncommitted work is lost.",
            json!({ "repo_path": repo_prop(), "paths": { "type": "array", "items": { "type": "string" } } }),
            vec!["repo_path", "paths"],
        ),
        tool(
            "apply_stash",
            "Apply a stash to the working tree (index 0 is the most recent).",
            json!({ "repo_path": repo_prop(), "index": { "type": "integer", "description": "Stash index (default 0)" }, "drop_after": { "type": "boolean", "description": "Drop the stash after applying (default false)" } }),
            vec!["repo_path"],
        ),
        tool(
            "drop_stash",
            "Delete a stash entry permanently.",
            json!({ "repo_path": repo_prop(), "index": { "type": "integer", "description": "Stash index (default 0)" } }),
            vec!["repo_path"],
        ),
        tool(
            "delete_branch",
            "Delete a local branch. Without force, refuses a branch with unmerged commits.",
            json!({ "repo_path": repo_prop(), "name": { "type": "string" }, "force": { "type": "boolean", "description": "Delete even if the branch has unmerged commits (default false)" } }),
            vec!["repo_path", "name"],
        ),
        tool(
            "create_tag",
            "Create a tag pointing at a target ref — a branch open in Gitduda, HEAD, a commit SHA or another tag. With a message the tag is annotated, otherwise lightweight.",
            json!({ "repo_path": repo_prop(), "name": { "type": "string" }, "target": { "type": "string", "description": "Ref to tag (commit SHA, branch, tag; default HEAD)" }, "message": { "type": "string", "description": "Annotated tag message; omit for a lightweight tag" } }),
            vec!["repo_path", "name"],
        ),
        tool(
            "delete_tag",
            "Delete a local tag permanently.",
            json!({ "repo_path": repo_prop(), "name": { "type": "string" } }),
            vec!["repo_path", "name"],
        ),
        tool(
            "reset",
            "Reset the current branch to a ref (soft keeps changes staged, mixed unstages them, hard also rewrites the working tree).",
            json!({ "repo_path": repo_prop(), "target_ref": { "type": "string", "description": "Ref to reset to (commit SHA, branch, tag)" }, "mode": { "type": "string", "enum": ["soft", "mixed", "hard"] } }),
            vec!["repo_path", "target_ref", "mode"],
        ),
        tool(
            "revert_commit",
            "Create a new commit that undoes a previous commit. Merge commits need a mainline parent.",
            json!({ "repo_path": repo_prop(), "sha": { "type": "string" }, "mainline": { "type": "integer", "description": "1-based parent to revert relative to, required for merge commits" } }),
            vec!["repo_path", "sha"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestRepo;

    #[test]
    fn glob_matching() {
        assert!(glob_match("agent/*", "agent/fix-login"));
        assert!(glob_match("agent/*", "agent/a/b"));
        assert!(!glob_match("agent/*", "main"));
        assert!(glob_match("*", "anything/at/all"));
        assert!(glob_match("release-?", "release-1"));
        assert!(!glob_match("release-?", "release-10"));
        assert!(glob_match("main", "main"));
        assert!(!glob_match("main", "mainline"));
    }

    #[test]
    fn policies_match_tool_repo_and_branch() {
        let t = TestRepo::with_initial_commit();
        let policies = vec![WritePolicy {
            repo_path: t.path_str(),
            tool: "commit".into(),
            branch_pattern: "agent/*".into(),
        }];
        assert!(policy_allows(
            &policies,
            "commit",
            &t.path_str(),
            Some("agent/x")
        ));
        assert!(!policy_allows(
            &policies,
            "commit",
            &t.path_str(),
            Some("main")
        ));
        assert!(!policy_allows(
            &policies,
            "stage",
            &t.path_str(),
            Some("agent/x")
        ));
        assert!(!policy_allows(
            &policies,
            "commit",
            "/elsewhere",
            Some("agent/x")
        ));
        assert!(!policy_allows(&policies, "commit", &t.path_str(), None));
    }

    fn gate(timeout_ms: u64) -> Arc<WriteGate> {
        Arc::new(WriteGate::in_memory(
            WriteSettings {
                enabled: true,
                policies: vec![],
            },
            Duration::from_millis(timeout_ms),
        ))
    }

    fn capture(gate: &WriteGate) -> Arc<Mutex<Vec<(String, Value)>>> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        gate.set_emitter(Arc::new(move |e, p| {
            sink.lock().unwrap().push((e.to_string(), p))
        }));
        events
    }

    fn stage_one(t: &TestRepo) -> PreparedWrite {
        std::fs::write(t.path.join("a.txt"), "a").unwrap();
        prepare("stage", &json!({ "paths": ["a.txt"] }), &t.path_str()).unwrap()
    }

    #[tokio::test]
    async fn approval_flow_emits_a_request_and_waits_for_the_answer() {
        let t = TestRepo::with_initial_commit();
        let g = gate(5000);
        let events = capture(&g);
        let prepared = stage_one(&t);

        let waiter = {
            let g = g.clone();
            let p = prepared.clone();
            tokio::spawn(async move { g.authorize("stage", &p).await })
        };
        // Wait for the request to be published.
        for _ in 0..100 {
            if !g.pending_requests().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let pending = g.pending_requests();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].tool, "stage");
        assert_eq!(pending[0].branch.as_deref(), Some("main"));
        assert_eq!(pending[0].details, vec!["a.txt".to_string()]);
        assert_eq!(events.lock().unwrap()[0].0, CONSENT_REQUEST_EVENT);

        g.respond(pending[0].id, true, None).unwrap();
        assert_eq!(waiter.await.unwrap(), Decision::Approved);
        assert!(g.pending_requests().is_empty());
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .any(|(e, _)| e == CONSENT_RESOLVED_EVENT));
        assert!(
            g.respond(pending[0].id, true, None).is_err(),
            "answering twice fails clearly"
        );
    }

    #[tokio::test]
    async fn denial_timeout_and_no_ui_all_deny() {
        let t = TestRepo::with_initial_commit();
        let prepared = stage_one(&t);

        // No UI to ask.
        let g = gate(5000);
        assert_eq!(g.authorize("stage", &prepared).await, Decision::Denied);

        // Timeout.
        let g = gate(50);
        let events = capture(&g);
        assert_eq!(g.authorize("stage", &prepared).await, Decision::TimedOut);
        assert!(g.pending_requests().is_empty());
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .any(|(e, p)| e == CONSENT_RESOLVED_EVENT && p["expired"] == true));

        // Explicit denial.
        let g = gate(5000);
        capture(&g);
        let waiter = {
            let g = g.clone();
            let p = prepared.clone();
            tokio::spawn(async move { g.authorize("stage", &p).await })
        };
        while g.pending_requests().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        g.respond(g.pending_requests()[0].id, false, None).unwrap();
        assert_eq!(waiter.await.unwrap(), Decision::Denied);
    }

    #[tokio::test]
    async fn always_allow_saves_a_policy_used_next_time() {
        let t = TestRepo::with_initial_commit();
        let dir = tempfile::TempDir::new().unwrap();
        let g = Arc::new(WriteGate::load(dir.path()));
        g.set_settings(WriteSettings {
            enabled: true,
            policies: vec![],
        })
        .unwrap();
        capture(&g);
        let prepared = stage_one(&t);

        let waiter = {
            let g = g.clone();
            let p = prepared.clone();
            tokio::spawn(async move { g.authorize("stage", &p).await })
        };
        while g.pending_requests().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        g.respond(g.pending_requests()[0].id, true, Some("ma*".into()))
            .unwrap();
        assert_eq!(waiter.await.unwrap(), Decision::Approved);

        // Persisted, and the next identical request needs no prompt.
        let reloaded = WriteGate::load(dir.path());
        assert_eq!(reloaded.settings().policies.len(), 1);
        assert_eq!(reloaded.settings().policies[0].branch_pattern, "ma*");
        assert_eq!(
            g.authorize("stage", &prepared).await,
            Decision::AllowedByPolicy
        );
    }

    #[tokio::test]
    async fn turning_writes_off_denies_waiting_requests() {
        let t = TestRepo::with_initial_commit();
        let g = gate(5000);
        capture(&g);
        let prepared = stage_one(&t);
        let waiter = {
            let g = g.clone();
            let p = prepared.clone();
            tokio::spawn(async move { g.authorize("stage", &p).await })
        };
        while g.pending_requests().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        g.set_settings(WriteSettings::default()).unwrap();
        assert_eq!(waiter.await.unwrap(), Decision::Denied);
    }

    #[test]
    fn stage_and_commit_execute() {
        let t = TestRepo::with_initial_commit();
        std::fs::write(t.path.join("new.txt"), "n").unwrap();
        std::fs::remove_file(t.path.join("README.md")).unwrap();
        let p = prepare(
            "stage",
            &json!({ "paths": ["new.txt", "README.md"] }),
            &t.path_str(),
        )
        .unwrap();
        execute(&p).unwrap();

        let c = prepare(
            "commit",
            &json!({ "message": "agent: add new.txt" }),
            &t.path_str(),
        )
        .unwrap();
        assert!(
            c.summary.contains("2 staged files on main"),
            "{}",
            c.summary
        );
        assert!(c.details.iter().any(|d| d.contains("new.txt")));
        let out = execute(&c).unwrap();
        let repo = t.repo();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(out["commit"], head.id().to_string());
        assert_eq!(head.summary().ok().flatten(), Some("agent: add new.txt"));
        assert!(head.tree().unwrap().get_name("README.md").is_none());

        assert!(
            prepare("commit", &json!({ "message": "again" }), &t.path_str())
                .unwrap_err()
                .contains("Nothing is staged")
        );
    }

    #[test]
    fn stage_rejects_paths_outside_the_repo() {
        let t = TestRepo::with_initial_commit();
        for bad in ["../x", "/etc/passwd", ".git/config"] {
            assert!(
                prepare("stage", &json!({ "paths": [bad] }), &t.path_str()).is_err(),
                "{bad}"
            );
        }
        assert!(prepare("stage", &json!({ "paths": [] }), &t.path_str()).is_err());
    }

    #[test]
    fn branch_checkout_stash_and_worktree_execute() {
        let t = TestRepo::with_initial_commit();
        let p = prepare(
            "create_branch",
            &json!({ "name": "agent/one" }),
            &t.path_str(),
        )
        .unwrap();
        assert_eq!(p.branch.as_deref(), Some("agent/one"));
        execute(&p).unwrap();
        assert!(prepare(
            "create_branch",
            &json!({ "name": "agent/one" }),
            &t.path_str()
        )
        .is_err());
        assert!(prepare("create_branch", &json!({ "name": "-x" }), &t.path_str()).is_err());
        assert!(prepare("create_branch", &json!({ "name": "a..b" }), &t.path_str()).is_err());

        let co = prepare("checkout", &json!({ "branch": "agent/one" }), &t.path_str()).unwrap();
        execute(&co).unwrap();
        assert_eq!(t.current_branch(), "agent/one");
        assert!(prepare("checkout", &json!({ "branch": "nope" }), &t.path_str()).is_err());

        assert!(prepare("stash", &json!({}), &t.path_str())
            .unwrap_err()
            .contains("no changes"));
        std::fs::write(t.path.join("README.md"), "changed").unwrap();
        let st = prepare("stash", &json!({ "message": "wip" }), &t.path_str()).unwrap();
        assert_eq!(st.branch.as_deref(), Some("agent/one"));
        execute(&st).unwrap();
        assert_eq!(
            std::fs::read_to_string(t.path.join("README.md")).unwrap(),
            "# Test Repo"
        );

        let dir = tempfile::TempDir::new().unwrap();
        let wt = dir.path().join("wt-two");
        let w = prepare(
            "create_worktree",
            &json!({ "path": wt.to_string_lossy(), "branch": "agent/two" }),
            &t.path_str(),
        )
        .unwrap();
        execute(&w).unwrap();
        assert!(wt.join("README.md").exists());
        assert!(prepare(
            "create_worktree",
            &json!({ "path": "relative/path", "branch": "agent/three" }),
            &t.path_str()
        )
        .is_err());
    }

    #[test]
    fn missing_arguments_are_reported() {
        let t = TestRepo::with_initial_commit();
        assert!(prepare("commit", &json!({}), &t.path_str())
            .unwrap_err()
            .contains("message"));
        assert!(prepare("checkout", &json!({}), &t.path_str())
            .unwrap_err()
            .contains("branch"));
        assert!(prepare("nope", &json!({}), &t.path_str()).is_err());
    }

    #[test]
    fn unstage_write_discard_and_stash_execute() {
        let t = TestRepo::with_initial_commit();

        // write_file_content creates the file (and its parents) and stages it
        let w = prepare(
            "write_file_content",
            &json!({ "file_path": "src/app.rs", "content": "fn main() {}\n", "stage_after": true }),
            &t.path_str(),
        )
        .unwrap();
        execute(&w).unwrap();
        assert_eq!(
            std::fs::read_to_string(t.path.join("src/app.rs")).unwrap(),
            "fn main() {}\n"
        );
        {
            let repo = t.repo();
            assert!(repo
                .index()
                .unwrap()
                .get_path(Path::new("src/app.rs"), 0)
                .is_some());
        }

        // unstage: a newly added path leaves the index, the file survives on disk
        let u = prepare(
            "unstage",
            &json!({ "paths": ["src/app.rs"] }),
            &t.path_str(),
        )
        .unwrap();
        assert_eq!(u.branch.as_deref(), Some("main"));
        execute(&u).unwrap();
        {
            let repo = t.repo();
            assert!(repo
                .index()
                .unwrap()
                .get_path(Path::new("src/app.rs"), 0)
                .is_none());
        }
        assert!(t.path.join("src/app.rs").exists());

        // unstage: a tracked path resets the index entry to HEAD's version
        std::fs::write(t.path.join("README.md"), "staged edit").unwrap();
        t.stage_file("README.md");
        let u2 = prepare(
            "unstage",
            &json!({ "paths": ["README.md"] }),
            &t.path_str(),
        )
        .unwrap();
        execute(&u2).unwrap();
        {
            let repo = t.repo();
            let mut opts = git2::StatusOptions::new();
            opts.include_untracked(false);
            let statuses = repo.statuses(Some(&mut opts)).unwrap();
            let entry = statuses
                .iter()
                .find(|e| e.path().ok() == Some("README.md"))
                .expect("README.md still modified");
            assert!(!entry.status().contains(git2::Status::INDEX_MODIFIED));
        }

        // discard_changes: tracked restored from the index, untracked deleted
        std::fs::write(t.path.join("README.md"), "unstaged edit").unwrap();
        t.create_file("untracked.txt", "tmp\n");
        let d = prepare(
            "discard_changes",
            &json!({ "paths": ["README.md", "untracked.txt"] }),
            &t.path_str(),
        )
        .unwrap();
        assert!(d.summary.contains("Discard changes to 2 files"));
        assert!(d.details.iter().any(|x| x.contains("Restore")));
        assert!(d.details.iter().any(|x| x.contains("Delete untracked")));
        execute(&d).unwrap();
        assert_eq!(
            std::fs::read_to_string(t.path.join("README.md")).unwrap(),
            "# Test Repo"
        );
        assert!(!t.path.join("untracked.txt").exists());

        // discard refuses a nested repository before anything happens
        let nested = t.path.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        git2::Repository::init(&nested).unwrap();
        let err = prepare(
            "discard_changes",
            &json!({ "paths": ["nested"] }),
            &t.path_str(),
        )
        .unwrap_err();
        assert!(err.contains("git repository"));

        // stash round trip: apply with drop_after, then drop_stash
        std::fs::write(t.path.join("README.md"), "to stash").unwrap();
        let st = prepare("stash", &json!({ "message": "wip for test" }), &t.path_str()).unwrap();
        execute(&st).unwrap();

        // The stash applies cleanly onto the now-clean working tree; a dirty
        // tree would merge-conflict, the same way `git stash apply` does.
        let a = prepare("apply_stash", &json!({ "drop_after": true }), &t.path_str()).unwrap();
        assert_eq!(a.branch.as_deref(), Some("main"));
        execute(&a).unwrap();
        assert_eq!(
            std::fs::read_to_string(t.path.join("README.md")).unwrap(),
            "to stash"
        );
        assert!(
            prepare("apply_stash", &json!({ "index": 9 }), &t.path_str())
                .unwrap_err()
                .contains("No stash at index")
        );

        std::fs::write(t.path.join("README.md"), "another").unwrap();
        let st2 = prepare("stash", &json!({}), &t.path_str()).unwrap();
        execute(&st2).unwrap();
        let ds = prepare("drop_stash", &json!({ "index": 0 }), &t.path_str()).unwrap();
        execute(&ds).unwrap();
        assert!(
            prepare("drop_stash", &json!({ "index": 0 }), &t.path_str())
                .unwrap_err()
                .contains("No stash")
        );
    }

    #[test]
    fn write_file_content_rejects_traversal() {
        let t = TestRepo::with_initial_commit();
        for bad in ["../x.txt", "/etc/hosts", ".git/config"] {
            assert!(
                prepare(
                    "write_file_content",
                    &json!({ "file_path": bad, "content": "x" }),
                    &t.path_str()
                )
                .is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn branch_tag_reset_and_revert_execute() {
        let t = TestRepo::with_initial_commit();
        let root = t.head_oid();

        // delete_branch: refuses an unmerged branch without force
        t.create_branch("feature");
        t.checkout_branch("feature");
        t.create_commit("Feature work", &[("feature.txt", "f\n")]);
        t.checkout_branch("main");
        let err = prepare(
            "delete_branch",
            &json!({ "name": "feature" }),
            &t.path_str(),
        )
        .unwrap_err();
        assert!(err.contains("not fully merged"));

        // ... and force deletes it, naming the unmerged commits in the preview
        let d = prepare(
            "delete_branch",
            &json!({ "name": "feature", "force": true }),
            &t.path_str(),
        )
        .unwrap();
        assert_eq!(d.branch.as_deref(), Some("feature"));
        assert!(d
            .details
            .iter()
            .any(|x| x.contains("unmerged")));
        execute(&d).unwrap();
        assert!(t
            .repo()
            .find_branch("feature", git2::BranchType::Local)
            .is_err());

        // A branch at HEAD's tip deletes without force (`git branch -d`)
        t.create_branch("merged");
        t.checkout_branch("merged");
        t.checkout_branch("main");
        let d2 = prepare(
            "delete_branch",
            &json!({ "name": "merged" }),
            &t.path_str(),
        )
        .unwrap();
        execute(&d2).unwrap();
        assert!(t
            .repo()
            .find_branch("merged", git2::BranchType::Local)
            .is_err());

        // A preventDeletion branch rule holds even with force
        let rules_path =
            crate::utils::app_paths::repo_dir(&t.path.join(".git")).join("branch_rules.json");
        std::fs::create_dir_all(rules_path.parent().unwrap()).unwrap();
        std::fs::write(
            &rules_path,
            r#"[{ "pattern": "protected/*", "preventDeletion": true, "preventForcePush": false, "requirePullRequest": false, "preventDirectPush": false }]"#,
        )
        .unwrap();
        t.create_branch("protected/main");
        assert!(
            prepare(
                "delete_branch",
                &json!({ "name": "protected/main", "force": true }),
                &t.path_str()
            )
            .unwrap_err()
            .contains("branch rule")
        );
        std::fs::remove_file(&rules_path).unwrap();

        // create_tag: annotated from a branch target, lightweight from HEAD
        let at = prepare(
            "create_tag",
            &json!({ "name": "v1.1", "target": "main", "message": "release notes" }),
            &t.path_str(),
        )
        .unwrap();
        assert!(at.summary.contains("annotated tag v1.1 on main"));
        execute(&at).unwrap();
        {
            let repo = t.repo();
            let ref_oid = repo.refname_to_id("refs/tags/v1.1").unwrap();
            let tag = repo.find_tag(ref_oid).unwrap();
            assert_eq!(
                tag.message().ok().flatten().unwrap_or(""),
                "release notes"
            );
        }
        let lw = prepare("create_tag", &json!({ "name": "v2.0" }), &t.path_str()).unwrap();
        assert!(lw.summary.contains("lightweight tag v2.0 on HEAD"));
        execute(&lw).unwrap();
        assert!(
            prepare("create_tag", &json!({ "name": "v1.1" }), &t.path_str())
                .unwrap_err()
                .contains("already exists")
        );
        assert!(
            prepare("create_tag", &json!({ "name": "-bad" }), &t.path_str())
                .unwrap_err()
                .contains("Invalid tag name")
        );

        // delete_tag
        let dt = prepare("delete_tag", &json!({ "name": "v1.1" }), &t.path_str()).unwrap();
        execute(&dt).unwrap();
        assert!(t.repo().refname_to_id("refs/tags/v1.1").is_err());
        assert!(
            prepare("delete_tag", &json!({ "name": "nope" }), &t.path_str())
                .unwrap_err()
                .contains("No tag")
        );

        // reset: mixed moves HEAD back and keeps the files; hard restores the
        // tracked working tree (untracked files survive, like git reset --hard)
        let first = t.head_oid();
        t.create_commit("Second", &[("b.txt", "b\n")]);
        std::fs::write(t.path.join("README.md"), "hard test").unwrap();
        let r = prepare(
            "reset",
            &json!({ "target_ref": first.to_string(), "mode": "mixed" }),
            &t.path_str(),
        )
        .unwrap();
        assert_eq!(r.branch.as_deref(), Some("main"));
        execute(&r).unwrap();
        assert_eq!(t.head_oid(), first);
        assert!(t.path.join("b.txt").exists());
        let rh = prepare(
            "reset",
            &json!({ "target_ref": first.to_string(), "mode": "hard" }),
            &t.path_str(),
        )
        .unwrap();
        execute(&rh).unwrap();
        assert_eq!(
            std::fs::read_to_string(t.path.join("README.md")).unwrap(),
            "# Test Repo"
        );
        assert!(t.path.join("b.txt").exists());
        assert!(
            prepare(
                "reset",
                &json!({ "target_ref": "HEAD", "mode": "softish" }),
                &t.path_str()
            )
            .unwrap_err()
            .contains("Invalid reset mode")
        );

        // revert_commit: creates a new commit undoing the change
        let second = t.create_commit("Add feature file", &[("c.txt", "c\n")]);
        let rev = prepare(
            "revert_commit",
            &json!({ "sha": second.to_string() }),
            &t.path_str(),
        )
        .unwrap();
        assert_eq!(rev.branch.as_deref(), Some("main"));
        execute(&rev).unwrap();
        assert!(!t.path.join("c.txt").exists());
        {
            let repo = t.repo();
            let head = repo
                .head()
                .unwrap()
                .peel_to_commit()
                .unwrap();
            assert!(head
                .message()
                .unwrap_or("")
                .starts_with("Revert \"Add feature file\""));
        }

        // A root commit cannot be reverted
        assert!(
            prepare(
                "revert_commit",
                &json!({ "sha": root.to_string() }),
                &t.path_str()
            )
            .unwrap_err()
            .contains("root commit")
        );
    }

    #[test]
    fn revert_refuses_staged_changes() {
        let t = TestRepo::with_initial_commit();
        let second = t.create_commit("Second", &[("b.txt", "b\n")]);
        t.create_file("d.txt", "d\n");
        t.stage_file("d.txt");
        let err = prepare(
            "revert_commit",
            &json!({ "sha": second.to_string() }),
            &t.path_str(),
        )
        .unwrap_err();
        assert!(err.contains("staged changes"));
    }

    #[test]
    fn tool_list_names_every_write_tool() {
        let names: Vec<String> = write_tool_list().into_iter().map(|t| t.name).collect();
        assert_eq!(names, WRITE_TOOLS.to_vec());
    }
}

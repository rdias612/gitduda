# GitDuda MCP Server

GitDuda embeds an MCP (Model Context Protocol) server that lets clients such as
Claude Code, Cursor, and VS Code operate the repositories open in the app. The
server is started in **Settings → MCP Server** and serves two transports:
**HTTP** (`http://127.0.0.1:<port>/mcp`, authenticated with a Bearer token) and
**stdio** (`gitduda --mcp-stdio`, a bridge to the local HTTP server).

Both transports serve the **same catalog**: stdio is a bridge to the HTTP
server, so `tools/list` is identical on both.

## Permissions

| Group | Availability | Gate |
| :--- | :--- | :--- |
| **Read** (15) | Always listed | Honors the open-repository list (`repo_path` must be open in GitDuda) |
| **Insight** (5) | Always listed | Rely on app services at call time: a configured AI provider (`explain_commit`), the semantic index (`semantic_search_history`), or the GitHub integration (`get_pr_comments`) |
| **Write** (16) | Only in `tools/list` when enabled | Off by default in **Settings → MCP Server**; every call requires consent in the app (with a preview) or a saved per-repository/branch policy |

Write consent reaches the app through the `mcp-consent-request` event and is
answered through `mcp-consent-resolved`; an applied write is reported through
`mcp-write-applied`. A saved policy (`tool` + `repo_path` + `branch_pattern`
with a glob) authorizes the tool without asking again.

Every `repo_path` is validated: it must be absolute, exist, be a git
repository, and be on the open-repository list (or any valid repository while
none is open, during initial setup).

## Catalog

### Read (always available)

| Tool | What it does | Arguments |
| :--- | :--- | :--- |
| `get_open_repositories` | Repositories currently open in GitDuda | — |
| `get_commit_history` | Recent commit history | `repo_path`, `count` (20), `branch` (HEAD) |
| `get_commit` | Full detail of one commit: metadata, parents, and changed files (renames detected, per-file additions/deletions) | `repo_path`, `sha` |
| `search_commits` | Search commits by message text | `repo_path`, `query`, `max_results` (20) |
| `get_file_history` | Commits that touched a file, following renames by default | `repo_path`, `file_path`, `max_results` (100, max 500), `follow_renames` (true) |
| `get_branches` | All local and remote branches, marking the current one | `repo_path` |
| `get_tags` | All tags; annotated ones carry message, tagger, and date | `repo_path` |
| `get_stashes` | All stashes with index, message, and oid | `repo_path` |
| `get_worktrees` | All worktrees (path, branch, head, locked) | `repo_path` |
| `get_remotes` | Configured remotes with fetch and push URLs | `repo_path` |
| `get_status` | Working-tree state (staged / modified / untracked) | `repo_path` |
| `get_diff` | Diff of the working tree or between two refs | `repo_path`, `from_ref`, `to_ref`, `file_path` (optional, limits the diff to one file) |
| `get_commit_stats` | Statistics (files, insertions, deletions) between refs | `repo_path`, `from_ref`, `to_ref` (HEAD) |
| `get_file_blame` | Line-by-line authorship of a file | `repo_path`, `file_path` |
| `get_file_content` | A file's content from the working tree, truncated at `max_bytes` | `repo_path`, `file_path`, `max_bytes` (256 KiB, cap 2 MiB) |

### Insight (backed by app services)

| Tool | What it does | Requirement |
| :--- | :--- | :--- |
| `get_conflicts` | Files in a merge/rebase conflict, with which sides (base/ours/theirs) exist | — |
| `predict_rebase_conflicts` | Dry-run a rebase in a throwaway worktree; nothing changes | — |
| `semantic_search_history` | Find commits by meaning (local semantic index) | Semantic index |
| `explain_commit` | Explain a commit in plain language | AI provider |
| `get_pr_comments` | Discussion and review comments of a GitHub pull request | GitHub integration |

### Write (off by default + consent)

| Tool | What it does | Notes |
| :--- | :--- | :--- |
| `stage` | Add files to the index (deletions included) | Paths relative to the root |
| `unstage` | Remove files from the index | Tracked paths reset to HEAD; newly added ones leave the index |
| `commit` | Commit the staged changes on the current branch | |
| `checkout` | Switch the current branch | Refuses to overwrite local changes |
| `create_branch` | Create a branch without switching | Refuses invalid/duplicate names |
| `delete_branch` | Delete a local branch | Without `force`, refuses an unmerged branch; `preventDeletion` branch rules hold even with `force` |
| `create_tag` | Create a tag pointing at `target` (branch, HEAD, SHA, or another tag) | With `message` → annotated; without → lightweight; refuses duplicates |
| `delete_tag` | Delete a local tag | |
| `create_worktree` | Create a new branch in a worktree at an absolute path | Refuses an existing path |
| `stash` | Stash the working-tree changes | `include_untracked` optional |
| `apply_stash` | Apply a stash to the working tree | `drop_after` optional; refuses on conflict without dropping the stash |
| `drop_stash` | Delete a stash entry | Permanent |
| `write_file_content` | Write content to a working-tree file (creates or overwrites) | `stage_after` optional; same path validation as the app |
| `discard_changes` | Discard working-tree changes | Tracked files restore from the index; untracked files are deleted; refuses a nested repository before anything happens |
| `reset` | `git reset` the current branch | `mode` = `soft` \| `mixed` \| `hard`; refuses with a merge/rebase/cherry-pick/revert in progress |
| `revert_commit` | Create a new commit undoing another | Merge commits need `mainline` (1-based); on conflict, returns the affected files for resolution in the app |

## Examples

Every call is JSON-RPC via `tools/call`. Client examples:

```json
// Full detail of a commit
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
  "name":"get_commit",
  "arguments":{"repo_path":"C:/dev/project","sha":"abc1234..."}
}}
```

```json
// Create an annotated tag from a branch open in GitDuda
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
  "name":"create_tag",
  "arguments":{"repo_path":"C:/dev/project","name":"v1.1.0","target":"main","message":"Release 1.1.0"}
}}
```

```json
// Delete an unmerged branch (requires force and consent)
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
  "name":"delete_branch",
  "arguments":{"repo_path":"C:/dev/project","name":"feature/old","force":true}
}}
```

```json
// A file's history following renames
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
  "name":"get_file_history",
  "arguments":{"repo_path":"C:/dev/project","file_path":"src/app.ts"}
}}
```

```json
// Hard-reset the current branch (consent shows a preview)
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{
  "name":"reset",
  "arguments":{"repo_path":"C:/dev/project","target_ref":"origin/main","mode":"hard"}
}}
```

```json
// Revert a commit (merge commits: include "mainline": 1)
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{
  "name":"revert_commit",
  "arguments":{"repo_path":"C:/dev/project","sha":"abc1234..."}
}}
```

```json
// Edit a file and stage it
{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{
  "name":"write_file_content",
  "arguments":{"repo_path":"C:/dev/project","file_path":"docs/note.md","content":"# New note","stage_after":true}
}}
```

## What the MCP deliberately does not expose

Decisions from the catalog review (issue #29) — operations the app has, but
that do not make sense to expose to an agent in this phase:

- **Network operations** (`push`, `pull`, `fetch`, remote branch/tag deletion):
  external effect on the provider; the MCP client should ask the user or use
  the app.
- **Interactive history rewriting** (interactive rebase, `cherry_pick`,
  `amend`, `reword`, `squash`): an in-progress state machine is hard to drive
  without the UI; `revert_commit` and `reset` cover the common cases.
- **Global `clean`**: `discard_changes` per file covers the common case with
  per-call consent.
- **App configuration** (profiles, credentials, SSH/GPG): outside the MCP
  scope; operate in the app.

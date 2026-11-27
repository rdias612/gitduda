//! Compare and synchronize local refs against one remote's advertisement.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use git2::{build::CheckoutBuilder, AutotagOption, Direction, Oid, Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{command, State};

use super::{await_remote_task, guard_remote_op, network_deadline, network_timeout};
use crate::commands::{branch::local_branch_worktree, branch_rules};
use crate::error::{GitdudaError, Result};
use crate::services::{
    credentials_service,
    remote_ops::{RemoteOp, RemoteOpGuard, RemoteOpRegistry},
};

const REFLOG_MESSAGE: &str = "gitduda: sync refs with remote";

type Refs = BTreeMap<String, Oid>;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncRefKind {
    Branches,
    Tags,
}

impl SyncRefKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Branches => "refs/heads/",
            Self::Tags => "refs/tags/",
        }
    }

    fn refname(self, name: &str) -> String {
        format!("{}{name}", self.prefix())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefChange {
    name: String,
    local_oid: Option<String>,
    remote_oid: Option<String>,
    outgoing_commits: usize,
    worktree: Option<String>,
    protected: bool,
}

impl RefChange {
    fn new(name: &str, local: Option<Oid>, remote: Option<Oid>) -> Self {
        Self {
            name: name.into(),
            local_oid: local.map(|oid| oid.to_string()),
            remote_oid: remote.map(|oid| oid.to_string()),
            outgoing_commits: 0,
            worktree: None,
            protected: false,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRefsPreview {
    revision: String,
    remote_count: usize,
    new_refs: Vec<RefChange>,
    deletions: Vec<RefChange>,
    overwrites: Vec<RefChange>,
    current_branch: Option<String>,
    current_branch_deleted: bool,
    working_changes: bool,
    worktree_busy: bool,
    replacements: Vec<String>,
}

impl SyncRefsPreview {
    fn blocked(&self) -> bool {
        self.worktree_busy
            || self
                .deletions
                .iter()
                .chain(&self.overwrites)
                .chain(&self.new_refs)
                .any(|change| change.protected || change.worktree.is_some())
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ApplySyncRefsResult {
    Applied,
    Changed { preview: SyncRefsPreview },
}

struct PublishedRefs {
    url: String,
    refs: Refs,
}

fn local_refs(repo: &Repository, kind: SyncRefKind) -> Result<Refs> {
    repo.references_glob(&format!("{}*", kind.prefix()))?
        .map(|reference| {
            let reference = reference?;
            let name = reference
                .name()?
                .strip_prefix(kind.prefix())
                .ok_or(GitdudaError::InvalidReference)?
                .to_owned();
            // The direct target is the tag object itself for an annotated tag.
            let oid = reference
                .resolve()?
                .target()
                .ok_or(GitdudaError::InvalidReference)?;
            Ok((name, oid))
        })
        .collect()
}

fn published_refs(
    repo: &Repository,
    name: &str,
    kind: SyncRefKind,
    token: Option<String>,
    deadline: Option<Instant>,
    applying: bool,
) -> Result<PublishedRefs> {
    let mut remote = repo
        .find_remote(name)
        .map_err(|_| GitdudaError::RemoteNotFound(name.into()))?;
    let url = remote.url()?.to_owned();
    let (callbacks, _) = credentials_service::get_callbacks_with_deadline(token.clone(), deadline);
    let mut connection = remote.connect_auth(Direction::Fetch, Some(callbacks), None)?;
    let refs: Refs = connection
        .list()?
        .iter()
        .filter_map(|head| {
            let short = head.name().strip_prefix(kind.prefix())?;
            // A peeled advertisement describes the tag's target, not the tag itself.
            (!short.ends_with("^{}")).then(|| (short.to_owned(), head.oid()))
        })
        .collect();

    let needs_objects = applying || kind == SyncRefKind::Branches;
    if needs_objects && !refs.is_empty() {
        let odb = repo.odb()?;
        if refs.values().any(|oid| !odb.exists(*oid)) {
            let (mut options, _) =
                credentials_service::get_fetch_options_with_deadline(token, deadline);
            options.download_tags(AutotagOption::None);
            // Download only objects. No update_tips(), FETCH_HEAD, or tracking refs.
            connection.remote().download(
                &[format!("{}*:refs/gitduda-sync/*", kind.prefix())],
                Some(&mut options),
            )?;
        }
        for oid in refs.values() {
            repo.find_object(*oid, None)?;
        }
    }
    Ok(PublishedRefs { url, refs })
}

fn outgoing_on_delete(repo: &Repository, tip: Oid, published: &BTreeSet<Oid>) -> Result<usize> {
    if published.contains(&tip) {
        return Ok(0);
    }
    let mut walk = repo.revwalk()?;
    walk.push(tip)?;
    for oid in published {
        walk.hide(*oid)?;
    }
    walk.try_fold(0, |count, commit| commit.map(|_| count + 1))
        .map_err(Into::into)
}

fn make_preview(
    repo: &Repository,
    path: &Path,
    remote: &str,
    kind: SyncRefKind,
    local: &Refs,
    published: &PublishedRefs,
) -> Result<SyncRefsPreview> {
    let heads = &published.refs;
    let current_branch = if kind == SyncRefKind::Branches {
        repo.find_reference("HEAD")?
            .symbolic_target()?
            .and_then(|name| name.strip_prefix(kind.prefix()))
            .map(str::to_owned)
    } else {
        None
    };
    let current = current_branch.as_deref();
    let current_branch_deleted =
        current.is_some_and(|name| local.contains_key(name) && !heads.contains_key(name));
    let current_changed = current.is_some_and(|name| local.get(name) != heads.get(name));
    let worktree_busy = kind == SyncRefKind::Branches
        && (repo.state() != git2::RepositoryState::Clean || (current_changed && repo.is_bare()));
    let working_changes = if current_changed && !repo.is_bare() {
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        !repo.statuses(Some(&mut options))?.is_empty()
    } else {
        false
    };
    let mut preview = SyncRefsPreview {
        revision: String::new(),
        remote_count: heads.len(),
        new_refs: Vec::new(),
        deletions: Vec::new(),
        overwrites: Vec::new(),
        current_branch,
        current_branch_deleted,
        working_changes,
        worktree_busy,
        replacements: Vec::new(),
    };
    let rules = if kind == SyncRefKind::Branches {
        branch_rules::load_rules(path)?
    } else {
        Vec::new()
    };
    let published_oids: BTreeSet<Oid> = heads.values().copied().collect();
    for (name, &old) in local {
        let remote_oid = heads.get(name).copied();
        if remote_oid == Some(old) {
            continue;
        }
        let mut change = RefChange::new(name, Some(old), remote_oid);
        if kind == SyncRefKind::Branches {
            change.worktree = local_branch_worktree(repo, name);
            change.protected =
                remote_oid.is_none() && branch_rules::is_deletion_prevented(&rules, name);
            change.outgoing_commits = match remote_oid {
                Some(new) => repo.graph_ahead_behind(old, new)?.0,
                None => outgoing_on_delete(repo, old, &published_oids)?,
            };
        }
        if remote_oid.is_some() {
            preview.overwrites.push(change);
        } else {
            preview.deletions.push(change);
        }
    }
    for (name, &new) in heads {
        if !local.contains_key(name) {
            let mut change = RefChange::new(name, None, Some(new));
            if kind == SyncRefKind::Branches {
                change.worktree = local_branch_worktree(repo, name);
            }
            preview.new_refs.push(change);
        }
        if current_branch_deleted && local_branch_worktree(repo, name).is_none() {
            preview.replacements.push(name.clone());
        }
    }

    // Include unchanged refs too, since either side may have moved in lockstep.
    let refs_for_revision = |refs: &Refs| {
        refs.iter()
            .map(|(name, oid)| (name.clone(), oid.to_string()))
            .collect::<Vec<_>>()
    };
    let snapshot = serde_json::to_vec(&(
        remote,
        kind,
        &published.url,
        refs_for_revision(local),
        refs_for_revision(heads),
        &preview,
    ))?;
    preview.revision = URL_SAFE_NO_PAD.encode(Sha256::digest(snapshot));
    Ok(preview)
}

async fn inspect_remote(
    path: String,
    remote: String,
    kind: SyncRefKind,
    token: Option<String>,
    timeout_secs: Option<u64>,
    applying: bool,
    slot: RemoteOpGuard,
) -> Result<(PublishedRefs, RemoteOpGuard)> {
    let deadline = network_deadline(timeout_secs);
    let task = tokio::task::spawn_blocking(move || {
        guard_remote_op(&path, false, |_| Some(remote.clone()))?;
        let repo = Repository::open(&path)?;
        let published = published_refs(&repo, &remote, kind, token, deadline, applying)?;
        Ok((published, slot))
    });
    // A timed-out request can finish downloading objects, but it cannot reach
    // the separate local mutation phase after its caller has stopped waiting.
    await_remote_task(
        network_timeout(timeout_secs),
        "Reference sync",
        task,
        |_| {},
    )
    .await
}

#[command]
pub async fn preview_sync_refs(
    registry: State<'_, RemoteOpRegistry>,
    path: String,
    remote: String,
    kind: SyncRefKind,
    token: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<SyncRefsPreview> {
    let slot = registry.acquire(Path::new(&path), RemoteOp::SyncRefs)?;
    let (published, _slot) = inspect_remote(
        path.clone(),
        remote.clone(),
        kind,
        token,
        timeout_secs,
        false,
        slot,
    )
    .await?;
    crate::utils::blocking_git(move || {
        let repo = Repository::open(&path)?;
        let local = local_refs(&repo, kind)?;
        make_preview(&repo, Path::new(&path), &remote, kind, &local, &published)
    })
    .await
}

#[command]
#[allow(clippy::too_many_arguments)]
pub async fn apply_sync_refs(
    registry: State<'_, RemoteOpRegistry>,
    path: String,
    remote: String,
    kind: SyncRefKind,
    revision: String,
    replacement: Option<String>,
    token: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<ApplySyncRefsResult> {
    let slot = registry.acquire(Path::new(&path), RemoteOp::SyncRefs)?;
    let (published, slot) = inspect_remote(
        path.clone(),
        remote.clone(),
        kind,
        token,
        timeout_secs,
        true,
        slot,
    )
    .await?;
    crate::utils::blocking_git(move || {
        let _slot = slot;
        let path = Path::new(&path);
        let repo = Repository::open(path)?;
        apply(
            &repo,
            path,
            &remote,
            kind,
            &published,
            &revision,
            replacement.as_deref(),
        )
    })
    .await
}

fn apply(
    repo: &Repository,
    path: &Path,
    remote: &str,
    kind: SyncRefKind,
    published: &PublishedRefs,
    revision: &str,
    replacement: Option<&str>,
) -> Result<ApplySyncRefsResult> {
    let local = local_refs(repo, kind)?;
    let preview = make_preview(repo, path, remote, kind, &local, published)?;
    if preview.revision != revision {
        return Ok(ApplySyncRefsResult::Changed { preview });
    }
    if preview.blocked() {
        return Err(GitdudaError::OperationFailed(
            "The synchronization contains blocked branches".into(),
        ));
    }
    let switch_to = if preview.current_branch_deleted {
        Some(
            replacement
                .filter(|name| preview.replacements.iter().any(|item| item == name))
                .ok_or_else(|| {
                    GitdudaError::OperationFailed(
                        "Choose a branch to switch to before deleting the current branch".into(),
                    )
                })?,
        )
    } else {
        None
    };
    let checkout_branch = switch_to.or(preview.current_branch.as_deref());
    let checkout_target = checkout_branch
        .and_then(|name| published.refs.get(name))
        .copied()
        .filter(|oid| {
            preview.current_branch_deleted
                || checkout_branch.and_then(|name| local.get(name)) != Some(oid)
        });

    let mut transaction = repo.transaction()?;
    let replacement_ref = switch_to.map(|name| kind.refname(name));
    let mut switch = replacement_ref
        .as_ref()
        .map(|_| repo.transaction())
        .transpose()?;
    let changes = preview
        .new_refs
        .iter()
        .chain(&preview.deletions)
        .chain(&preview.overwrites);
    let mut locked: BTreeSet<_> = changes
        .clone()
        .map(|change| kind.refname(&change.name))
        .collect();
    if locked.is_empty() {
        return Ok(ApplySyncRefsResult::Applied);
    }
    if kind == SyncRefKind::Branches {
        locked.insert("HEAD".into());
        locked.extend(replacement_ref.iter().cloned());
    }
    for name in locked {
        match &mut switch {
            Some(switch) if name == "HEAD" || replacement_ref.as_ref() == Some(&name) => {
                switch.lock_ref(&name)?;
            }
            _ => transaction.lock_ref(&name)?,
        }
    }
    let rechecked = make_preview(
        repo,
        path,
        remote,
        kind,
        &local_refs(repo, kind)?,
        published,
    )?;
    if rechecked.revision != revision {
        return Ok(ApplySyncRefsResult::Changed { preview: rechecked });
    }
    for change in changes {
        let name = kind.refname(&change.name);
        let transaction = match &mut switch {
            Some(switch) if replacement_ref.as_ref() == Some(&name) => switch,
            _ => &mut transaction,
        };
        if let Some(&oid) = published.refs.get(&change.name) {
            transaction.set_target(&name, oid, None, REFLOG_MESSAGE)?;
        } else {
            transaction.remove(&name)?;
        }
    }
    if let (Some(switch), Some(name)) = (&mut switch, &replacement_ref) {
        switch.set_symbolic_target("HEAD", name, None, REFLOG_MESSAGE)?;
    }
    if let Some(oid) = checkout_target {
        let commit = repo.find_commit(oid)?;
        repo.checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))?;
    }
    // Open the selected branch (creating it if needed) before removing the
    // previous HEAD branch. The rest of the batch is already locked.
    if let Some(switch) = switch {
        switch.commit()?;
    }
    transaction.commit()?;
    Ok(ApplySyncRefsResult::Applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestRepo;

    const ORIGIN: &str = "origin";

    fn preview(repo: &Repository, kind: SyncRefKind, published: &PublishedRefs) -> SyncRefsPreview {
        make_preview(
            repo,
            repo.workdir().unwrap(),
            ORIGIN,
            kind,
            &local_refs(repo, kind).unwrap(),
            published,
        )
        .unwrap()
    }

    fn apply_preview(
        repo: &Repository,
        kind: SyncRefKind,
        published: &PublishedRefs,
        revision: &str,
        replacement: Option<&str>,
    ) -> Result<ApplySyncRefsResult> {
        apply(
            repo,
            repo.workdir().unwrap(),
            ORIGIN,
            kind,
            published,
            revision,
            replacement,
        )
    }

    fn annotated_tag(repo: &Repository, name: &str, message: &str) -> Oid {
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.tag(name, commit.as_object(), &sig, message, false)
            .unwrap()
    }

    #[test]
    fn tags_use_the_advertised_object_and_refresh_stale_previews() {
        let remote = TestRepo::with_initial_commit();
        let source = remote.repo();
        annotated_tag(&source, "stable", "Same on both sides");
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::clone(&remote.path_str(), dir.path()).unwrap();
        let kind = SyncRefKind::Tags;
        let old_tag = annotated_tag(&repo, "v1.0.0", "Local annotation");
        annotated_tag(&repo, "local-only", "Delete this tag");
        let new_tag = annotated_tag(&source, "v1.0.0", "Published annotation");

        let published = published_refs(&repo, ORIGIN, kind, None, None, false).unwrap();
        let first = preview(&repo, kind, &published);
        assert_eq!(first.new_refs.len(), 0);
        assert_eq!(first.deletions[0].name, "local-only");
        assert_eq!(first.overwrites[0].local_oid, Some(old_tag.to_string()));
        assert_eq!(first.overwrites[0].remote_oid, Some(new_tag.to_string()));
        assert!(!repo.odb().unwrap().exists(new_tag));

        let new_name = annotated_tag(&source, "new-tag", "New remote tag");
        let published = published_refs(&repo, ORIGIN, kind, None, None, true).unwrap();
        let result = apply_preview(&repo, kind, &published, &first.revision, None).unwrap();
        let ApplySyncRefsResult::Changed { preview: updated } = result else {
            panic!("expected refreshed comparison");
        };
        assert_eq!(updated.new_refs[0].name, "new-tag");
        assert_eq!(repo.refname_to_id("refs/tags/v1.0.0").unwrap(), old_tag);
        assert!(repo.find_reference("refs/tags/new-tag").is_err());

        assert!(matches!(
            apply_preview(&repo, kind, &published, &updated.revision, None).unwrap(),
            ApplySyncRefsResult::Applied
        ));
        assert_eq!(repo.refname_to_id("refs/tags/new-tag").unwrap(), new_name);
        assert_eq!(repo.refname_to_id("refs/tags/v1.0.0").unwrap(), new_tag);
        assert_eq!(
            repo.find_tag(new_tag).unwrap().message().unwrap(),
            Some("Published annotation")
        );
        assert!(repo.find_reference("refs/tags/local-only").is_err());
        assert!(source.find_reference("refs/tags/local-only").is_err());
        let synced = preview(&repo, kind, &published);
        assert!(
            synced.new_refs.is_empty()
                && synced.deletions.is_empty()
                && synced.overwrites.is_empty()
        );
    }

    #[test]
    fn remote_without_tags_lists_every_local_tag_for_deletion() {
        let remote = TestRepo::with_initial_commit();
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::clone(&remote.path_str(), dir.path()).unwrap();
        annotated_tag(&repo, "obsolete", "Local");
        let kind = SyncRefKind::Tags;
        let published = published_refs(&repo, ORIGIN, kind, None, None, false).unwrap();
        let first = preview(&repo, kind, &published);
        assert_eq!(first.remote_count, 0);
        assert_eq!(first.deletions[0].name, "obsolete");
        annotated_tag(&repo, "added-locally", "Also local");
        let ApplySyncRefsResult::Changed { preview: updated } =
            apply_preview(&repo, kind, &published, &first.revision, None).unwrap()
        else {
            panic!("expected refreshed comparison");
        };
        assert_eq!(updated.deletions.len(), 2);
        assert_eq!(local_refs(&repo, kind).unwrap().len(), 2);
        apply_preview(&repo, kind, &published, &updated.revision, None).unwrap();
        assert!(local_refs(&repo, kind).unwrap().is_empty());
    }

    #[test]
    fn branches_compare_untracked_refs_and_switch_before_deleting_current_branch() {
        let remote = TestRepo::with_initial_commit();
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::clone(&remote.path_str(), dir.path()).unwrap();
        let base = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = base.tree().unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let local = repo
            .commit(Some("HEAD"), &sig, &sig, "Local commit", &tree, &[&base])
            .unwrap();
        repo.branch("local/topic", &repo.find_commit(local).unwrap(), false)
            .unwrap();
        let dir_before = dir.path().join("README.md");
        std::fs::write(&dir_before, "uncommitted change").unwrap();
        let tracked = repo.refname_to_id("refs/remotes/origin/main").unwrap();
        let fetch_head = std::fs::read(repo.path().join("FETCH_HEAD")).ok();

        let latest = remote.create_commit("Remote commit", &[("README.md", "Remote files")]);
        let source = remote.repo();
        source
            .find_branch("main", git2::BranchType::Local)
            .unwrap()
            .rename("release", false)
            .unwrap();
        let kind = SyncRefKind::Branches;
        let published = published_refs(&repo, ORIGIN, kind, None, None, false).unwrap();
        let plan = preview(&repo, kind, &published);
        assert_eq!(plan.replacements, ["release"]);
        assert!(plan.current_branch_deleted && plan.working_changes);
        assert_eq!(
            plan.deletions
                .iter()
                .map(|c| (&*c.name, c.outgoing_commits))
                .collect::<Vec<_>>(),
            [("local/topic", 1), ("main", 1)]
        );
        assert_eq!(repo.head().unwrap().target(), Some(local));
        assert_eq!(
            std::fs::read_to_string(&dir_before).unwrap(),
            "uncommitted change"
        );
        assert_eq!(
            std::fs::read(repo.path().join("FETCH_HEAD")).ok(),
            fetch_head
        );
        assert!(apply_preview(&repo, kind, &published, &plan.revision, None).is_err());

        assert!(matches!(
            apply_preview(&repo, kind, &published, &plan.revision, Some("release")).unwrap(),
            ApplySyncRefsResult::Applied
        ));
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/release");
        assert_eq!(repo.head().unwrap().target(), Some(latest));
        assert_eq!(std::fs::read_to_string(dir_before).unwrap(), "Remote files");
        assert_eq!(
            repo.refname_to_id("refs/remotes/origin/main").unwrap(),
            tracked
        );
        assert_eq!(local_refs(&repo, kind).unwrap(), published.refs);
    }

    #[test]
    fn branches_block_another_worktree_and_can_overwrite_the_current_branch() {
        let remote = TestRepo::with_initial_commit();
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::clone(&remote.path_str(), dir.path()).unwrap();
        let base = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("in-use", &base, false).unwrap();
        let linked_dir = tempfile::tempdir().unwrap();
        let linked_path = linked_dir.path().join("linked");
        let branch = repo.find_reference("refs/heads/in-use").unwrap();
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&branch));
        let linked = repo
            .worktree("linked", &linked_path, Some(&options))
            .unwrap();
        let readme = dir.path().join("README.md");
        std::fs::write(&readme, "dirty").unwrap();
        let latest = remote.create_commit("Advance main", &[("README.md", "New contents")]);
        let kind = SyncRefKind::Branches;
        let published = published_refs(&repo, ORIGIN, kind, None, None, false).unwrap();
        let blocked = preview(&repo, kind, &published);
        assert!(blocked.deletions[0].worktree.is_some());
        assert!(blocked.working_changes);
        assert!(apply_preview(&repo, kind, &published, &blocked.revision, None).is_err());
        assert_eq!(repo.head().unwrap().target(), Some(base.id()));
        assert_eq!(std::fs::read_to_string(&readme).unwrap(), "dirty");
        let linked_repo = Repository::open_from_worktree(&linked).unwrap();
        linked_repo.set_head_detached(base.id()).unwrap();

        let plan = preview(&repo, kind, &published);
        assert!(plan.overwrites.iter().any(|change| change.name == "main"));
        apply_preview(&repo, kind, &published, &plan.revision, None).unwrap();
        assert_eq!(repo.head().unwrap().target(), Some(latest));
        assert_eq!(std::fs::read_to_string(readme).unwrap(), "New contents");
    }
}

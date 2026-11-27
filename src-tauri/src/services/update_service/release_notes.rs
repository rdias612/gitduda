use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use std::time::Duration;

use futures_util::{stream, StreamExt};
use regex::{Captures, Regex};
use reqwest::redirect::Policy;
use serde::Deserialize;

use crate::services::provider_http::{ensure_url_allowed, DEFAULT_PROVIDER_USER_AGENT};

const GITHUB_API: &str = "https://api.github.com";
const RELEASE_REPOSITORY: &str = "rdias612/gitduda";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
const PLACEHOLDER: &str = "See the assets below to download and install this version.";

static COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
static LIST_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[-*+]|[0-9]+[.)])\s+").unwrap());
static ISSUE_REF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)https://github\.com/([a-z0-9_.-]+/[a-z0-9_.-]+)/issues/([1-9][0-9]*)\b|\b([a-z0-9_.-]+/[a-z0-9_.-]+)#([1-9][0-9]*)\b|(?:^|[\s(\[*`])#([1-9][0-9]*)\b",
    )
    .unwrap()
});
static MARKDOWN_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\((https?://[^)]+)\)").unwrap());
static PULL_REQUEST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:\s+by\s+@\S+)?\s+in\s+https://github\.com/\S+/pull/\d+\s*$").unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IssueRef {
    repository: String,
    number: u64,
}

#[derive(Deserialize)]
struct Release {
    body: Option<String>,
}

#[derive(Deserialize)]
struct Issue {
    title: String,
    pull_request: Option<serde_json::Value>,
}

async fn get<T: serde::de::DeserializeOwned>(client: &reqwest::Client, path: &str) -> Option<T> {
    let url = format!("{GITHUB_API}/{path}");
    ensure_url_allowed(&url).ok()?;
    client
        .get(url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()
}

/// The release description may have been edited after the updater manifest was generated.
/// Its copy in the manifest still provides notes if GitHub's API is unavailable.
pub(super) async fn load(version: &str, manifest_notes: Option<&str>) -> Vec<String> {
    let client = reqwest::Client::builder()
        .user_agent(DEFAULT_PROVIDER_USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(Policy::none())
        .build()
        .ok();
    let release = match &client {
        Some(client) => {
            let tag = urlencoding::encode(version);
            get::<Release>(
                client,
                &format!("repos/{RELEASE_REPOSITORY}/releases/tags/v{tag}"),
            )
            .await
        }
        None => None,
    };
    let notes = match &release {
        Some(release) => release.body.as_deref(),
        None => manifest_notes,
    };
    let changes = change_entries(notes.unwrap_or_default());
    let mut titles = HashMap::new();

    if let Some(client) = &client {
        let mut seen = HashSet::new();
        let references = changes
            .iter()
            .flat_map(|change| ISSUE_REF.captures_iter(change))
            .filter_map(|reference| issue_ref(&reference))
            .filter(|issue| seen.insert(issue.clone()))
            .collect::<Vec<_>>();
        let mut issues = stream::iter(references)
            .map(|issue| async move {
                let path = format!("repos/{}/issues/{}", issue.repository, issue.number);
                let title = get::<Issue>(client, &path)
                    .await
                    .filter(|issue| issue.pull_request.is_none() && !issue.title.trim().is_empty())
                    .map(|issue| issue.title);
                (issue, title)
            })
            .buffer_unordered(6);
        let deadline = tokio::time::Instant::now() + REQUEST_TIMEOUT;
        while let Ok(Some((issue, title))) = tokio::time::timeout_at(deadline, issues.next()).await
        {
            if let Some(title) = title {
                titles.insert(issue, title);
            }
        }
    }

    format_changes(changes, &titles)
}

fn issue_ref(reference: &Captures<'_>) -> Option<IssueRef> {
    let (repository, number) = match (reference.get(1), reference.get(3)) {
        (Some(repo), _) => (repo.as_str(), &reference[2]),
        (_, Some(repo)) => (repo.as_str(), &reference[4]),
        _ => (RELEASE_REPOSITORY, &reference[5]),
    };
    Some(IssueRef {
        repository: repository.to_ascii_lowercase(),
        number: number.parse().ok()?,
    })
}

fn change_entries(notes: &str) -> Vec<String> {
    let notes = COMMENT.replace_all(notes, "");
    let mut entries: Vec<String> = Vec::new();
    let mut in_entry = false;
    for line in notes.lines() {
        let text = line.trim();
        if text.is_empty()
            || text.starts_with("# ")
            || text.starts_with("##")
            || text.starts_with("**Full Changelog**")
            || text == PLACEHOLDER
        {
            in_entry = false;
        } else if let Some(marker) = LIST_MARKER.find(text) {
            entries.push(text[marker.end()..].to_string());
            in_entry = true;
        } else if in_entry && line.starts_with(char::is_whitespace) {
            if let Some(entry) = entries.last_mut() {
                entry.push(' ');
                entry.push_str(text);
            }
        } else {
            entries.push(text.to_string());
            in_entry = false;
        }
    }
    entries
}

fn format_changes(changes: Vec<String>, titles: &HashMap<IssueRef, String>) -> Vec<String> {
    let mut formatted = Vec::new();
    for change in changes {
        let mut found_title = false;
        for issue in ISSUE_REF
            .captures_iter(&change)
            .filter_map(|reference| issue_ref(&reference))
        {
            if let Some(title) = titles.get(&issue) {
                formatted.push(format!("#{} — {title}", issue.number));
                found_title = true;
            }
        }
        if !found_title {
            let description = PULL_REQUEST.replace(&change, "");
            let description = MARKDOWN_LINK.replace_all(&description, |link: &Captures<'_>| {
                let label = &link[1];
                match ISSUE_REF
                    .captures(&link[2])
                    .and_then(|reference| issue_ref(&reference))
                {
                    Some(issue) if label != format!("#{}", issue.number) => {
                        format!("#{} — {label}", issue.number)
                    }
                    _ => label.to_string(),
                }
            });
            formatted.push(description.into_owned());
        }
    }
    let mut seen = HashSet::new();
    formatted.retain(|change| seen.insert(change.clone()));
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_referenced_issues_and_keeps_standalone_changes() {
        let notes = r#"## What's Changed
* Refresh the graph (closes #42) by @dev in https://github.com/rdias612/gitduda/pull/30
* [Keyboard fix](https://github.com/rdias612/gitduda/issues/43)
* Keep changes readable
  on narrow screens.
* Fix startup in https://github.com/rdias612/gitduda/pull/31
* #44 — Keep the title from the notes if the API is unavailable
* [Keep the reference and label](https://github.com/rdias612/gitduda/issues/45)
* Include a note about other/project#7

**Full Changelog**: https://github.com/rdias612/gitduda/compare/v1...v2"#;
        let titles = [
            ((RELEASE_REPOSITORY, 42), "Refresh the commit graph"),
            ((RELEASE_REPOSITORY, 43), "Restore keyboard focus"),
            (("other/project", 7), "Fix the dependency"),
        ]
        .into_iter()
        .map(|((repository, number), title)| {
            (
                IssueRef {
                    repository: repository.into(),
                    number,
                },
                title.into(),
            )
        })
        .collect();
        assert_eq!(
            format_changes(change_entries(notes), &titles),
            [
                "#42 — Refresh the commit graph",
                "#43 — Restore keyboard focus",
                "Keep changes readable on narrow screens.",
                "Fix startup",
                "#44 — Keep the title from the notes if the API is unavailable",
                "#45 — Keep the reference and label",
                "#7 — Fix the dependency",
            ]
        );
    }

    #[test]
    fn empty_or_placeholder_release_notes_produce_no_changes() {
        assert!(change_entries("").is_empty());
        assert!(change_entries("  \n<!-- add notes here -->\n## What's Changed").is_empty());
        assert!(change_entries(PLACEHOLDER).is_empty());
    }
}

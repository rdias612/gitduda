//! Test harness for commit message generation pipeline.
//!
//! Captures `git diff HEAD` from the real repo and runs it through the
//! extract_file_diffs → batch summarize → synthesize pipeline, printing
//! each stage so we can inspect quality.
//!
//! Run with:
//! ```sh
//! cd src-tauri && cargo test --test commit_message_harness -- --nocapture
//! ```

use std::process::Command;

// We can't access private functions directly, so we duplicate the extraction
// logic here for testing purposes.

struct FileDiff {
    path: String,
    content: String,
}

fn extract_file_diffs(diff: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            if let Some(path) = current_path.take() {
                files.push(FileDiff {
                    path,
                    content: std::mem::take(&mut current_lines).join("\n"),
                });
            }
            current_path = Some(path.to_string());
        } else if line.starts_with("--- ") || line.starts_with("diff ") {
            continue;
        } else if current_path.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(path) = current_path {
        files.push(FileDiff {
            path,
            content: current_lines.join("\n"),
        });
    }

    // Truncate each file's content
    let max_chars_per_file = if files.len() > 10 { 1500 } else { 2500 };
    for file in &mut files {
        if file.content.len() > max_chars_per_file {
            let end = file.content.floor_char_boundary(max_chars_per_file);
            file.content = file.content[..end].to_string();
            file.content.push_str("\n[truncated]");
        }
    }

    files
}

#[test]
fn test_extract_file_diffs_on_real_repo() {
    // Capture the real git diff
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output = Command::new("git")
        .args(["diff", "HEAD", "--no-color"])
        .current_dir(&repo_root)
        .output()
        .expect("Failed to run git diff");

    let diff = String::from_utf8_lossy(&output.stdout);

    if diff.is_empty() {
        eprintln!("No uncommitted changes — nothing to test.");
        return;
    }

    eprintln!("═══════════════════════════════════════════════════════════");
    eprintln!("  COMMIT MESSAGE GENERATION HARNESS");
    eprintln!("═══════════════════════════════════════════════════════════");
    eprintln!();
    eprintln!(
        "Raw diff: {} bytes, {} lines",
        diff.len(),
        diff.lines().count()
    );
    eprintln!();

    // Stage 1: Extract per-file diffs
    let file_diffs = extract_file_diffs(&diff);
    eprintln!("───────────────────────────────────────────────────────────");
    eprintln!("  STAGE 1: Extracted {} file diffs", file_diffs.len());
    eprintln!("───────────────────────────────────────────────────────────");
    for (i, fd) in file_diffs.iter().enumerate() {
        eprintln!("  [{:>2}] {} ({} chars)", i + 1, fd.path, fd.content.len());
    }
    eprintln!();

    // Stage 2: Show what each per-file prompt would look like (ALL files)
    eprintln!("───────────────────────────────────────────────────────────");
    eprintln!(
        "  STAGE 2: Per-file summarization ({} files, {} inference calls)",
        file_diffs.len(),
        file_diffs.len()
    );
    eprintln!("───────────────────────────────────────────────────────────");

    for (i, file_diff) in file_diffs.iter().enumerate() {
        eprintln!(
            "  [{:>2}] {} — {} chars of diff content",
            i + 1,
            file_diff.path,
            file_diff.content.len()
        );
    }

    // Stage 2b: Show condensing pass info
    if file_diffs.len() > 5 {
        eprintln!();
        eprintln!(
            "  → {} files > 5 — condensing pass will group into 3-6 bullets",
            file_diffs.len()
        );
    }

    // Stage 3: Show what the final synthesis prompt would look like
    // (using placeholder summaries since we don't have a real model here)
    eprintln!();
    eprintln!("───────────────────────────────────────────────────────────");
    eprintln!("  STAGE 3: Final synthesis");
    eprintln!("───────────────────────────────────────────────────────────");

    let placeholder_summaries: Vec<String> = file_diffs
        .iter()
        .map(|f| {
            let has_additions = f.content.contains("\n+");
            let has_deletions = f.content.contains("\n-");
            let action = match (has_additions, has_deletions) {
                (true, true) => "modify",
                (true, false) => "add to",
                (false, true) => "remove from",
                (false, false) => "update",
            };
            format!("- {} code", action)
        })
        .collect();

    let changes_text = placeholder_summaries.join("\n");
    eprintln!("  Placeholder per-file summaries:");
    for s in &placeholder_summaries {
        eprintln!("    {s}");
    }

    let synthesis_prompt = format!(
        "<|start_header_id|>system<|end_header_id|>\n\n\
         Write a git commit message for these changes. Output ONLY the commit message.\n\
         Line 1: type(scope): short summary (under 50 chars)\n\
         Line 2: blank\n\
         Line 3+: bullet points summarizing the key changes (group related items)\n\
         Types: feat, fix, docs, style, refactor, test, chore. Imperative mood. No quotes.<|eot_id|>\
         <|start_header_id|>user<|end_header_id|>\n\n{changes_text}<|eot_id|>\
         <|start_header_id|>assistant<|end_header_id|>\n\n"
    );

    eprintln!();
    eprintln!(
        "  Synthesis prompt size: {} chars (~{} tokens)",
        synthesis_prompt.len(),
        synthesis_prompt.len() / 4
    );

    eprintln!();
    eprintln!("═══════════════════════════════════════════════════════════");
    eprintln!("  SUMMARY");
    eprintln!("═══════════════════════════════════════════════════════════");
    eprintln!("  Files changed:     {}", file_diffs.len());
    let condense_calls = if file_diffs.len() > 5 { 1 } else { 0 };
    eprintln!(
        "  Inference calls:   {} (per-file) + {} (condense) + 1 (summary) = {}",
        file_diffs.len(),
        condense_calls,
        file_diffs.len() + condense_calls + 1
    );
    let total_content: usize = file_diffs.iter().map(|f| f.content.len()).sum();
    eprintln!(
        "  Total diff content: {} chars across batches",
        total_content
    );
    eprintln!();
}

//! File operations command handlers
//! Reveal files in file manager, open in default apps, and editor integration

use std::path::Path;
use tauri::command;

use super::path_utils::resolve_path_within_repo;
use crate::error::{GitdudaError, Result};
use crate::utils::create_command;

/// Escape cmd.exe metacharacters to prevent command injection (Windows only).
#[cfg(target_os = "windows")]
fn escape_cmd_meta(s: &str) -> String {
    s.chars()
        .map(|c| {
            if "&|<>^()%!".contains(c) {
                format!("^{}", c)
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Result of an open/reveal operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResult {
    pub success: bool,
    pub message: Option<String>,
    /// Something the user should know even though the open succeeded — e.g.
    /// that a fallback app was used instead of the configured editor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    /// Whether no editor is configured so the UI can guide the user to Settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_configured: Option<bool>,
}

/// Editor configuration from git config
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfig {
    /// core.editor setting
    pub editor: Option<String>,
    /// GIT_VISUAL or core.visual setting
    pub visual: Option<String>,
}

/// Information about a code editor preset and its availability on the current OS
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorInfo {
    pub id: String,
    pub display_name: String,
    pub command: String,
    pub available: bool,
}

#[cfg(target_os = "windows")]
fn windows_reveal_argument(target: &Path) -> std::ffi::OsString {
    use std::ffi::OsString;

    // Explorer parses its own command line and requires the switch outside
    // quotes with only the path quoted.
    let mut select_arg = OsString::from("/select,\"");
    select_arg.push(target.as_os_str());
    select_arg.push("\"");
    select_arg
}

#[cfg(target_os = "windows")]
fn windows_reveal_command(target: &Path) -> std::process::Command {
    use std::os::windows::process::CommandExt;

    // Deliberately NOT `create_command`: that returns the `GitCommand` wrapper,
    // whose whole purpose is reporting git runs to the Output panel, and which
    // exposes no `raw_arg` because no git invocation needs one. Explorer parses
    // its own command line, so the `/select,"…"` argument has to reach it
    // unquoted-then-quoted exactly as `windows_reveal_argument` builds it.
    // `create_command`'s only relevant side effect here was CREATE_NO_WINDOW,
    // reproduced below.
    let mut command = std::process::Command::new("explorer.exe");
    // CREATE_NO_WINDOW = 0x08000000, so revealing a file never flashes a
    // console window. Same flag `create_command` sets for every child.
    command.creation_flags(0x08000000);
    command.raw_arg(windows_reveal_argument(target));
    command
}

/// Run git config command to get a value
fn get_git_config_value(repo_path: Option<&Path>, key: &str, global: bool) -> Option<String> {
    get_git_config_value_with(repo_path, key, global, None)
}

fn get_git_config_value_with(
    repo_path: Option<&Path>,
    key: &str,
    global: bool,
    global_config_override: Option<&Path>,
) -> Option<String> {
    let mut cmd = create_command("git");

    if let Some(path) = repo_path {
        cmd.current_dir(path);
    }

    cmd.arg("config");
    if global {
        if let Some(cfg_path) = global_config_override {
            cmd.arg("--file").arg(cfg_path);
        } else {
            cmd.arg("--global");
        }
    }
    cmd.arg("--get");
    cmd.arg(key);

    let output = cmd.output().ok()?;
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    } else {
        None
    }
}

/// Run git config command to set a value
fn set_git_config_value_with(
    repo_path: Option<&Path>,
    key: &str,
    value: &str,
    global: bool,
    global_config_override: Option<&Path>,
) -> Result<()> {
    let mut cmd = create_command("git");

    if let Some(path) = repo_path {
        cmd.current_dir(path);
    }

    cmd.arg("config");
    if global {
        if let Some(cfg_path) = global_config_override {
            cmd.arg("--file").arg(cfg_path);
        } else {
            cmd.arg("--global");
        }
    } else {
        cmd.arg("--local");
    }
    cmd.arg(key);
    cmd.arg(value);

    let output = cmd
        .output()
        .map_err(|e| GitdudaError::OperationFailed(format!("Failed to run git config: {}", e)))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(GitdudaError::OperationFailed(format!(
            "Failed to set config: {}",
            stderr
        )))
    }
}

/// Reveal a file or folder in the system file manager
/// Unlike open_file_manager which opens a directory, this reveals a specific file
#[command]
pub async fn reveal_in_file_manager(path: String) -> Result<OpenResult> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err(GitdudaError::InvalidPath(path));
    }

    #[cfg(target_os = "windows")]
    {
        windows_reveal_command(target).spawn().map_err(|e| {
            GitdudaError::OperationFailed(format!("Failed to reveal in explorer: {}", e))
        })?;
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use open -R to reveal in Finder
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| {
                GitdudaError::OperationFailed(format!("Failed to reveal in Finder: {}", e))
            })?;
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, try dbus first for better integration, fall back to xdg-open for parent dir
        let revealed = try_dbus_reveal(target);
        if !revealed {
            // Fall back to opening the parent directory
            let parent = target.parent().unwrap_or(target);
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| {
                    GitdudaError::OperationFailed(format!(
                        "Failed to reveal in file manager: {}",
                        e
                    ))
                })?;
        }
    }

    Ok(OpenResult {
        success: true,
        message: None,
        notice: None,
        not_configured: None,
    })
}

/// Build the percent-encoded `file://` URI handed to FileManager1.ShowItems.
/// Returns `None` for paths that cannot be expressed as a file URL (for
/// example a relative path), so the caller can fall back to `xdg-open`.
#[cfg(target_os = "linux")]
fn dbus_reveal_uri(path: &Path) -> Option<String> {
    url::Url::from_file_path(path)
        .ok()
        .map(|url| url.to_string())
}

#[cfg(target_os = "linux")]
fn try_dbus_reveal(path: &Path) -> bool {
    // Try to use dbus to reveal file in file manager
    // This works with Nautilus, Dolphin, and other modern file managers
    let Some(uri) = dbus_reveal_uri(path) else {
        return false;
    };
    std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
            &format!("array:string:{}", uri),
            "string:",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Open a file with the system's default application
#[command]
pub async fn open_in_default_app(path: String) -> Result<OpenResult> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err(GitdudaError::InvalidPath(path));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &escape_cmd_meta(&path)])
            .spawn()
            .map_err(|e| GitdudaError::OperationFailed(format!("Failed to open file: {}", e)))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| GitdudaError::OperationFailed(format!("Failed to open file: {}", e)))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| GitdudaError::OperationFailed(format!("Failed to open file: {}", e)))?;
    }

    Ok(OpenResult {
        success: true,
        message: None,
        notice: None,
        not_configured: None,
    })
}

#[cfg(target_os = "windows")]
fn has_windows_executable_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "exe" | "com" | "cmd" | "bat"
            )
        })
        .unwrap_or(false)
}

#[cfg(any(target_os = "windows", test))]
fn select_windows_where_candidate_with(
    stdout: &str,
    is_file: impl Fn(&Path) -> bool,
) -> Option<String> {
    let mut fallback_script: Option<String> = None;

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let candidate = Path::new(line);
        if !is_file(candidate) {
            continue;
        }
        let ext = candidate
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        match ext.as_deref() {
            Some("exe") | Some("com") => return Some(line.to_string()),
            Some("cmd") | Some("bat") if fallback_script.is_none() => {
                fallback_script = Some(line.to_string());
            }
            _ => {}
        }
    }

    fallback_script
}

fn resolve_direct_path(trimmed: &str) -> Option<String> {
    let path = Path::new(trimmed);
    if path.is_file() {
        return Some(trimmed.to_string());
    }

    #[cfg(target_os = "windows")]
    if !has_windows_executable_extension(path) {
        for ext in ["exe", "com", "cmd", "bat"] {
            let candidate = std::path::PathBuf::from(format!("{trimmed}.{ext}"));
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }

    None
}

/// Resolve an editor command or path to an existing executable.
fn resolve_editor_executable(executable: &str) -> Option<String> {
    let trimmed = executable.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains('/') || trimmed.contains('\\') || Path::new(trimmed).is_absolute() {
        return resolve_direct_path(trimmed);
    }

    #[cfg(target_os = "windows")]
    {
        let output = create_command("where").arg(trimmed).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        select_windows_where_candidate_with(&stdout, |p| p.is_file())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = create_command("which").arg(trimmed).output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && Path::new(line).is_file())
            .map(|line| line.to_string())
    }
}

fn is_editor_executable_available(executable: &str) -> bool {
    resolve_editor_executable(executable).is_some()
}

/// Open a file in the configured editor with optional line number
#[command]
pub async fn open_in_configured_editor(
    path: String,
    #[allow(non_snake_case)] filePath: String,
    line: Option<u32>,
) -> Result<OpenResult> {
    open_in_configured_editor_with_deps(
        path,
        filePath,
        line,
        None,
        |name| std::env::var(name).ok(),
        resolve_editor_executable,
    )
    .await
}

async fn open_in_configured_editor_with_deps(
    path: String,
    file_path: String,
    line: Option<u32>,
    global_config_override: Option<&Path>,
    env_var: impl Fn(&str) -> Option<String>,
    resolve_executable: impl Fn(&str) -> Option<String>,
) -> Result<OpenResult> {
    let repo_path = Path::new(&path);
    let canonical_repo = repo_path.canonicalize().map_err(|e| {
        GitdudaError::OperationFailed(format!("Failed to resolve repo path: {}", e))
    })?;
    let requested_path = Path::new(&file_path);
    let target_path = if requested_path == repo_path || requested_path == canonical_repo.as_path() {
        if repo_path.is_absolute() {
            repo_path.to_path_buf()
        } else {
            canonical_repo
        }
    } else {
        resolve_path_within_repo(repo_path, &file_path)?
    };
    let target_file = target_path.to_string_lossy().to_string();

    let target = Path::new(&target_file);
    if !target.exists() {
        return Err(GitdudaError::InvalidPath(target_file));
    }

    // Get editor from git config (local first, then global)
    let editor = get_git_config_value(Some(repo_path), "core.editor", false)
        .or_else(|| get_git_config_value_with(None, "core.editor", true, global_config_override))
        .or_else(|| env_var("GIT_EDITOR").filter(|v| !v.trim().is_empty()))
        .or_else(|| env_var("VISUAL").filter(|v| !v.trim().is_empty()))
        .or_else(|| env_var("EDITOR").filter(|v| !v.trim().is_empty()));

    match editor {
        Some(editor_cmd) if is_terminal_editor(&editor_cmd) && target.is_dir() => {
            let mut result = open_in_default_app(target_file).await?;
            result.notice = Some(format!(
                "Git's core.editor ({editor_cmd}) runs in a terminal, so the repository \
                 directory was opened with the system default app instead."
            ));
            Ok(result)
        }
        Some(editor_cmd) if is_terminal_editor(&editor_cmd) => Ok(editor_unavailable(format!(
            "The configured editor ({editor_cmd}) requires a terminal and cannot open files \
             from Gitduda. Configure a GUI editor in Git settings."
        ))),
        Some(editor_cmd) => {
            // Parse the editor command and add line number support
            let (cmd, args) = parse_editor_command(&editor_cmd, &target_file, line);
            let Some(resolved_cmd) = resolve_executable(&cmd) else {
                return Err(GitdudaError::OperationFailed(format!(
                    "Configured editor executable '{}' could not be found. Verify it is installed and on your PATH or update Settings → External Tools.",
                    cmd
                )));
            };

            let mut command = std::process::Command::new(&resolved_cmd);
            command
                .args(&args)
                .current_dir(repo_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());

            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                // CREATE_NO_WINDOW = 0x08000000 prevents console flash for .cmd/.bat shims.
                command.creation_flags(0x08000000);
            }

            command.spawn().map_err(|e| {
                GitdudaError::OperationFailed(format!(
                    "Failed to open editor '{}': {}",
                    editor_cmd, e
                ))
            })?;

            Ok(OpenResult {
                success: true,
                message: Some(format!("Opened in {}", cmd)),
                notice: None,
                not_configured: None,
            })
        }
        None if target.is_dir() => Ok(OpenResult {
            success: false,
            message: Some(
                "No editor is configured (e.g., VS Code). Configure an editor in Settings."
                    .to_string(),
            ),
            notice: None,
            not_configured: Some(true),
        }),
        // Do not hand repository files to the OS default handler, which may
        // execute them; and do not fall back to the file manager for directories
        // when the user explicitly asked to open in an editor.
        None => Ok(no_editor_configured_for_file()),
    }
}

fn no_editor_configured_for_file() -> OpenResult {
    OpenResult {
        not_configured: Some(true),
        ..editor_unavailable(
            "No editor is configured. Configure a GUI editor in Git settings to open files.",
        )
    }
}

fn editor_unavailable(message: impl Into<String>) -> OpenResult {
    OpenResult {
        success: false,
        message: Some(message.into()),
        notice: None,
        not_configured: None,
    }
}

/// Whether `editor_cmd` (a `core.editor`-style command line) needs a terminal
/// to show anything. Includes no-op editors like `true`, which "succeed"
/// without opening anything at all.
fn is_terminal_editor(editor_cmd: &str) -> bool {
    let parts = split_command(editor_cmd);
    let Some(cmd) = parts.first() else {
        return false;
    };
    // Both separators, so a Windows path is recognised wherever it is parsed.
    let base = cmd.rsplit(['/', '\\']).next().unwrap_or(cmd).to_lowercase();
    let name = base
        .strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".cmd"))
        .or_else(|| base.strip_suffix(".bat"))
        .unwrap_or(&base);
    match name {
        "vi" | "vim" | "nvim" | "neovim" | "view" | "nano" | "pico" | "micro" | "hx" | "helix"
        | "kak" | "joe" | "ne" | "mg" | "jed" | "ed" | "vis" | "true" | ":" => true,
        // GUI unless told to stay in the terminal.
        "emacs" | "emacsclient" => parts[1..]
            .iter()
            .any(|a| matches!(a.as_str(), "-nw" | "-t" | "--tty" | "--no-window-system")),
        _ => false,
    }
}

/// Split a command string respecting quoted arguments.
/// Handles double-quoted and single-quoted strings (e.g., `"C:\Program Files\Code\Code.exe" --wait`).
fn split_command(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    for ch in input.chars() {
        match ch {
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            c if c.is_whitespace() && !in_double_quote && !in_single_quote => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            c => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Parse editor command and add line number argument based on editor type
fn parse_editor_command(
    editor_cmd: &str,
    file_path: &str,
    line: Option<u32>,
) -> (String, Vec<String>) {
    let parts = split_command(editor_cmd);
    if parts.is_empty() {
        return (editor_cmd.to_string(), vec![file_path.to_string()]);
    }

    let cmd = parts[0].clone();
    let mut args: Vec<String> = parts[1..].to_vec();

    // Detect editor type and add line number support
    let cmd_lower = cmd.to_lowercase();
    let cmd_name = Path::new(&cmd)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&cmd_lower)
        .to_lowercase();

    if let Some(line_num) = line {
        match cmd_name.as_str() {
            // VS Code, Cursor, and similar
            "code" | "cursor" | "code-insiders" | "codium" | "vscodium" => {
                args.push("--goto".to_string());
                args.push(format!("{}:{}", file_path, line_num));
                return (cmd, args);
            }
            // Vim-based editors
            "vim" | "nvim" | "neovim" | "vi" | "gvim" | "mvim" => {
                args.push(format!("+{}", line_num));
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // Emacs
            "emacs" | "emacsclient" => {
                args.push(format!("+{}", line_num));
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // Sublime Text
            "subl" | "sublime_text" | "sublime" => {
                args.push(format!("{}:{}", file_path, line_num));
                return (cmd, args);
            }
            // Atom (deprecated but still used)
            "atom" => {
                args.push(format!("{}:{}", file_path, line_num));
                return (cmd, args);
            }
            // JetBrains IDEs
            "idea" | "idea64" | "pycharm" | "webstorm" | "phpstorm" | "goland" | "clion"
            | "rider" | "rubymine" | "datagrip" => {
                args.push("--line".to_string());
                args.push(line_num.to_string());
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // Notepad++ (Windows)
            "notepad++" => {
                args.push(format!("-n{}", line_num));
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // Kate (KDE)
            "kate" => {
                args.push("--line".to_string());
                args.push(line_num.to_string());
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // Gedit (GNOME)
            "gedit" => {
                args.push(format!("+{}", line_num));
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // Nano
            "nano" => {
                args.push(format!("+{}", line_num));
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // TextMate
            "mate" => {
                args.push("--line".to_string());
                args.push(line_num.to_string());
                args.push(file_path.to_string());
                return (cmd, args);
            }
            // Helix
            "hx" | "helix" => {
                args.push(format!("{}:{}", file_path, line_num));
                return (cmd, args);
            }
            // Zed
            "zed" => {
                args.push(format!("{}:{}", file_path, line_num));
                return (cmd, args);
            }
            _ => {
                // Unknown editor, just pass the file
                args.push(file_path.to_string());
            }
        }
    } else {
        args.push(file_path.to_string());
    }

    (cmd, args)
}

/// Get the configured editor settings
#[command]
pub async fn get_editor_config(path: String, global: bool) -> Result<EditorConfig> {
    get_editor_config_with(path, global, None)
}

fn get_editor_config_with(
    path: String,
    global: bool,
    global_config_override: Option<&Path>,
) -> Result<EditorConfig> {
    let editor = if global {
        get_git_config_value_with(None, "core.editor", true, global_config_override)
    } else {
        let repo_path = Path::new(&path);
        // Try local first, then global as fallback
        get_git_config_value_with(Some(repo_path), "core.editor", false, None).or_else(|| {
            get_git_config_value_with(None, "core.editor", true, global_config_override)
        })
    };

    let visual = std::env::var("VISUAL")
        .ok()
        .or_else(|| std::env::var("GIT_VISUAL").ok());

    Ok(EditorConfig { editor, visual })
}

/// Set the configured editor
#[command]
pub async fn set_editor_config(path: String, editor: String, global: bool) -> Result<OpenResult> {
    set_editor_config_with(path, editor, global, None)
}

fn set_editor_config_with(
    path: String,
    editor: String,
    global: bool,
    global_config_override: Option<&Path>,
) -> Result<OpenResult> {
    let repo_path = if global {
        None
    } else {
        let p = Path::new(&path);
        if git2::Repository::open(p).is_err() {
            return Err(GitdudaError::RepositoryNotFound(path));
        }
        Some(p)
    };

    let trimmed = editor.trim();
    if trimmed.is_empty() {
        if global {
            if let Some(cfg_path) = global_config_override {
                let cfg_str = cfg_path.to_string_lossy();
                super::config::run_git_config_unset(
                    None,
                    &["--file", cfg_str.as_ref(), "--unset", "core.editor"],
                )?;
            } else {
                super::config::run_git_config_unset(None, &["--global", "--unset", "core.editor"])?;
            }
        } else {
            super::config::run_git_config_unset(repo_path, &["--local", "--unset", "core.editor"])?;
        }

        return Ok(OpenResult {
            success: true,
            message: Some(format!(
                "Editor cleared ({})",
                if global { "global" } else { "local" }
            )),
            notice: None,
            not_configured: None,
        });
    }

    set_git_config_value_with(
        repo_path,
        "core.editor",
        trimmed,
        global,
        global_config_override,
    )?;

    Ok(OpenResult {
        success: true,
        message: Some(format!(
            "Editor set to '{}' ({})",
            trimmed,
            if global { "global" } else { "local" }
        )),
        notice: None,
        not_configured: None,
    })
}

fn editor_presets_for_os() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut presets = vec![
        ("vscode", "VS Code", "code"),
        ("cursor", "Cursor", "cursor"),
        ("zed", "Zed", "zed"),
        ("sublime", "Sublime Text", "subl"),
    ];

    if cfg!(target_os = "windows") {
        presets.push(("notepad++", "Notepad++", "notepad++"));
    } else if cfg!(target_os = "macos") {
        presets.push(("textmate", "TextMate", "mate"));
    } else if cfg!(target_os = "linux") {
        presets.push(("kate", "Kate", "kate"));
        presets.push(("gedit", "Gedit", "gedit"));
    }

    presets
}

fn detect_available_editors_with(probe: impl Fn(&str) -> bool) -> Vec<EditorInfo> {
    editor_presets_for_os()
        .into_iter()
        .map(|(id, display_name, command)| EditorInfo {
            id: id.to_string(),
            display_name: display_name.to_string(),
            command: command.to_string(),
            available: probe(command),
        })
        .collect()
}

/// Get a list of commonly available code editors for the current OS
#[command]
pub async fn get_available_editors() -> Result<Vec<EditorInfo>> {
    Ok(detect_available_editors_with(
        is_editor_executable_available,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestRepo;

    #[test]
    fn test_split_command_simple() {
        let parts = split_command("code --wait");
        assert_eq!(parts, vec!["code", "--wait"]);
    }

    #[test]
    fn test_split_command_double_quoted_path() {
        let parts = split_command(r#""C:\Program Files\VS Code\Code.exe" --wait"#);
        assert_eq!(parts, vec![r"C:\Program Files\VS Code\Code.exe", "--wait"]);
    }

    #[test]
    fn test_split_command_single_quoted_path() {
        let parts = split_command("'/usr/local/bin/my editor' --line 5");
        assert_eq!(parts, vec!["/usr/local/bin/my editor", "--line", "5"]);
    }

    #[test]
    fn test_parse_editor_command_quoted_path() {
        let (cmd, args) = parse_editor_command(
            r#""C:\Program Files\VS Code\Code.exe" --wait"#,
            "/path/to/file.rs",
            Some(42),
        );
        assert_eq!(cmd, r"C:\Program Files\VS Code\Code.exe");
        assert!(args.contains(&"--wait".to_string()));
    }

    #[test]
    fn test_parse_editor_command_vscode() {
        let (cmd, args) = parse_editor_command("code", "/path/to/file.rs", Some(42));
        assert_eq!(cmd, "code");
        assert!(args.contains(&"--goto".to_string()));
        assert!(args.contains(&"/path/to/file.rs:42".to_string()));
    }

    #[test]
    fn terminal_editors_are_detected() {
        for editor in [
            "vim",
            "/usr/bin/nvim -f",
            "nano",
            "'C:\\Program Files\\Vim\\vim.exe'",
            "emacs -nw",
            "emacsclient -t",
            "true",
        ] {
            assert!(is_terminal_editor(editor), "{editor}");
        }
        for editor in [
            "code --wait",
            "gvim -f",
            "mvim",
            "emacs",
            "emacsclient -c",
            "subl -n -w",
            "",
        ] {
            assert!(!is_terminal_editor(editor), "{editor}");
        }
    }

    #[test]
    fn terminal_editor_fallback_reports_that_no_file_was_opened() {
        let result = editor_unavailable(
            "The configured editor (vim) requires a terminal and cannot open files from Gitduda.",
        );

        assert!(!result.success);
        assert!(result.message.unwrap().contains("requires a terminal"));
    }

    #[test]
    fn missing_editor_for_file_reports_configuration_guidance() {
        let result = no_editor_configured_for_file();

        assert!(!result.success);
        assert_eq!(result.not_configured, Some(true));
        assert!(result
            .message
            .as_deref()
            .unwrap()
            .contains("Configure a GUI editor"));
    }

    #[test]
    fn test_parse_editor_command_vim() {
        let (cmd, args) = parse_editor_command("vim", "/path/to/file.rs", Some(42));
        assert_eq!(cmd, "vim");
        assert!(args.contains(&"+42".to_string()));
        assert!(args.contains(&"/path/to/file.rs".to_string()));
    }

    #[test]
    fn test_parse_editor_command_sublime() {
        let (cmd, args) = parse_editor_command("subl", "/path/to/file.rs", Some(42));
        assert_eq!(cmd, "subl");
        assert!(args.contains(&"/path/to/file.rs:42".to_string()));
    }

    #[test]
    fn test_parse_editor_command_with_args() {
        let (cmd, args) = parse_editor_command("code --wait", "/path/to/file.rs", Some(42));
        assert_eq!(cmd, "code");
        assert!(args.contains(&"--wait".to_string()));
        assert!(args.contains(&"--goto".to_string()));
    }

    #[test]
    fn test_parse_editor_command_no_line() {
        let (cmd, args) = parse_editor_command("code", "/path/to/file.rs", None);
        assert_eq!(cmd, "code");
        assert!(args.contains(&"/path/to/file.rs".to_string()));
        assert!(!args.contains(&"--goto".to_string()));
    }

    #[test]
    fn test_parse_editor_command_unknown_editor() {
        let (cmd, args) = parse_editor_command("myeditor", "/path/to/file.rs", Some(42));
        assert_eq!(cmd, "myeditor");
        assert!(args.contains(&"/path/to/file.rs".to_string()));
    }

    #[tokio::test]
    async fn test_reveal_in_file_manager_invalid_path() {
        let result =
            reveal_in_file_manager("/nonexistent/path/that/does/not/exist".to_string()).await;
        assert!(result.is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_reveal_uses_single_select_argument() {
        let target = Path::new(r"C:\repo with spaces\src\main.rs");
        let argument = windows_reveal_argument(target);

        assert_eq!(argument, r#"/select,"C:\repo with spaces\src\main.rs""#);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_dbus_reveal_uri_percent_encodes_reserved_characters() {
        assert_eq!(
            dbus_reveal_uri(Path::new("/home/u/my repo/a b.txt")).as_deref(),
            Some("file:///home/u/my%20repo/a%20b.txt")
        );
        assert_eq!(
            dbus_reveal_uri(Path::new("/home/u/repo/a#b?c.txt")).as_deref(),
            Some("file:///home/u/repo/a%23b%3Fc.txt")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_dbus_reveal_uri_rejects_relative_path() {
        assert!(dbus_reveal_uri(Path::new("relative/a.txt")).is_none());
    }

    #[tokio::test]
    async fn test_open_in_default_app_invalid_path() {
        let result = open_in_default_app("/nonexistent/path/that/does/not/exist".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_open_in_configured_editor_invalid_path() {
        let repo = TestRepo::with_initial_commit();
        let result =
            open_in_configured_editor(repo.path_str(), "nonexistent_file.txt".to_string(), None)
                .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_open_in_configured_editor_accepts_repo_root() {
        let repo = TestRepo::with_initial_commit();
        repo.repo()
            .config()
            .unwrap()
            .set_str("core.editor", "missing-gitduda-test-editor")
            .unwrap();

        let result = open_in_configured_editor(repo.path_str(), repo.path_str(), None).await;

        assert!(matches!(result, Err(GitdudaError::OperationFailed(_))));
    }

    #[tokio::test]
    async fn test_open_in_configured_editor_rejects_absolute_path_outside_repo() {
        let repo = TestRepo::with_initial_commit_named("repo");
        let outside_path = repo.dir.path().join("outside.txt");
        std::fs::write(&outside_path, "outside").unwrap();
        repo.repo()
            .config()
            .unwrap()
            .set_str("core.editor", "missing-gitduda-test-editor")
            .unwrap();

        let result = open_in_configured_editor(
            repo.path_str(),
            outside_path.to_string_lossy().into_owned(),
            None,
        )
        .await;

        assert!(matches!(result, Err(GitdudaError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn test_get_editor_config() {
        let repo = TestRepo::with_initial_commit();
        let result = get_editor_config(repo.path_str(), false).await;
        assert!(result.is_ok());
        // Config may or may not have editor set
    }

    #[tokio::test]
    async fn test_set_editor_config() {
        let repo = TestRepo::with_initial_commit();

        // Set editor locally
        let result = set_editor_config(repo.path_str(), "code --wait".to_string(), false).await;
        assert!(result.is_ok());

        // Verify it was set
        let config = get_editor_config(repo.path_str(), false).await.unwrap();
        assert_eq!(config.editor, Some("code --wait".to_string()));
    }

    #[tokio::test]
    async fn test_set_editor_config_nonexistent_repo() {
        let result =
            set_editor_config("/nonexistent/repo".to_string(), "code".to_string(), false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_open_in_configured_editor_dir_without_editor_returns_not_configured() {
        let repo = TestRepo::with_initial_commit();

        let result = open_in_configured_editor_with_deps(
            repo.path_str(),
            repo.path_str(),
            None,
            None,
            |_| None,
            |_| None,
        )
        .await
        .unwrap();

        assert!(!result.success);
        assert_eq!(result.not_configured, Some(true));
        assert!(result
            .message
            .as_deref()
            .unwrap()
            .contains("No editor is configured"));
    }

    #[tokio::test]
    async fn test_open_in_configured_editor_ignores_empty_env_vars() {
        let repo = TestRepo::with_initial_commit();

        let result = open_in_configured_editor_with_deps(
            repo.path_str(),
            repo.path_str(),
            None,
            None,
            |_| Some("   ".to_string()),
            |_| None,
        )
        .await
        .unwrap();

        assert!(!result.success);
        assert_eq!(result.not_configured, Some(true));
    }

    #[tokio::test]
    async fn test_open_in_configured_editor_rejects_missing_executable_before_spawn() {
        let repo = TestRepo::with_initial_commit();

        let err = open_in_configured_editor_with_deps(
            repo.path_str(),
            repo.path_str(),
            None,
            None,
            |name| {
                if name == "EDITOR" {
                    Some("nonexistent-editor-bin --wait".to_string())
                } else {
                    None
                }
            },
            |_| None,
        )
        .await
        .unwrap_err();

        match err {
            GitdudaError::OperationFailed(msg) => {
                assert!(msg.contains("nonexistent-editor-bin"), "{msg}");
                assert!(msg.contains("could not be found"), "{msg}");
            }
            other => panic!("Expected OperationFailed, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_editor_executable_direct_path() {
        let dir = tempfile::tempdir().unwrap();
        let bin_path = dir.path().join("my-editor");
        std::fs::write(&bin_path, "stub").unwrap();

        let resolved = resolve_editor_executable(&bin_path.to_string_lossy());
        assert_eq!(
            resolved.as_deref(),
            Some(bin_path.to_string_lossy().as_ref())
        );

        let missing = dir.path().join("missing-editor");
        assert!(resolve_editor_executable(&missing.to_string_lossy()).is_none());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_resolve_editor_executable_direct_path_windows_cmd_extension() {
        let dir = tempfile::tempdir().unwrap();
        let shim_path = dir.path().join("code.cmd");
        std::fs::write(&shim_path, "@echo off").unwrap();

        let without_ext = dir.path().join("code");
        let resolved = resolve_editor_executable(&without_ext.to_string_lossy());
        assert_eq!(
            resolved.as_deref(),
            Some(shim_path.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn test_select_windows_where_candidate_ignores_posix_shim_and_prioritizes_exe() {
        let dir = tempfile::tempdir().unwrap();
        let posix_shim = dir.path().join("code");
        let cmd_shim = dir.path().join("code.cmd");
        let exe_bin = dir.path().join("Code.exe");
        std::fs::write(&posix_shim, "#!/bin/sh").unwrap();
        std::fs::write(&cmd_shim, "@echo off").unwrap();
        std::fs::write(&exe_bin, "MZ").unwrap();

        // When only POSIX shim + .cmd exist, ignores POSIX shim and selects .cmd
        let stdout_cmd = format!("{}\r\n{}\r\n", posix_shim.display(), cmd_shim.display());
        let selected_cmd = select_windows_where_candidate_with(&stdout_cmd, |p| p.is_file());
        assert_eq!(
            selected_cmd.as_deref(),
            Some(cmd_shim.to_string_lossy().as_ref())
        );

        // When .cmd appears before .exe, prioritizes .exe
        let stdout_exe = format!(
            "{}\r\n{}\r\n{}\r\n",
            posix_shim.display(),
            cmd_shim.display(),
            exe_bin.display()
        );
        let selected_exe = select_windows_where_candidate_with(&stdout_exe, |p| p.is_file());
        assert_eq!(
            selected_exe.as_deref(),
            Some(exe_bin.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn test_detect_available_editors_with_marks_availability_and_os_presets() {
        let editors = detect_available_editors_with(|cmd| cmd == "code" || cmd == "zed");

        let code = editors.iter().find(|e| e.command == "code").unwrap();
        assert_eq!(code.display_name, "VS Code");
        assert!(code.available);

        let zed = editors.iter().find(|e| e.command == "zed").unwrap();
        assert!(zed.available);

        let cursor = editors.iter().find(|e| e.command == "cursor").unwrap();
        assert!(!cursor.available);

        assert!(editors.iter().any(|e| e.command == "subl"));

        if cfg!(target_os = "windows") {
            assert!(editors.iter().any(|e| e.command == "notepad++"));
            assert!(!editors.iter().any(|e| e.command == "mate"));
        } else if cfg!(target_os = "macos") {
            assert!(editors.iter().any(|e| e.command == "mate"));
            assert!(!editors.iter().any(|e| e.command == "notepad++"));
        } else if cfg!(target_os = "linux") {
            assert!(editors.iter().any(|e| e.command == "kate"));
            assert!(editors.iter().any(|e| e.command == "gedit"));
            assert!(!editors.iter().any(|e| e.command == "notepad++"));
        }
    }

    #[test]
    fn test_global_editor_config_works_without_repo_and_unsets_on_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("gitconfig");
        std::fs::write(&cfg_path, "").unwrap();

        let set_res = set_editor_config_with(
            String::new(),
            "code --wait".to_string(),
            true,
            Some(&cfg_path),
        )
        .unwrap();
        assert!(set_res.success);

        let got = get_editor_config_with(String::new(), true, Some(&cfg_path)).unwrap();
        assert_eq!(got.editor.as_deref(), Some("code --wait"));

        let clear_res =
            set_editor_config_with(String::new(), "   ".to_string(), true, Some(&cfg_path))
                .unwrap();
        assert!(clear_res.success);

        let after_clear = get_editor_config_with(String::new(), true, Some(&cfg_path)).unwrap();
        assert_eq!(after_clear.editor, None);
    }

    #[test]
    fn test_local_editor_config_unsets_on_empty() {
        let repo = TestRepo::with_initial_commit();

        set_editor_config_with(repo.path_str(), "subl".to_string(), false, None).unwrap();
        let got = get_editor_config_with(repo.path_str(), false, None).unwrap();
        assert_eq!(got.editor.as_deref(), Some("subl"));

        set_editor_config_with(repo.path_str(), String::new(), false, None).unwrap();
        let after_clear = get_editor_config_with(repo.path_str(), false, None).unwrap();
        assert_eq!(after_clear.editor, None);
    }
}

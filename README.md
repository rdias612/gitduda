<div align="center">

# Gitduda

**Local-first Git client with AI, MCP, and zero telemetry.**

[![Build & Release](https://github.com/rdias612/gitduda/actions/workflows/build.yml/badge.svg)](https://github.com/rdias612/gitduda/actions/workflows/build.yml)
[![CI](https://github.com/rdias612/gitduda/actions/workflows/ci.yml/badge.svg)](https://github.com/rdias612/gitduda/actions/workflows/ci.yml)
[![GitHub release](https://img.shields.io/github/v/release/rdias612/gitduda)](https://github.com/rdias612/gitduda/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](https://github.com/rdias612/gitduda/releases)

[Website](https://gitduda.dev.br) · [Releases](https://github.com/rdias612/gitduda/releases) · [Why Gitduda](#why-gitduda) · [At a Glance](#at-a-glance) · [Installation](#installation) · [Build & Dev](#build--development)

<br />

</div>

Gitduda is a native, cross-platform desktop Git client built with **Tauri 2.0, Rust, Lit 3, and TypeScript**: a fast visual commit graph, a 3-way merge conflict editor, four forge integrations, on-device semantic commit search, and a built-in MCP server — no accounts, no subscriptions, no telemetry.

> [!NOTE]
> **Zero telemetry by design.** Everything runs locally on your machine. The only network traffic is Git push/pull/fetch, integrations and cloud AI providers you explicitly configure, and the opt-in **Show Avatars** setting — off by default, it sends an MD5 hash of commit author emails to gravatar.com, and is blocked whenever Offline Mode or the remote allowlist is active.

---

## Why Gitduda?

Gitduda started as an active fork of [Gitnado](https://github.com/hegsie/gitnado) by Ben Hegarty, sharpening the daily developer loop and slimming the runtime while staying fully independent:

- **Full Side-by-Side Isolation** — Uses dedicated config/keyring namespaces, `gitduda://` deep links, and bundle ID `io.github.rdias612.gitduda`, so it runs alongside Gitnado with zero state or credential collisions.
- **Leaner AI Runtime** — Removed the embedded `llama-cpp-2`/GGUF engine for faster builds and smaller binaries. AI features connect to **6 pluggable providers** (local: **Ollama**, **LM Studio** · cloud: **OpenAI**, **Anthropic**, **GitHub Models**, **Google Gemini**), while semantic commit search stays 100% on-device with pure-Rust **Candle** + `sqlite-vec` (Metal on Apple Silicon, automatic CPU fallback).
- **Instant Editor Round-Trips** — Global `Ctrl+Shift+A` (`Cmd+Shift+A` on macOS) shortcut opens the active repository in your configured external editor, with automatic editor detection in Settings (`open-in-editor`).
- **Sharper Stage-to-Commit Ergonomics** — Explicit **Stage All / Unstage All** buttons, multi-file selection in context menus, and a default commit message pre-filled from staged files.
- **Workspace & Tag Agility** — Switch workspaces directly from the repository selector, and create branches straight from any tag's context menu.
- **Safer In-App Updates** — Prompts for explicit confirmation before downloading or installing updates.
- **Streamlined Windows Dev Tooling** — Ships [`run-dev.ps1`](run-dev.ps1) (`-Release`, `-Install`, `-Check`) and [`run-dev.bat`](run-dev.bat) for one-command local builds.

### At a Glance

| Capability | Gitduda | GitHub Desktop | GitKraken | Sourcetree |
| :--- | :---: | :---: | :---: | :---: |
| Zero telemetry / no account required | ✅ | ❌ | ❌ | ❌ |
| Open source (MIT) | ✅ | ✅ | ❌ | ❌ |
| Local AI providers (Ollama / LM Studio) | ✅ | ❌ | ❌ (cloud only) | ❌ |
| On-device semantic commit search | ✅ | ❌ | ❌ | ❌ |
| AI conflict resolution & changelog | ✅ | ❌ | ❌ | ❌ |
| Built-in MCP server (Bearer auth) | ✅ | ❌ | ❌ | ❌ |
| Git worktrees UI | ✅ | ❌ | ❌ | ❌ |
| 4-mode image diff viewer | ✅ | ❌ | ❌ | ❌ |
| Interactive rebase UI | ✅ | ❌ | ✅ | ✅ |
| 3-way merge conflict editor | ✅ | ❌ | ✅ | ✅ |

---

## Essential Features

| Area | Capabilities |
| :--- | :--- |
| **Core Git & History** | Interactive commit graph, branches, tags, remotes, stashes, amend, interactive rebase, cherry-pick, visual reflog undo/redo, worktrees, submodules, and Git LFS |
| **Diff, Merge & Terminal** | Hunk-level staging/unstaging, Shiki syntax highlighting, 4-mode image diff viewer (*side-by-side*, *onion-skin*, *swipe*, *difference*), 3-way merge conflict editor, and integrated `@xterm/xterm` terminal |
| **AI & Semantic Search** | 6 configurable providers (**Ollama**, **LM Studio**, **OpenAI**, **Anthropic**, **GitHub Models**, **Google Gemini**) for commit messages, 3-way conflict suggestions, and changelogs; on-device Candle + `sqlite-vec` semantic search |
| **MCP Server** | Built-in Model Context Protocol server (**Settings → MCP Server**) secured with `Authorization: Bearer <token>`, consent-gated write tools (off by default), and `--mcp-stdio` support — full tool catalog in [docs/mcp-server.md](docs/mcp-server.md) |
| **Profiles & Forges** | Unified work/personal identities, GPG signing, SSH key management, chunked OS keyring storage, and **GitHub / GitLab / Azure DevOps / Bitbucket** integrations with URL-based profile auto-switching |
| **Localization** | Runtime language switching via `@lit/localize` (`en` and `fr` today, incremental rollout) |

<details>
<summary><strong>Quick-connect your AI coding assistant via MCP (Claude Code, Cursor, VS Code)</strong></summary>

1. Open **Settings → MCP Server** and enable the server.
2. Copy the generated Bearer token (the write tools — staging and commits, branches, tags, stashes, worktrees, file edits, reset and revert — are **off by default** and require per-call consent or branch policy approval; see [docs/mcp-server.md](docs/mcp-server.md) for the full catalog and examples).
3. Paste the client snippet generated in Settings:

```bash
# Claude Code (HTTP transport)
claude mcp add --transport http gitduda http://127.0.0.1:<port>/mcp \
  --header 'Authorization: Bearer <token>'
```

```json
// Cursor (.cursor/mcp.json)
{
  "mcpServers": {
    "gitduda": {
      "url": "http://127.0.0.1:<port>/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

```json
// VS Code (.vscode/mcp.json)
{
  "servers": {
    "gitduda": {
      "type": "http",
      "url": "http://127.0.0.1:<port>/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

</details>

---

## Installation

| Platform | Minimum OS / Arch | Installers ([Releases](https://github.com/rdias612/gitduda/releases)) | Package Manager |
| :--- | :--- | :--- | :--- |
| **macOS** | — | Not currently supported | — |
| **Windows** | Windows 10+ (x64) | `.msi`, `.exe` | `winget install rdias612.Gitduda` |
| **Linux** | x64 with GTK 3.24+ | `.deb`, `.AppImage`, `.rpm` | Download from Releases |

```powershell
# Windows (winget or Scoop)
winget install rdias612.Gitduda
scoop install https://raw.githubusercontent.com/rdias612/gitduda/main/packaging/scoop/gitduda.json
```

> [!NOTE]
> **macOS (Apple) Support:** Prebuilt macOS (Apple) builds are not currently supported.

> [!NOTE]
> **Linux Keyring Requirement:** Credentials are stored via `keyutils` backed by the freedesktop Secret Service API. An unlocked provider (such as **GNOME Keyring** or **KWallet**) must be running, or saving credentials will fail.

### First Launch in 3 Steps

1. **Open or Clone** — Press `Cmd/Ctrl+O` to open a repo, or switch workspaces directly from the repository selector.
2. **Stage & Commit** — Use `Cmd/Ctrl+P` for the Command Palette, **Stage All / Unstage All**, hunk staging, or multi-select context actions, then review the auto-filled or AI-generated commit message and press `Cmd/Ctrl+Enter`.
3. **Jump Straight to Code** — Press **`Ctrl+Shift+A`** (**`Cmd+Shift+A`** on macOS) to open the active repository in your configured external editor (press `?` anytime for all shortcuts).

---

## Build & Development

Gitduda is built with **Tauri 2.0**, **Rust (`1.95+`)**, **Lit 3**, **TypeScript**, **Vite**, **Zustand**, **Shiki**, **`@xterm/xterm`**, and **SQLite + `sqlite-vec` + Candle**.

### Prerequisites

- **Node.js 20+**, **Rust 1.95+**, and **Git 2.20+**
- **Windows**: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (*Desktop development with C++*) plus NASM and LLVM — [`run-dev.ps1`](run-dev.ps1) locates them automatically
- **macOS** *(source builds only; prebuilt releases are not currently supported)*: Xcode Command Line Tools (`xcode-select --install`)
- **Linux (Debian/Ubuntu)**: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libgtk-3-dev libglib2.0-dev libdbus-1-dev`

### Running & Testing Locally

```bash
git clone https://github.com/rdias612/gitduda.git
cd gitduda
npm install
npm run tauri:dev            # Start app in dev mode (builds go to src-tauri/target/release/bundle/)
```

On **Windows**, the helper scripts bootstrap dependencies and native toolchains automatically (`run-dev.bat` works from Command Prompt with the same flags):

```powershell
.\run-dev.ps1                 # Install deps if needed and launch tauri:dev
.\run-dev.ps1 -Check          # Run typecheck & lint before launching
.\run-dev.ps1 -Release        # Build and launch release binary (--no-bundle)
.\run-dev.ps1 -Install        # Force fresh npm install before launching
```

```bash
# Verification suite
npm run typecheck             # TypeScript checks (app + E2E)
npm run lint                  # ESLint (src + e2e)
npm test                      # Contract tests + Web Test Runner unit suite
npm run test:e2e              # Playwright E2E suite
cd src-tauri && cargo fmt --check && cargo clippy && cargo test
```

### Project Documentation

- [Packaging & Release Automation](docs/packaging.md) — Homebrew, winget, Scoop, and MSIX pipelines
- [Microsoft Store Web Installer](docs/microsoft-store-web-installer.md) — Store web installer setup details
- [OAuth Setup](docs/oauth-setup.md) — Forge OAuth apps and `gitduda://` callbacks
- [Localisation Guide](docs/localisation.md) — Adding or updating `@lit/localize` translations (`en`, `fr`)
- [Website Maintenance](docs/website.md) — Managing [gitduda.dev.br](https://gitduda.dev.br)

---

## Contributing, License & Acknowledgments

Contributions are welcome! Fork the repo, create a branch (`feat/your-feature`), verify with `npm run typecheck`, `npm run lint`, and `npm test` (see [.github/copilot-instructions.md](.github/copilot-instructions.md) for conventions), and open a Pull Request against `main`.

- **License**: [MIT License](LICENSE) (`Copyright (c) 2025 Ben Hegarty`, `Copyright (c) 2026 Rian Dias`).
- **Acknowledgments**: Forked from **[Gitnado](https://github.com/hegsie/gitnado)** by Ben Hegarty, and built on **[Tauri](https://tauri.app/)**, **[libgit2](https://libgit2.org/)**, **[Lit](https://lit.dev/)**, **[Shiki](https://shiki.style/)**, **[Candle](https://github.com/huggingface/candle)**, and **`sqlite-vec`**.

# GEMINI.md - OnetCli Instructional Context

This file serves as the foundational mandate for all AI interactions within the OnetCli project. It defines the project's architecture, development standards, and operational workflows.

## Project Overview

**OnetCli** (One Net Client) is a high-performance, cross-platform desktop client for databases, SSH, terminals, and AI tools.
- **Tech Stack:** Built with [GPUI](https://gpui.rs) (the GPU-accelerated UI framework from the Zed editor), written in Rust (2024 edition).
- **Core Features:**
    - **Databases:** PostgreSQL, MySQL, SQLite, SQL Server, Oracle, ClickHouse, DuckDB.
    - **NoSQL:** Redis and MongoDB explorers.
    - **Infrastructure:** Integrated SSH terminal, SFTP file manager, and Serial port support.
    - **AI Assistant:** LLM-powered natural language to SQL, query explanation, and BI analysis.
    - **Cloud Sync:** Encrypted synchronization of connections and settings.

## Workspace Structure

The workspace follows a modular crate-based architecture:

- **`main/`**: Application entry point (`main/src/main.rs`). Orchestrates all subsystems.
- **`crates/core` (one-core)**: Business logic, cloud sync, AI integration, encryption, and tab management.
- **`crates/ui` (gpui-component)**: A reusable library of 60+ UI components.
- **`crates/one_ui`**: Application-specific UI components extending `gpui-component`.
- **Feature Crates**: Paired as `[feature]` (backend) and `[feature]_view` (UI).
    - Database: `db` / `db_view`
    - Terminal: `terminal` / `terminal_view`
    - SSH/SFTP: `ssh`, `sftp` / `sftp_view`
    - Redis: `redis_view`
    - MongoDB: `mongodb_view`

## Building and Running

### Prerequisites
- **macOS/Linux:** Run `./script/bootstrap`
- **Windows:** Run `.\script\install-window.ps1` (PowerShell)

### Core Commands
- **Run App:** `cargo run -p main`
- **Build:** `cargo build`
- **Test All:** `cargo test --all`
- **Lint:** `cargo clippy -- --deny warnings`
- **Format:** `cargo fmt --check`
- **Unused Deps:** `cargo machete`
- **Component Gallery:** `cargo run` (launches `crates/story`)

## Development Mandates

### 1. Language and Style
- **Code Identifiers:** Always use English (variables, functions, classes).
- **AI Responses/Comments/Docs/Git:** Always use **Simplified Chinese (简体中文)**.
- **Commit Messages:** Follow a concise "Why" over "What" style, written in Simplified Chinese.

### 2. Engineering Standards (Hard Limits)
- **Function Length:** ≤ 50 lines (excluding empty lines).
- **File Size:** ≤ 300 lines.
- **Nesting Depth:** ≤ 3 layers.
- **Parameters:** Position parameters ≤ 3.
- **Complexity:** Cyclomatic complexity ≤ 10 per function.
- **Magic Numbers:** Strictly forbidden; extract to named constants.

### 3. Testing Strategy
Categorize changes and apply the corresponding validation level:
- **Level 0 (Direct):** Minor UI/Style tweaks. Direct modification + manual verification.
- **Level 1 (Regression):** Small bug fixes. Fix first, then add a regression test.
- **Level 2 (TDD):** New features, public APIs, or high-risk logic. Use `test-driven-development` skill.
- **Level 3 (Review):** Significant changes. Must undergo `requesting-code-review`.
- **Level 4 (Verification):** Mandatory for ALL completions. Use `verification-before-completion`.

### 4. Application Initialization Sequence
Order matters in `main/src/onetcli_app.rs`:
1. `gpui_component::init(cx)` — **Must be first** before any UI usage.
2. `one_core::init(cx)` and `one_ui::init(cx)`.
3. Feature view initializations (`db_view`, `terminal_view`, etc.).
4. `TabContentRegistry` and `Root` view setup.

## Key Architecture Patterns

- **Root View:** Every window's outermost view must be a `Root` (manages dialogs, notifications, and focus).
- **Dock System:** Panel layout with drag-and-drop support (`DockArea` -> `DockItem`).
- **Tab Container:** Manages multi-tab workflows via a global `TabContentRegistry`.
- **Stateless Components:** Prefer `RenderOnce` trait for UI components.
- **Theme System:** Global singleton accessed via `cx.theme()`. Supports light/dark modes.
- **Input System:** Based on `ropey` with LSP and tree-sitter integration.

## Communication Protocol

- **Status Reporting:** Use "Execution Progress" pattern (🎯 Task, 📋 Plan, ✅/🔄/⏸ status, 🛠️ Progress).
- **Analysis:** Use "Analysis Answer" pattern (✅ Conclusion, 🧠 Key Analysis, 🔍 Deep Dive).
- **Sub-agents:** Delegate repetitive or high-output tasks to `generalist` or `codebase_investigator`. Use `gpt-5.4` with `high/xhigh` reasoning effort.

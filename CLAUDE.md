# CLAUDE.md

## Project Overview

`openapi-tui` is a terminal UI application for browsing and running APIs defined with OpenAPI v3.0/v3.1 specifications. Built with Rust using `ratatui` for the TUI layer and `tokio` for async runtime.

## Build & Development Commands

```bash
# Build
cargo build

# Run tests
cargo test --all-features --workspace

# Format code
cargo fmt --all

# Lint (CI treats warnings as errors)
cargo clippy --all-targets --all-features --workspace -- -D warnings

# Check documentation
cargo doc --no-deps --document-private-items --all-features --workspace --examples

# Run locally
cargo run -- -i examples/petstore.json
cargo run -- -i path/to/spec.yml
```

## CI Checks (all must pass)

The CI pipeline (`.github/workflows/ci.yml`) runs on every push to `main` and all PRs:

1. **Test** – `cargo test --all-features --workspace`
2. **Rustfmt** – `cargo fmt --all --check`
3. **Clippy** – `cargo clippy --all-targets --all-features --workspace -- -D warnings`
4. **Docs** – `cargo doc --no-deps --document-private-items --all-features --workspace --examples`

All CI jobs use the **nightly** Rust toolchain.

## Architecture

```
src/
  main.rs          # Entry point, tokio async runtime
  app.rs           # Main application loop, event handling
  cli.rs           # CLI argument parsing (clap)
  tui.rs           # Terminal setup/teardown, Frame type alias
  state.rs         # Application state
  action.rs        # Actions dispatched through the app
  config.rs        # Configuration loading
  request.rs       # HTTP request logic
  response.rs      # HTTP response handling
  pages/           # Full-screen page layouts
    home.rs        # Main desktop layout
    phone.rs       # Compact/phone layout
  panes/           # Individual UI panes (widgets)
    apis.rs        # API list pane
    tags.rs        # Tags pane
    header.rs      # Header pane
    footer.rs      # Footer/command input pane
    request.rs     # Request details pane
    response.rs    # Response details pane
    response_viewer.rs  # Response body viewer
    body_editor.rs  # Request body editor
    address.rs     # URL/address pane
    parameter_editor.rs  # Parameter editor pane
    history.rs     # Request history pane
  components/
    schema_viewer.rs  # YAML schema viewer with syntax highlighting
```

## Key Dependencies

- `ratatui 0.30.0` – TUI framework
- `crossterm 0.29.0` – Terminal backend (must stay in sync with ratatui)
- `ratatui-textarea 0.8.0` – Multi-line text editor widget (replaces `tui-textarea`)
- `tui-input 0.15.0` – Single-line input widget
- `syntect 5.2.0` – Syntax highlighting for schema viewer
- `tokio` – Async runtime (full features)
- `reqwest` – HTTP client for running API calls
- `openapi-31` – OpenAPI spec parsing

## Dependency Notes

- **crossterm** must match the version used by ratatui's crossterm backend. ratatui 0.30.0 uses crossterm 0.29.0.
- **ratatui-textarea** (`ratatui-textarea = "0.8.0"`) is the ratatui-org maintained fork of `tui-textarea`, updated for ratatui 0.30+. Use `ratatui_textarea::TextArea` in imports.
- **syntect-tui** was removed; its style translation logic is inlined in `schema_viewer.rs` to avoid ratatui version conflicts.
- In ratatui 0.30.0, `Block`, `BorderType`, `Borders`, `Padding` moved out of `ratatui::widgets::block` and are now exported directly from `ratatui::widgets`. Do not use `widgets::{block::*, *}`.

## Code Conventions

- 2-space indentation
- `use ratatui::{prelude::*, widgets::*}` is the standard ratatui import pattern
- Panes implement the `Pane` trait; pages compose panes via layout splits
- Error handling uses `color_eyre::eyre::Result`

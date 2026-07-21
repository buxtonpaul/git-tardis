# Git-tardis

A tool for time travelling through your git repositories, allowing you to navigate through your commit history and trace the evolution of your code line-by-line and function-by-function.

## Features

- **Timeline Navigation**: View files with instant navigation to previous/next commits affecting the current line, enclosing function, or entire file.
- **Inline Rewrite**: Perform inline fixup edits applied automatically via Git autosquash rebases.
- **Edit Here**: Trigger interactive rebase pausing at any historical commit to allow shell/editor adjustments before continuing.
- **Neovim RPC Integration**: Seamlessly launched as a floating window within Neovim, inheriting active color themes via RPC.
- **High Performance TUI**: Fast syntax highlighting and split navigation built with Rust, Ratatui, Crossterm, and Tree-sitter.
- **Cross-Platform**: macOS and Linux support.

---

## Technical Specifications & Research

The design and architecture of Git-tardis are backed by validated proofs of concept and technical specifications:

| Focus Area | Specification Document | Proof-of-Concept | Key Decision / Outcome |
| :--- | :--- | :--- | :--- |
| **Neovim RPC Integration** | [`docs/research/neovim-rpc-terminal-integration.md`](docs/research/neovim-rpc-terminal-integration.md) | [`research/neovim-rpc-poc/`](research/neovim-rpc-poc/) | Bidirectional socket loop via `nvim-rs` + Tokio, using `BufWipeout` autocommands for cleanup. |
| **Tree-sitter Enclosing Scope** | [`docs/research/tree-sitter-function-parsing.md`](docs/research/tree-sitter-function-parsing.md) | [`tree-sitter-poc/`](research/tree-sitter-poc/) | Dynamic cursor point matching with bottom-up AST traversal across Rust, Go, Python, and TS. |
| **Ratatui Sidebar & Navigation** | [`docs/research/ratatui-sidebar-layout.md`](docs/research/ratatui-sidebar-layout.md) | [`ratatui-layout-poc/`](research/ratatui-layout-poc/) | Monolithic state model with percentage horizontal splits and scope-routed key events. |
| **Interactive Rebase Engine** | [`docs/research/interactive-rebase-modifications.md`](docs/research/interactive-rebase-modifications.md) | [`interactive-rebase-poc/`](research/interactive-rebase-poc/) | Non-interactive fixup rebases vs `GIT_SEQUENCE_EDITOR` marked edits with TUI suspension loops. |
| **Conflict Resolution Flow** | [`docs/research/conflict-detection-and-resolution.md`](docs/research/conflict-detection-and-resolution.md) | Integrated in rebase PoC | Fail-fast auto-abort with autostash recovery for inline edits; subshell delegation for edit-here. |
| **Neovim Lua Plugin Launcher** | [`docs/research/neovim-plugin-launcher-packaging.md`](docs/research/neovim-plugin-launcher-packaging.md) | [`neovim-plugin-poc/`](research/neovim-plugin-poc/) | `git-tardis.nvim` floating window wrapper passing `$NVIM` environment variable. |
| **Syntax Highlighting Engine** | [`docs/research/syntax-highlighting-performance.md`](docs/research/syntax-highlighting-performance.md) | [`syntax-highlighting-poc/`](research/syntax-highlighting-poc/) | Tree-sitter query highlighting (~4.5x–10x faster than `syntect`), viewport cropping, span merging (60% reduction), and Neovim RPC theme syncing. |

---

## Wayfinder Map

See [Issue #1 (Wayfinder Map)](https://github.com/buxtonpaul/git-tardis/issues/1) for the overall roadmap and technical specifications tracking.

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

## Installation & Quick Start

### Option 1: Homebrew (macOS & Linux)

```bash
brew tap buxtonpaul/tap
brew install git-tardis
```

### Option 2: Build from Source

Ensure you have the Rust toolchain installed:

```bash
git clone https://github.com/buxtonpaul/git-tardis.git
cd git-tardis
cargo build --release
cargo install --path .
```

Verify installation:

```bash
git-tardis --version
```

For full installation details and Neovim plugin configuration (`git-tardis.nvim`), see [docs/installation.md](docs/installation.md) and [docs/neovim-plugin.md](docs/neovim-plugin.md).

---

## Basic Usage

Launch Git-tardis in any Git repository:

```bash
# Launch in current repository
git-tardis

# Focus on a specific file and line on startup
git-tardis -f src/main.rs -l 42
```

Press `?` inside Git-tardis at any time to open the interactive keybindings help screen.

---

## Technical Specifications & Research

The design and architecture of Git-tardis are backed by validated proofs of concept and technical specifications (see [docs/technical-specification.md](docs/technical-specification.md) for full architectural specs and historical commit references):

### Active Proofs-of-Concept & Research
| Focus Area | Specification Document | Proof-of-Concept | Key Decision / Outcome |
| :--- | :--- | :--- | :--- |
| **Interactive Rebase Engine** | [`docs/research/interactive-rebase-modifications.md`](docs/research/interactive-rebase-modifications.md) | [`research/interactive-rebase-poc/`](research/interactive-rebase-poc/) | Non-interactive fixup rebases vs `GIT_SEQUENCE_EDITOR` marked edits with TUI suspension loops. |
| **Conflict Resolution Flow** | [`docs/research/conflict-detection-and-resolution.md`](docs/research/conflict-detection-and-resolution.md) | Integrated in rebase PoC | Fail-fast auto-abort with autostash recovery for inline edits; subshell delegation for edit-here. |
| **Neovim RPC Integration** | [`docs/research/neovim-rpc-terminal-integration.md`](docs/research/neovim-rpc-terminal-integration.md) | [`research/neovim-rpc-poc/`](research/neovim-rpc-poc/) | Bidirectional socket loop via `nvim-rs` + Tokio, using `BufWipeout` autocommands for cleanup. |
| **Neovim Lua Plugin Launcher** | [`docs/research/neovim-plugin-launcher-packaging.md`](docs/research/neovim-plugin-launcher-packaging.md) | [`research/neovim-plugin-poc/`](research/neovim-plugin-poc/) | `git-tardis.nvim` floating window wrapper passing `$NVIM` environment variable. |

### Completed Features (Integrated into Main Crate)
The research PoCs for keybindings, layout, syntax highlighting, and tree-sitter scope locators have been fully implemented in `src/` and removed from the active working tree. See [Section 9.1 of the Technical Specification](docs/technical-specification.md#91-historical-research--proof-of-concept-code-references) for historical Git commit hashes (`5fce52e`, `50c9b77`, `4ac3ab7`, `c5319d0`, `43f017b`).

---

## Wayfinder Map

See [Issue #1 (Wayfinder Map)](https://github.com/buxtonpaul/git-tardis/issues/1) for the overall roadmap and technical specifications tracking.

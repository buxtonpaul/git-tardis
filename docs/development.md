# Developer Guide & Technical Architecture

This document provides developer guidelines, build instructions, architectural overviews, and technical specifications for contributing to **Git-tardis**.

---

## 1. Prerequisites & Development Toolchain

To build and test Git-tardis locally, ensure you have the following installed:

- **Rust Toolchain**: `rustc` and `cargo` (Rust 1.80+ recommended).
- **Git**: `git >= 2.30`.
- **C Compiler / Build Tools**: Required for tree-sitter C grammar compilation (`clang` or `gcc`).

---

## 2. Building & Testing

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Running Tests

Run the full unit and integration test suite:

```bash
cargo test
```

### Code Formatting & Linting

Run Clippy to verify code quality against workspace warnings:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Format code using `rustfmt`:

```bash
cargo fmt --all -- --check
```

---

## 3. Project Structure

```text
git-tardis/
├── src/
│   ├── app/           # App state management, viewport, file tree, layout
│   ├── cli/           # CLI argument parsing (clap)
│   ├── config/        # TOML configuration and keybindings parser
│   ├── git/           # Git repository interactions, blame, diff engine
│   ├── rebase/        # Interactive rebase engine & sequence editor handler
│   ├── timeline/      # Candidate commit locator for line/function/file modes
│   ├── treesitter/    # Tree-sitter scope locator and syntax highlighter
│   └── ui/            # Ratatui TUI components, layouts, keymap dispatcher, markdown
├── tests/             # End-to-end and module integration test suites
├── docs/              # User guides, technical specifications, and research docs
├── lua/               # Neovim plugin source code (`git-tardis.nvim`)
└── research/          # Proof-of-concept prototypes for architecture decisions
```

---

## 4. Technical Specifications & Research

The design and architecture of Git-tardis are backed by validated proofs of concept and technical specifications (see [technical-specification.md](technical-specification.md) for full architectural specs and historical commit references).

### Active Proofs-of-Concept & Research

| Focus Area | Specification Document | Proof-of-Concept | Key Decision / Outcome |
| :--- | :--- | :--- | :--- |
| **Interactive Rebase Engine** | [`research/interactive-rebase-modifications.md`](research/interactive-rebase-modifications.md) | [`research/interactive-rebase-poc/`](../research/interactive-rebase-poc/) | Non-interactive fixup rebases vs `GIT_SEQUENCE_EDITOR` marked edits with TUI suspension loops. |
| **Conflict Resolution Flow** | [`research/conflict-detection-and-resolution.md`](research/conflict-detection-and-resolution.md) | Integrated in rebase PoC | Subshell delegation and conflict abort/resume flow for interactive edit-here rebases. |
| **Neovim RPC Integration** | [`research/neovim-rpc-terminal-integration.md`](research/neovim-rpc-terminal-integration.md) | [`research/neovim-rpc-poc/`](../research/neovim-rpc-poc/) | Bidirectional socket loop via `nvim-rs` + Tokio, using `BufWipeout` autocommands for cleanup. |
| **Neovim Lua Plugin Launcher** | [`research/neovim-plugin-launcher-packaging.md`](research/neovim-plugin-launcher-packaging.md) | [`research/neovim-plugin-poc/`](../research/neovim-plugin-poc/) | `git-tardis.nvim` floating window wrapper passing `$NVIM` environment variable. |

### Completed Features (Integrated into Main Crate)

The research PoCs for keybindings, layout, syntax highlighting, and tree-sitter scope locators have been fully implemented in `src/`. See [Section 9.1 of the Technical Specification](technical-specification.md#91-historical-research--proof-of-concept-code-references) for historical Git commit references (`5fce52e`, `50c9b77`, `4ac3ab7`, `c5319d0`, `43f017b`).

---

## 5. Development Guidelines & Conventions

- **Branching**: Work on features or bug fixes in dedicated branches (e.g. `feature/...` or `issue-XX-...`).
- **Code Style**: Follow standard Rust idioms. Run `cargo clippy --workspace --all-targets -- -D warnings` before committing.
- **Testing**: Add unit tests in `src/` or integration tests in `tests/` for any new feature or bug fix.

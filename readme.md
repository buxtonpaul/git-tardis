# Git-tardis ⌛🛸

**Git-tardis** is a high-performance Terminal UI (TUI) for time travelling through your Git repositories. It lets you navigate back and forth through commit history to trace how code evolved line-by-line and function-by-function, make historical fixup edits on the fly, and seamlessly integrate into Neovim.

![Rust](https://img.shields.io/badge/language-Rust-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

---

## ✨ Features

- ⏳ **Line & Function Time Travel**: Jump directly between historical commits affecting the exact line under your cursor or its enclosing function using Tree-sitter AST parsing.
- ⚡ **Interactive Rebase ("Edit Here")**: Pause an interactive rebase at any historical commit (`e`), dropping into a shell/editor subshell to make adjustments before resuming.
- 🔍 **Diff & Full File Views**: Seamlessly switch between viewing full historical file contents and Git diffs (`v`).
- 📁 **File Tree & Modified File Explorers**: Browse the workspace tree at any target commit or view only modified files.
- 🎨 **Neovim RPC Integration**: Launch as a floating window inside Neovim (`git-tardis.nvim`) with synchronized cursor positions, buffer paths, and inherited color schemes.
- 🚀 **Lightning Fast**: Built with Rust, Ratatui, Crossterm, and Tree-sitter for instant response times even in large repositories.

---

## 📦 Installation

### Option 1: Homebrew (macOS & Linux)

```bash
brew tap buxtonpaul/tap
brew install git-tardis
```

### Option 2: Cargo / Build from Source

Ensure you have the Rust toolchain installed:

```bash
git clone https://github.com/buxtonpaul/git-tardis.git
cd git-tardis
cargo install --path .
```

Verify the installation:

```bash
git-tardis --version
```

For full installation options and requirements, see the [Installation Guide](docs/installation.md).

---

## 🚀 Quick Start & Usage

Launch `git-tardis` inside any Git repository:

```bash
# Open Git-tardis in the current working directory
git-tardis

# Focus on a specific file and line on startup
git-tardis -f src/main.rs -l 42

# Launch directly in Function Navigation mode
git-tardis -f src/app/mod.rs -l 100 --jump-mode function
```

Press `?` inside Git-tardis at any time to display the interactive keybindings help screen.

---

## ⌨️ Common Keybindings

| Key / Shortcut | Action |
| :--- | :--- |
| **`[`** / **`Ctrl+p`** | Jump to **previous commit** affecting current line/function/file |
| **`]`** / **`Ctrl+n`** | Jump to **next commit** affecting current line/function/file |
| **`m`** | Cycle navigation scope mode (`Auto` ➔ `Line` ➔ `Function` ➔ `File` ➔ `Commit`) |
| **`v`** | Toggle between **Diff View** and **Full File View** |
| **`1`** / **`2`** / **`3`** | Focus **Commit Timeline** (`1`), **File Tree** (`2`), or **Modified Files** (`3`) |
| **`e`** | **Edit Here**: Trigger interactive rebase paused at selected commit |
| **`?`** | Toggle interactive **Keybindings Help** overlay |
| **`q`** / **`Esc`** | Quit / Dismiss modal |

---

## 🔌 Editor Extensions

### Neovim Plugin (`git-tardis.nvim`)

Git-tardis integrates natively with Neovim! Run `:GitTardis`, `:GitTardisToggle`, or launch mode commands (`:InspectPrevFunctionCommitAtLine`, `:InspectPrevLineCommitAtLine`, `:InspectPrevFileCommitAtLine`, `:InspectPrevCommit`) to open Git-tardis in a floating terminal window with:
- Automatic sync of current file path, cursor line, and jump mode
- Dynamic inheritance of active Neovim color schemes
- Instant keymap integration for line and function history inspection

Check out the [Neovim Plugin Guide](docs/neovim-plugin.md) for full setup and keymaps with `lazy.nvim`, `packer`, or `vim-plug`.

### Visual Studio Code Extension (`git-tardis-vscode`)

Integrate Git-tardis into VS Code! Launch from the Command Palette or default shortcuts (`Cmd+Alt+G F` / `Ctrl+Alt+G F` for function time travel, `Cmd+Alt+G L` / `Ctrl+Alt+G L` for line time travel):
- Direct opening at active file path and 1-based line position
- Integrated terminal tab/panel execution with auto-closing on process exit
- Configurable binary paths, terminal locations (`editor` vs `panel`), and keybindings

Check out the [VS Code Extension Guide](docs/vscode-extension.md) for installation and usage instructions.

---

## ⚙️ Configuration

Git-tardis can be customized via a TOML configuration file located at `~/.config/git-tardis/config.toml` (or `$XDG_CONFIG_HOME/git-tardis/config.toml`). You can customize keybindings, color themes, and default navigation modes.

---

## 🛠️ Documentation & Contributing

- 📖 **[Installation Guide](docs/installation.md)** – Platform-specific setup instructions.
- 🔌 **[Neovim Plugin Guide](docs/neovim-plugin.md)** – Installing and configuring `git-tardis.nvim`.
- ⚡ **[VS Code Extension Guide](docs/vscode-extension.md)** – Setup and configuration for VS Code.
- 💻 **[Developer & Architecture Guide](docs/development.md)** – Build instructions, project layout, PoCs, and architectural overviews.
- 📐 **[Technical Specification](docs/technical-specification.md)** – Deep-dive architectural specification and design documents.

---

## 📄 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

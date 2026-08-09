# Git-tardis VS Code Extension (`git-tardis-vscode`)

VS Code extension for **[Git-tardis](https://github.com/buxtonpaul/git-tardis)** — time travel debugging in Git history with line and function granularity.

## Features

- **Context-Aware Time Travel**: Launch `git-tardis` focused on the active file and cursor line directly from VS Code.
- **Function & Line Jump Modes**: Execute dedicated commands to jump straight to historical commits modifying the enclosing function or active line.
- **Terminal Lifecycle Management**: Opens `git-tardis` in an editor tab or bottom panel with automatic pane closing on exit.

## Commands & Default Shortcuts

| Command | Shortcut | Description |
| :--- | :--- | :--- |
| `Git-tardis: Open` | `Ctrl+Alt+G O` / `Cmd+Alt+G O` | Open Git-tardis at active file and line |
| `Git-tardis: Toggle Terminal` | `Ctrl+Alt+G T` / `Cmd+Alt+G T` | Toggle active terminal pane |
| `Git-tardis: Jump Prev Function Commit` | `Ctrl+Alt+G F` / `Cmd+Alt+G F` | Inspect previous commit affecting function |
| `Git-tardis: Jump Prev Line Commit` | `Ctrl+Alt+G L` / `Cmd+Alt+G L` | Inspect previous commit affecting line |
| `Git-tardis: Jump Prev File Commit` | `Ctrl+Alt+G A` / `Cmd+Alt+G A` | Inspect previous commit affecting file |
| `Git-tardis: Jump Prev Commit` | `Ctrl+Alt+G C` / `Cmd+Alt+G C` | Inspect previous commit in history |

## Settings

- `git-tardis.binaryPath`: Binary path or command (default `"git-tardis"`).
- `git-tardis.terminalLocation`: Terminal placement (`"editor"` or `"panel"`, default `"editor"`).
- `git-tardis.autoClosePane`: Auto close terminal tab when process exits (default `true`).

## Documentation

For full usage, configuration, and architecture details, see [VS Code Extension Guide](../../docs/vscode-extension.md).

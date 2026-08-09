# VS Code Extension Integration: `git-tardis-vscode`

The `git-tardis-vscode` extension integrates **Git-tardis** directly into Visual Studio Code. It resolves the active editor's file path and cursor line, launches `git-tardis` inside an integrated VS Code terminal pane or tab, and manages the terminal process lifecycle automatically.

---

## 1. Launch Modes

Git-tardis supports six commands corresponding to different navigation modes and actions:

| VS Code Command ID | Command Palette Title | Default Shortcut | Description |
| :--- | :--- | :--- | :--- |
| `git-tardis.open` | `Git-tardis: Open` | `Ctrl+Alt+G O` / `Cmd+Alt+G O` | Opens Git-tardis targeting the active file and line number |
| `git-tardis.toggle` | `Git-tardis: Toggle Terminal` | `Ctrl+Alt+G T` / `Cmd+Alt+G T` | Toggles the active Git-tardis terminal window open/closed |
| `git-tardis.inspectPrevFunction` | `Git-tardis: Jump Prev Function Commit` | `Ctrl+Alt+G F` / `Cmd+Alt+G F` | Opens Git-tardis in `--jump-mode function` targeting the active function |
| `git-tardis.inspectPrevLine` | `Git-tardis: Jump Prev Line Commit` | `Ctrl+Alt+G L` / `Cmd+Alt+G L` | Opens Git-tardis in `--jump-mode line` targeting the current line |
| `git-tardis.inspectPrevFile` | `Git-tardis: Jump Prev File Commit` | `Ctrl+Alt+G A` / `Cmd+Alt+G A` | Opens Git-tardis in `--jump-mode file` targeting the active file |
| `git-tardis.inspectPrevCommit` | `Git-tardis: Jump Prev Commit` | `Ctrl+Alt+G C` / `Cmd+Alt+G C` | Opens Git-tardis in `--jump-mode commit` |

---

## 2. Installation & Setup

### Prerequisites
- Install `git-tardis` CLI tool on your system path (e.g. via `brew install git-tardis` or `cargo install --path .`).
- Ensure Visual Studio Code (v1.80.0 or higher) is installed.

### Installing the Extension

From source (Development / Manual Install):
1. Navigate to `editors/vscode`:
   ```bash
   cd editors/vscode
   npm install
   npm run compile
   ```
2. Press `F5` in VS Code to run in Extension Development Host, or package with `npx vsce package` to create a `.vsix` bundle and install via `code --install-extension git-tardis-vscode-0.1.0.vsix`.

---

## 3. Configuration Settings

The extension contributes the following settings under the `git-tardis.*` namespace:

| Setting | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `git-tardis.binaryPath` | `string` | `"git-tardis"` | Path or executable name for the `git-tardis` binary |
| `git-tardis.terminalLocation` | `string` (`"editor"` \| `"panel"`) | `"editor"` | Controls whether the terminal pane opens as an editor tab (`"editor"`) or in the bottom drawer (`"panel"`) |
| `git-tardis.autoClosePane` | `boolean` | `true` | Automatically disposes and closes the terminal tab/panel when `git-tardis` exits |

### Example `settings.json`:

```json
{
  "git-tardis.binaryPath": "/usr/local/bin/git-tardis",
  "git-tardis.terminalLocation": "editor",
  "git-tardis.autoClosePane": true
}
```

---

## 4. Keybindings & Usage

### Executing Commands
1. Open any file in a Git repository.
2. Place your cursor on a line or function of interest.
3. Trigger a command via Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`) by typing `Git-tardis` or use key shortcuts:
   - `Cmd+Alt+G F` / `Ctrl+Alt+G F` — Instantly step back to the previous commit modifying the enclosing function.
   - `Cmd+Alt+G L` / `Ctrl+Alt+G L` — Instantly step back to the previous commit modifying the current line.
   - `Cmd+Alt+G T` / `Ctrl+Alt+G T` — Toggle the Git-tardis terminal tab open or closed.

### Exiting Git-tardis
- Inside Git-tardis, press `q` to quit the application.
- When `git-tardis.autoClosePane` is set to `true`, the VS Code terminal pane automatically closes and returns focus to your code editor.

---

## 5. Active Context Resolution

When executing any `git-tardis` command:
1. **File & Line Identification**: The extension reads `activeTextEditor.document.uri.fsPath` and line position (`activeTextEditor.selection.active.line + 1`).
2. **Workspace Root Resolution**: Resolves workspace folder via `vscode.workspace.getWorkspaceFolder(...)` to ensure terminal process runs in the correct Git root.
3. **Fallback Handling**: If no file is currently active, `git-tardis` opens at the workspace root directory in standard navigation mode.

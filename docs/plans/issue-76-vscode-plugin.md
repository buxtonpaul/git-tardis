# Implementation Plan - Issue #76: VSCode Plugin for git-tardis

## Architectural Approach

The VS Code extension (`git-tardis-vscode`) provides integration between VS Code and the `git-tardis` CLI tool, matching the editor-context capabilities and terminal lifecycle management established in the Neovim plugin (`git-tardis.nvim`).

### Core Design Decisions

1. **Location & Organization**:
   - Extension source placed under `editors/vscode/` (e.g. `editors/vscode/src/extension.ts`), keeping editor integrations structured alongside Neovim configuration files.
   - Separate modules for terminal lifecycle management (`terminalManager.ts`), command definitions (`commands.ts`), and extension setup (`extension.ts`).

2. **Active Context Resolution**:
   - Query `vscode.window.activeTextEditor` for document file path (`document.uri.fsPath`) and line number (`selection.active.line + 1` to convert 0-based VS Code line index to 1-based CLI index).
   - Infer Git repository root using `vscode.workspace.getWorkspaceFolder(document.uri)`.

3. **Terminal Lifecycle & Pane Management**:
   - Use `vscode.window.createTerminal` with `shellPath` pointing to the `git-tardis` executable and `shellArgs` carrying CLI arguments (`--file`, `--line`, `--jump-mode`).
   - Terminal location defaulted to `vscode.TerminalLocation.Editor` (opens as an editor tab/pane alongside active code), configurable via setting `git-tardis.terminalLocation` (`"editor"` or `"panel"`).
   - Track active terminal process instances. On execution exit (`onDidEndTerminalShellExecution` / `onDidCloseTerminal`), automatically dispose the terminal pane if `git-tardis.autoClosePane` is set to `true`.

4. **Registered Commands**:
   - `git-tardis.open`: Open Git-tardis at active file and line.
   - `git-tardis.toggle`: Toggle active Git-tardis terminal window.
   - `git-tardis.inspectPrevFunction`: Launch with `--jump-mode function`.
   - `git-tardis.inspectPrevLine`: Launch with `--jump-mode line`.
   - `git-tardis.inspectPrevFile`: Launch with `--jump-mode file`.
   - `git-tardis.inspectPrevCommit`: Launch with `--jump-mode commit`.

5. **Configuration Options (`git-tardis.*`)**:
   - `git-tardis.binaryPath`: Executable path (default `"git-tardis"`).
   - `git-tardis.terminalLocation`: Location for terminal pane (`"editor"` vs `"panel"`, default `"editor"`).
   - `git-tardis.autoClosePane`: Automatically close terminal tab/panel when `git-tardis` process exits (default `true`).

---

## Target Files

### To Create:
- `editors/vscode/package.json` — VS Code extension manifest (commands, configuration schemas, keybindings, build scripts).
- `editors/vscode/tsconfig.json` — TypeScript compiler configuration for VS Code extension development.
- `editors/vscode/esbuild.js` — Esbuild script for bundling extension source into `dist/extension.js`.
- `editors/vscode/src/extension.ts` — Extension activation/deactivation hooks and subscription registration.
- `editors/vscode/src/commands.ts` — Command handlers translating active VS Code state into `git-tardis` CLI arguments.
- `editors/vscode/src/terminalManager.ts` — Manager class encapsulating VS Code terminal creation, toggle logic, and auto-close event listeners.
- `editors/vscode/src/test/suite/extension.test.ts` — Integration/unit tests for command argument construction and context resolution.
- `editors/vscode/README.md` — Extension installation and usage instructions.
- `docs/vscode-extension.md` — Full extension documentation detailing launch modes, keybindings, and settings.

### To Modify:
- `README.md` — Add VS Code extension section alongside Neovim documentation in the root README.

---

## Implementation Steps

### Phase 1: Project & Toolchain Setup (`editors/vscode/`)
1. Create directory structure `editors/vscode/src/` and `editors/vscode/src/test/`.
2. Create `editors/vscode/package.json` with dependencies (`@types/vscode`, `typescript`, `esbuild`) and configuration schemas:
   - Contributed configuration settings:
     - `git-tardis.binaryPath` (string, default `"git-tardis"`)
     - `git-tardis.terminalLocation` (enum `["editor", "panel"]`, default `"editor"`)
     - `git-tardis.autoClosePane` (boolean, default `true`)
   - Contributed commands & default keybindings (`Ctrl+Alt+G T` / `Cmd+Alt+G T` prefix mappings).
3. Add `editors/vscode/tsconfig.json` targeting `ES2022` with CommonJS module resolution for VS Code extension host runtime.
4. Add `editors/vscode/esbuild.js` script for fast development and production bundling.

### Phase 2: Terminal Lifecycle & Command Core
5. Implement `editors/vscode/src/terminalManager.ts`:
   - `TerminalManager` class tracking active `vscode.Terminal` instance.
   - `launch(args: string[])`: Spawns terminal instance using `vscode.window.createTerminal`, listens to `onDidEndTerminalShellExecution` / `onDidCloseTerminal` to handle pane disposal when `autoClosePane` is `true`.
   - `toggle()`: Toggles terminal focus or closes/opens instance if currently running.
   - `dispose()`: Cleanly disposes terminal on extension deactivation.
6. Implement `editors/vscode/src/commands.ts`:
   - `buildCmdArgs(jumpMode?: string)`: Resolves active text editor URI and selection. Computes `--file` path, `--line` 1-based line number, and `--jump-mode` flags.
   - Expose registration functions for all 6 commands (`open`, `toggle`, `inspectPrevFunction`, `inspectPrevLine`, `inspectPrevFile`, `inspectPrevCommit`).
7. Implement `editors/vscode/src/extension.ts`:
   - `activate(context: vscode.ExtensionContext)`: Instantiates `TerminalManager`, registers commands, adds disposables to `context.subscriptions`.
   - `deactivate()`: Cleanly disposes terminal instances.

### Phase 3: Tests, Documentation & Integration
8. Add tests in `editors/vscode/src/test/suite/extension.test.ts`:
   - Test CLI argument builder for active file, cursor position, and jump modes.
   - Test fallback behavior when no file is active in editor.
9. Create `docs/vscode-extension.md` documenting launch modes, settings, keybindings, and terminal drawer/editor configurations.
10. Update root `README.md` with VS Code integration guide and link to `docs/vscode-extension.md`.

---

## Verification Strategy

### Automated Verification
1. **Compilation & Type Check**:
   ```bash
   cd editors/vscode && npm run compile
   ```
   Ensures zero TypeScript errors or missing extension API imports.

2. **Unit / Extension Tests**:
   ```bash
   cd editors/vscode && npm test
   ```
   Runs extension test suite in VS Code Extension Testing Environment (`@vscode/test-electron`).

### Manual Inspection & End-to-End Checks
1. **Launch from VS Code Extension Host** (`F5` in VS Code / Launch Extension):
   - Open a Rust or TypeScript file inside a Git repository in the Extension Development Host window.
   - Place cursor on a specific line inside a function.
2. **Test Default View**:
   - Execute command `Git-tardis: Open` (`git-tardis.open`).
   - Confirm terminal tab opens alongside the editor focused on the active file and line.
3. **Test Jump Modes**:
   - Execute `Git-tardis: Jump Prev Function Commit` (`git-tardis.inspectPrevFunction`).
   - Verify `git-tardis` launches with `--jump-mode function --file <file> --line <line>`.
4. **Test Auto-Close Lifecycle**:
   - Exit `git-tardis` inside terminal by pressing `q`.
   - Verify the VS Code terminal editor tab/panel closes automatically.

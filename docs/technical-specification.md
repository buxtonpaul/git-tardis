# Technical Specification: Git-tardis

**Version:** 1.0.0  
**Date:** July 21, 2026  
**Status:** Approved Technical Specification  
**Repository:** `git-tardis`

---

## 1. System Overview & Architecture Scope

### 1.1 Product Vision
**Git-tardis** is a high-performance terminal utility and Neovim extension written in Rust that enables instant "time travel" through a repository's Git history. It allows developers to seamlessly navigate code back and forth across commits at the file, function, or line level, and perform historical edits directly without breaking flow or manually manipulating Git reflogs.

### 1.2 Core Target Capabilities
1. **Multi-Scope Timeline Navigation**: Jump between historical states of the active repository at three distinct granularities:
   - **File Scope**: Step through commits modifying the open file.
   - **Function Scope**: Step through commits modifying the specific enclosing function/method under the editor cursor using Tree-sitter AST queries.
   - **Line Scope**: Step through commits modifying the line under the cursor (blame history).
2. **Inline Historical Rewrite ("Inline rewrite")**: Edit code in the active viewer, stage the changes as a fixup commit, and automatically execute a non-interactive, atomic rebase to squash the edit back into a selected historical commit.
3. **Interactive Rebase Pause ("Edit here")**: Temporarily pause Git history at any chosen commit, suspend TUI rendering, drop into an interactive subshell or Neovim buffer to perform multi-file modifications, and resume TUI rendering upon rebase completion.
4. **Deep Neovim Integration (`git-tardis.nvim`)**: Package as a Neovim Lua plugin floating window, communicating bidirectionally with host Neovim over MessagePack-RPC (`$NVIM`).
5. **Theme-Aware Syntax Highlighting**: Render high-speed syntax-highlighted code using Tree-sitter, automatically synchronized with host Neovim color schemes or terminal ANSI palettes.

### 1.3 Platform Support & Dependencies
- **Operating Systems**: macOS and Linux (x86_64, aarch64).
- **Core Technologies**: Rust 2021 Edition, Tokio async runtime, Ratatui (TUI layout), Crossterm (terminal I/O), Tree-sitter (AST parsing), `nvim-rs` (Neovim RPC), Git CLI.

---

## 2. High-Level System Architecture

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        Neovim Host Instance                            │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ git-tardis.nvim (Lua Plugin)                                     │  │
│  │  - Spawns terminal pty via vim.fn.termopen()                     │  │
│  │  - Sets $NVIM socket environment variable                        │  │
│  └─────────────────────────────────┬────────────────────────────────┘  │
└────────────────────────────────────┼───────────────────────────────────┘
                                     │ Unix Domain Socket / $NVIM
┌────────────────────────────────────▼───────────────────────────────────┐
│                        Git-tardis Rust Binary                          │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ App Event Loop & Tokio Runtime                                   │  │
│  │                                                                  │  │
│  │   ┌────────────────────┐            ┌─────────────────────────┐  │  │
│  │   │ Neovim RPC Client  │            │ Git CLI & Rebase Engine │  │  │
│  │   │ - nvim-rs / Tokio  │            │ - Inline Autosquash     │  │  │
│  │   │ - Theme Sync       │            │ - Sequence Editor       │  │  │
│  │   └─────────┬──────────┘            └────────────┬────────────┘  │  │
│  │             │                                    │               │  │
│  │   ┌─────────▼──────────┐            ┌────────────▼────────────┐  │  │
│  │   │ Tree-sitter Engine │            │ Monolithic AppState     │  │  │
│  │   │ - Function AST     │            │ - Navigation Mode       │  │  │
│  │   │ - Viewport Highlight│           │ - Panel Focus & Selections│ │  │
│  │   └─────────┬──────────┘            └────────────┬────────────┘  │  │
│  │             │                                    │               │  │
│  │             └─────────────────┬──────────────────┘               │  │
│  │                               │                                  │  │
│  │                     ┌─────────▼─────────┐                        │  │
│  │                     │ Ratatui TUI View  │                        │  │
│  │                     │ - Split Layout    │                        │  │
│  │                     │ - Crossterm I/O   │                        │  │
│  │                     └───────────────────┘                        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Codebase Module Structure
- **`src/main.rs`**: Application entrypoint, CLI argument parser (including sequence editor invocation flag), and Crossterm terminal setup/cleanup.
- **`src/app/`**: Application state machine (`AppState`), panel focus management, and event handling loop.
- **`src/ui/`**: Ratatui rendering layouts, split pane draw functions, focus highlighting, and modal dialogs.
- **`src/git/`**: Git repository interaction wrapper, commit log parser, diff calculator, and blame queries.
- **`src/rebase/`**: Sequence editor modifications, non-interactive rebase execution, TUI suspension routines, and conflict recovery.
- **`src/treesitter/`**: Parser manager, multi-language grammar registry, dynamic function boundary locator, and viewport query highlighter.
- **`src/nvim/`**: RPC client using `nvim-rs`, socket client initialization, theme palette query (`nvim_get_hl`), and autocommand event listener (`BufWipeout`).

---

## 3. Neovim RPC Integration & Launcher Architecture

### 3.1 Socket Path Resolution
When launched within Neovim via `:terminal` or `vim.fn.termopen()`, host Neovim sets the `$NVIM` environment variable containing the path to its listening socket:

```rust
let socket_path = std::env::var("NVIM").map_err(|_| {
    "Git-tardis must be run inside a Neovim terminal so $NVIM is set."
})?;
```

### 3.2 Asynchronous RPC Loop
Git-tardis establishes an async MessagePack-RPC stream over the Unix domain socket using `nvim-rs` backed by `tokio::net::UnixStream`.

```rust
use async_trait::async_trait;
use nvim_rs::{compat::tokio::Compat, Handler, Neovim};
use tokio::io::WriteHalf;
use tokio::net::UnixStream;

type Writer = Compat<WriteHalf<UnixStream>>;

#[derive(Clone)]
pub struct NeovimHandler {
    tx: tokio::sync::mpsc::UnboundedSender<String>,
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Writer;

    async fn handle_notify(&self, name: String, _args: Vec<rmpv::Value>, _neovim: Neovim<Self::Writer>) {
        let _ = self.tx.send(name);
    }

    async fn handle_request(&self, _name: String, _args: Vec<rmpv::Value>, _neovim: Neovim<Self::Writer>) -> Result<rmpv::Value, rmpv::Value> {
        Ok(rmpv::Value::Nil)
    }
}
```

### 3.3 Scratch Buffer & Floating Window Autocommands
To open floating dialogs or overlay buffers back in Neovim:
1. Create a scratch buffer (`nvim_create_buf(false, true)`).
2. Set `'bufhidden'` to `'wipe'`.
3. Open floating window (`nvim_open_win`).
4. Query RPC channel ID using `nvim_get_api_info()`.
5. Register a transient `BufWipeout` autocommand on the scratch buffer that fires `rpcnotify(channel_id, 'float_closed')`.
6. Git-tardis awaits `'float_closed'` notification asynchronously before resuming state.

### 3.4 `git-tardis.nvim` Packaging Specification
The Neovim plugin launcher is structured as follows:
- **`lua/git-tardis/init.lua`**: Main configuration and window launcher. Calculates centered floating window dimensions (`0.85` height/width ratio) and calls `vim.fn.termopen("git-tardis")`.
- **`plugin/git-tardis.lua`**: Registers user commands `:GitTardis [path]` and `:GitTardisToggle`.
- Compatible with `lazy.nvim`, `pckr.nvim`, and `vim-plug`.

---

## 4. AST-Aware Code Inspection (Tree-sitter)

### 4.1 Supported Languages & Grammar Mapping
Tree-sitter parsers are compiled for target languages with ABI compliance (`tree-sitter = "0.26"`):

| Language | Extension | Node `kind()` Identifier Strings |
| :--- | :--- | :--- |
| **Rust** | `.rs` | `"function_item"`, `"closure_expression"` |
| **C** | `.c`, `.h` | `"function_definition"` |
| **C++** | `.cpp`, `.hpp`, `.cc`, `.cxx`, `.hh` | `"function_definition"`, `"template_declaration"` |
| **Go** | `.go` | `"function_declaration"`, `"method_declaration"`, `"func_literal"` |
| **Python** | `.py` | `"function_definition"`, `"lambda"` |
| **TypeScript / JS** | `.ts`, `.js`, `.tsx`, `.jsx` | `"function_declaration"`, `"function_expression"`, `"arrow_function"`, `"method_definition"`, `"generator_function_declaration"`, `"generator_function"` |

### 4.2 Dynamic Enclosing Function Resolution Algorithm
To map a 1-based editor line number to its exact enclosing function byte and line range:

1. **Whitespace Normalization**: Compute line row index `line_number - 1`. Find the column index of the first non-whitespace character on the line. Construct search `Point { row, column }`.
2. **Bottom-Up Traversal**: Retrieve `root_node.descendant_for_point_range(point, point)`. Walk up parent chain (`node.parent()`). Return `(start_line, end_line)` on first match against target language function kinds.
3. **Top-Down Declarator Inspection Fallback**: If bottom-up traversal yields no function node (e.g. cursor rests on `const` in `const foo = () => {}`), inspect the line's statement node (`lexical_declaration`) and search its child AST subtree for an assigned function/arrow node.
4. **Syntax Fault Tolerance**:
   - **Localized Syntax Errors**: Tree-sitter isolates errors into `(ERROR)` nodes. Walking up past `(ERROR)` to the enclosing block cleanly resolves the valid function boundary.
   - **Severe/Unclosed Scope Errors**: If scope braces are incomplete, Tree-sitter produces a top-level `(ERROR)` without a function node. The algorithm safely returns `Ok(None)`, falling back to line/file navigation.

### 4.3 Grammar Extensibility & Dynamic Loading Architecture
Git-tardis implements a **Hybrid Grammar Architecture** to combine static performance with dynamic user extensibility:
- **Static Core**: Built-in parsers (Rust, C, C++, Go, Python, TypeScript/JS, Lua) for zero-setup execution.
- **Dynamic Plugin Loader**: Users can load custom external shared objects (`.so`/`.dylib`) using `libloading` via the standard Tree-sitter C entrypoint (`tree_sitter_<lang>`). Shared library handles are wrapped in `Arc<libloading::Library>` to guarantee symbol memory validity.
- **TOML & Lua Configuration**: Custom file extension mappings, AST node kinds, and `.scm` queries can be supplied via `~/.config/git-tardis/config.toml` or Neovim `setup()` options.
- **Graceful Fallback Chain**: If a language parser is uninstalled, fails ABI checks, or raises query errors, Git-tardis automatically falls back to File Mode or Line Mode navigation without raising TUI panics. See [`docs/research/tree-sitter-grammar-extensibility.md`](research/tree-sitter-grammar-extensibility.md).

---

## 5. UI Layout Engine & Navigation State Machine

### 5.1 Structural Data Model
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Sidebar,
    CodeViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarView {
    FileExplorer,
    ModifiedFiles,
    CommitTimeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationMode {
    File,
    Function,
    Line,
}

pub struct AppState {
    pub active_panel: ActivePanel,
    pub sidebar_view: SidebarView,
    pub nav_mode: NavigationMode,

    // Independent selections preserved across tab swaps
    pub files: Vec<String>,
    pub file_selected: usize,

    pub modified_files: Vec<String>,
    pub modified_selected: usize,

    pub commits: Vec<(String, String)>,
    pub commit_selected: usize,

    // Code viewer state
    pub code_lines: Vec<String>,
    pub cursor_line: usize, // 1-based index
    
    pub running: bool,
}
```

### 5.2 Ratatui Split Geometry
```text
┌────────────────────────────────────────────────────────────────────────┐
│ Header & View Selector (Tabs: 1: Explorer | 2: Modified | 3: Timeline)  │ Length(3)
├──────────────────────────────────┬─────────────────────────────────────┤
│ Left Sidebar (32% Width)         │ Right Code Viewer (68% Width)       │
│                                  │                                     │
│ Border: Cyan (Active Focus)      │ Border: DarkGray (Inactive Focus)   │ Min(0)
│                                  │                                     │
├──────────────────────────────────┴─────────────────────────────────────┤
│ Footer (Keybindings & Active Navigation Mode Indicator)                 │ Length(3)
└────────────────────────────────────────────────────────────────────────┘
```

### 5.3 Hierarchical Key Event Routing Matrix

| Scope | Key Binding | Action |
| :--- | :--- | :--- |
| **Global** | `q`, `Esc` | Quit application |
| **Global** | `Tab`, `h`, `l`, `Left`, `Right` | Swap panel focus (`Sidebar` $\leftrightarrow$ `CodeViewer`) |
| **Global** | `1`, `2`, `3` | Switch `SidebarView` tab (`Explorer`, `Modified`, `Timeline`) |
| **Global** | `m` | Cycle `NavigationMode` (`File` $\rightarrow$ `Function` $\rightarrow$ `Line` $\rightarrow$ `File`) |
| **Sidebar Scope** | `j`, `k`, `Up`, `Down` | Navigate selected list item in active sidebar view |
| **Code Viewer Scope** | `j`, `k`, `Up`, `Down` | Move code cursor line up/down |
| **Code Viewer Scope** | `n`, `]`, `p`, `[` | Execute timeline jump (`next`/`previous` commit) based on `nav_mode` |
| **Code Viewer Scope** | `r` | Trigger "Inline rewrite" on current commit |
| **Code Viewer Scope** | `e` | Trigger "Edit here" on current commit |

---

## 6. Syntax Highlighting & Theme Engine

### 6.1 Syntax Engine Benchmark Selection
Tree-sitter is selected as the primary syntax highlighting engine over `syntect`:
- **Execution Speed**: Full file parse and highlight on 5,000 lines requires **20.7 ms** in Tree-sitter vs **94.6 ms** in `syntect` (**~4.5x–10x faster**).
- **Zero Architectural Overhead**: Tree-sitter parsers are already linked for function boundary queries.

### 6.2 Rendering Performance Optimizations
1. **Viewport Cropping (Virtual Scrolling)**: Construct Ratatui `Line` / `Span` vectors **only** for visible rows in the viewport (`viewport_top..viewport_top + height`). Reduces frame allocation cost from **93.8 ms** to **8.8 ms** (**10x reduction**).
2. **Adjacent Span Merging**: Merge adjacent spans sharing identical `Style` attributes. Reduces span object count across the viewport by **60.5%** (from 377 down to 149 spans), cutting render tree traversal and Crossterm diffing overhead.

### 6.3 Theme Synchronization
- **Neovim Host Sync**: Query active highlight groups via `nvim_get_hl(0, { name = "Group" })` over RPC. Map integer RGB values (`fg`, `bg`) and modifier flags (`bold`, `italic`) directly into `ratatui::style::Style`.
- **ANSI Terminal Fallback**: When running standalone, map Tree-sitter query captures to 16-color ANSI rules (`Color::Magenta`, `Color::Blue`, `Color::Green`, `Color::Yellow`).

---

## 7. Interactive Rebase & Timeline Mutation Mechanics

```text
                       Git-tardis Timeline Mutations
                                     │
           ┌─────────────────────────┴─────────────────────────┐
           ▼                                                   ▼
  [1. Inline Rewrite]                                 [2. Edit Here]
   (Automated Fixup)                                  (Historical Pause)
           │                                                   │
  Stage edits:                                        Modify todo file:
  `git commit --fixup <target>`                       Replace `pick <target>` with `edit <target>`
           │                                          via `GIT_SEQUENCE_EDITOR`
  Run non-interactive rebase:                                  │
  `GIT_SEQUENCE_EDITOR="true"`                        Execute `git rebase -i <upstream>`
  `git rebase -i --autosquash --autostash`                     │
           │                                          Git stops at target commit
  Automated squash directly                                    │
  into target commit                                  Suspend TUI (raw mode off, alt screen off)
                                                               │
                                                      Spawn interactive `$SHELL`
                                                               │
                                                      User edits & completes rebase
                                                               │
                                                      Resume TUI (raw mode on, alt screen on)
```

### 7.1 "Inline Rewrite" (Silent Autosquash) Workflow
1. **Upstream Determination**: Query parent commit of $C_{target}$ (`git rev-parse --verify --quiet C_target~1`). If $C_{target}$ is the root commit, use `--root`.
2. **Staging**: Stage working tree modifications (`git add -u`).
3. **Fixup Commit**: Execute `git commit --fixup <C_target_hash>`, creating subject `fixup! <C_target_summary>`.
4. **Silent Rebase**: Execute:
   ```bash
   GIT_SEQUENCE_EDITOR="true" git rebase -i --autosquash --autostash <upstream>
   ```
   Setting `GIT_SEQUENCE_EDITOR="true"` accepts the autosquash sequence non-interactively, merging the fixup commit directly into $C_{target}$.

### 7.2 "Edit Here" (Pause, Suspend TUI, Edit, Resume) Workflow
1. **Cross-Platform Sequence Editor Helper**: To mark $C_{target}$ as `edit` without relying on system-specific tools (`sed`), Git-tardis passes its own binary path:
   ```bash
   GIT_SEQUENCE_EDITOR="/path/to/git-tardis --internal-sequence-editor-mark-edit <C_target_hash>" git rebase -i <upstream>
   ```
   When Git calls `git-tardis`, the binary rewrites `pick <C_target_hash>` to `edit <C_target_hash>` in `.git/rebase-merge/git-rebase-todo` and exits with code `0`.
2. **TUI Suspension**:
   ```rust
   crossterm::terminal::disable_raw_mode()?;
   crossterm::execute!(stdout, LeaveAlternateScreen, Show)?;
   ```
3. **Subshell Spawning**: Spawn `$SHELL` (or `/bin/sh` fallback). Control hands to the user inside the repository at commit $C_{target}$.
4. **Rebase Continuation & TUI Resumption**:
   When the subshell exits, check if `.git/rebase-merge` exists. If present, execute `git rebase --continue`. Re-enable raw mode and alternate screen (`enable_raw_mode()`, `EnterAlternateScreen`).

---

## 8. Conflict Detection & Resolution Strategy

### 8.1 "Inline Rewrite" Conflict Strategy: Fail-Fast & Auto-Clean
1. **Detection**: If `git rebase` exits with non-zero status and `.git/rebase-merge` exists, a conflict occurred during autosquash.
2. **Automatic Cleanup**: Git-tardis immediately executes `git rebase --abort` in the background. Autostashed uncommitted changes are restored automatically.
3. **TUI Overlay Modal**: Git-tardis displays an informative modal detailing the conflicting file and commit:
   ```text
   ┌─────────────────────────────────────────────────────────────┐
   │                   Inline Rewrite Failed                     │
   ├─────────────────────────────────────────────────────────────┤
   │ Changes in 'src/main.rs' conflict with commit 3aaba99       │
   │ ("Commit B: Feature component").                            │
   │                                                             │
   │ Rebase has been aborted and repository restored.            │
   │                                                             │
   │                 [Press Enter or Esc to dismiss]             │
   └─────────────────────────────────────────────────────────────┘
   ```
4. Pressing `Enter` or `Esc` dismisses the modal and restores normal TUI interaction.

### 8.2 "Edit Here" Conflict Strategy: Native Subshell Delegation
1. Conflict detection and manual merge resolution are delegated directly to the interactive subshell environment.
2. Git CLI outputs conflict details to standard terminal `stdout`/`stderr`.
3. The user resolves conflicts using standard tools (`nvim`, `git add`, `git rebase --continue`) within the subshell before returning to Git-tardis.

---

## 9. Verification & Architectural Sign-off

All core mechanisms specified in this document have been prototyped, benchmarked, and verified with dedicated test suites in `research/`:

- **RPC Communication & Autocommands**: Verified in `research/neovim-rpc-poc/`
- **Tree-sitter AST Function Queries**: Verified in `research/tree-sitter-poc/`
- **Ratatui Split Layout & Keyboard Routing**: Verified in `research/ratatui-layout-poc/`
- **Interactive Rebase & Sequence Editing**: Verified in `research/interactive-rebase-poc/`
- **Conflict Handling Lifecycle**: Verified in `docs/research/conflict-detection-and-resolution.md`
- **Neovim Lua Plugin Packaging**: Verified in `research/neovim-plugin-poc/`
- **Syntax Highlighting & Theme Syncing**: Verified in `research/syntax-highlighting-poc/`

---

*This technical specification serves as the authoritative architectural blueprint for the implementation of Git-tardis.*

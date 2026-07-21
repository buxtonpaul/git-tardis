# Prototype: Ratatui Sidebar Layout & Navigation States

This document details the findings and validated design decisions from our interactive terminal layout prototype.

The prototype is located in the codebase at `research/ratatui-layout-poc/`.

---

## 1. Rust Data Model for Panels and States

To manage a split interface with dynamic sidebars and context-specific active highlights, we established a clear, type-safe enum state machine:

### Structural State Definitions:
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
```

### Monolithic AppState Struct:
By keeping active selections for *all* sidebar views in a single state container, we preserve the user's scroll positions and selections even as they switch back and forth between different views:

```rust
pub struct AppState {
    pub active_panel: ActivePanel,
    pub sidebar_view: SidebarView,
    pub nav_mode: NavigationMode,

    // Selections are preserved independently
    pub files: Vec<String>,
    pub file_selected: usize,

    pub modified_files: Vec<String>,
    pub modified_selected: usize,

    pub commits: Vec<(String, String)>,
    pub commit_selected: usize,

    // Code state
    pub code_lines: Vec<String>,
    pub cursor_line: usize, // 1-based index
    
    pub running: bool,
}
```

---

## 2. Split Layout Drawing & Resize Management

Ratatui's `Layout` engine automatically handles terminal resizing on every render tick by calculating bounding rectangles (`Rect`) dynamically relative to the active frame's bounding box (`frame.area()`). 

We defined a cleanly segmented split model:

```rust
fn ui(frame: &mut Frame, state: &AppState) {
    // 1. Split terminal vertically into Header, Body, and Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tab view selector
            Constraint::Min(0),    // Main workspace
            Constraint::Length(3), // Footer keybinding status
        ])
        .split(frame.area());

    // 2. Split the main workspace horizontally
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(32), // Left Sidebar width
            Constraint::Percentage(68), // Right Code Viewer width
        ])
        .split(chunks[1]);

    // Render left sidebar on main_chunks[0]
    // Render right code viewer on main_chunks[1]
}
```

### Visual Polish Decisions:
* **Focus Borders**: The active pane gets a `Color::Cyan` border, while the inactive pane gets a `Color::DarkGray` border. This makes it instantly clear to the user where keystroke inputs are currently routed.
* **Centered Selections**: Highlights in lists change styles (e.g. `bg(Color::Blue).fg(Color::White)`) only when the containing panel is active, falling back to a subtle highlight (e.g. `fg(Color::Yellow)`) when focus shifts away.

---

## 3. Key Event Routing Matrix

Input handling is routed hierarchically based on the `active_panel` state. This prevents key collisions and keeps shortcut scopes intuitive:

### Input Routing Schema:
* **Global Shortcuts**:
  - `q` / `Esc`: Exits the application.
  - `Tab` / `h` / `l` / `Left` / `Right`: Swaps active panel focus (`Sidebar` $\leftrightarrow$ `CodeViewer`).
  - `1` / `2` / `3`: Switches the `SidebarView` tab immediately.
  - `m`: Cycles the `NavigationMode` (`File` $\rightarrow$ `Function` $\rightarrow$ `Line` $\rightarrow$ `File`).
* **Sidebar Scope Shortcuts**:
  - `j` / `k` (or `Up` / `Down`): Changes selected item index within the *active* sidebar view.
* **Code Viewer Scope Shortcuts**:
  - `j` / `k` (or `Up` / `Down`): Moves the code line cursor up/down.
  - `n` / `]` / `p` / `[`: Executes the time-travel next/previous jumps based on the selected `NavigationMode`!

---

## 4. Verdict & Summary of Findings

* **Verdict**: **The design state model is completely validated.** Split rendering and state persistence across tab switches feel natural and extremely fast. Hierarchical routing solves all keyboard layout overlapping problems.
* **Time-Travel Jumps**: Binding the jump behavior (`next`/`previous` commits) directly to the active `nav_mode` and the highlighted node/line under the code cursor creates an incredibly powerful and clean UX.

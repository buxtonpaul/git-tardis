# Research: Configurable Keybindings & Mode-Specific Navigation Architecture

This specification document details the design and implementation for the flexible keybinding configuration system in **Git-tardis**. It covers hierarchical keymap scopes (`global`, `sidebar`, `code_viewer`), multi-key sequence matching (`]f`, `]l`, `]m`), user configuration formats (TOML and Neovim Lua `setup()`), and mode-specific timeline navigation shortcuts.

---

## Executive Summary

To support power-user workflows and seamless Vim integration, Git-tardis requires a keybinding engine capable of:
1. **User Customization**: Reading keybindings from TOML configuration files (`~/.config/git-tardis/config.toml`) and Neovim `setup()` options.
2. **Mode-Specific Navigation Shortcuts**: Allowing direct jumping across historical commits using explicit navigation shortcuts (`]f`/`[f]` for Function Mode, `]l`/`[l]` for Line Mode, `]m`/`[m]` for File Mode), while preserving global cycle-and-jump keybindings (`]`/`[`).
3. **Hierarchical Scope Overrides**: Routing keystrokes through a 2-tier context hierarchy (`ActivePanelScope` $\rightarrow$ `GlobalScope`), allowing context-sensitive panel navigation while retaining global hotkeys like `<Tab>`, `q`, and `1`/`2`/`3`.

---

## 1. Keystroke Representation & Parsing

Keystrokes are parsed from standard Vim string notation into typed Rust data structures:

### Supported Notation:
- **Single Character**: `"j"`, `"k"`, `"q"`, `"]"`, `"["`, `"m"`
- **Special Keys**: `<Esc>`, `<Tab>`, `<CR>` (Enter), `<Up>`, `<Down>`, `<Left>`, `<Right>`
- **Control Modifiers**: `<C-d>`, `<C-u>`, `<C-f>`, `<C-b>`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyStroke {
    Char(char),
    Esc,
    Tab,
    Enter,
    Up,
    Down,
    Left,
    Right,
    Ctrl(char),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySequence(pub Vec<KeyStroke>);
```

---

## 2. Multi-Key Sequence Matching & Timeout Engine

When handling multi-key sequences (e.g., `]` vs `]f`), single-key inputs can be both a valid action and a prefix to longer sequences.

### Match Resolution Enum:
```rust
#[derive(Debug, PartialEq, Eq)]
pub enum MatchResult {
    FullMatch(Action),
    AmbiguousMatch(Action), // Exact match, but also prefix to longer sequence
    PrefixMatch,
    NoMatch,
}
```

### Timeout Resolution Algorithm:
1. **Keystroke Arrives**: Appended to `pending_keys: Vec<KeyStroke>`.
2. **Check Scope Hierarchy**: Query `KeymapRegistry::resolve(active_scope, &pending_keys)`.
   - `FullMatch(action)`: Execute action immediately and clear `pending_keys`.
   - `PrefixMatch`: Buffer keystroke and start a pending key timer (`timeoutlen = 500ms`).
   - `AmbiguousMatch(action)`: Buffer keystroke and start pending key timer.
     - If next key arrives before timeout (e.g. `'f'`), resolve `"]f"` $\rightarrow$ `FullMatch(JumpNextFunction)`.
     - If timer expires or non-matching key arrives, execute `action` (`JumpNextAuto`) and clear buffer.
   - `NoMatch`: Clear `pending_keys`.

---

## 3. Scope Hierarchy & Context Routing

Git-tardis organizes keymaps into three distinct scopes:

1. **`global`**: Hotkeys active regardless of which panel is focused (e.g., `<Tab>` panel toggle, `q` quit, `1`/`2`/`3` sidebar view switches, `m` navigation mode cycle).
2. **`sidebar`**: Actions active when the left Sidebar panel is focused (e.g., `j`/`k` vertical list selection, `<CR>` select file).
3. **`code_viewer`**: Actions active when the right Code Viewer panel is focused (e.g., `j`/`k` vertical line movement, `e` inline rewrite, `E` edit here, `]` / `[` timeline jumps).

### Context Resolution Order:
When a key sequence is entered:
1. Search active panel scope (`code_viewer` or `sidebar`).
2. If no match is found, fall back to searching `global` scope.

---

## 4. Mode-Specific Navigation Shortcuts

Git-tardis provides both auto-mode jumps and mode-explicit shortcuts:

| Key Binding | Target Scope | Action Enum | Behavior |
| :--- | :--- | :--- | :--- |
| `]` | `code_viewer` | `JumpNextAuto` | Jump to next commit using active navigation mode (`Commit`, `File`, `Function`, or `Line`). |
| `[` | `code_viewer` | `JumpPrevAuto` | Jump to previous commit using active navigation mode. |
| `<C-d>` | `global` | `HalfPageDown` | Scroll half-page down in active panel. |
| `<C-u>` | `global` | `HalfPageUp` | Scroll half-page up in active panel. |
| `<C-f>` / `<PageDown>` | `global` | `PageDown` | Scroll full-page down in active panel. |
| `<C-b>` / `<PageUp>` | `global` | `PageUp` | Scroll full-page up in active panel. |
| `<C-e>` | `code_viewer` | `ScrollLineDown` | Scroll viewport down 1 line. |
| `<C-y>` | `code_viewer` | `ScrollLineUp` | Scroll viewport up 1 line. |
| `zz` | `code_viewer` | `CenterCursor` | Center current cursor line in viewport. |
| `zt` | `code_viewer` | `CursorTop` | Position current cursor line at top of viewport. |
| `zb` | `code_viewer` | `CursorBottom` | Position current cursor line at bottom of viewport. |

---

## 5. User Configuration Schemas (TOML & Lua)

### TOML Configuration (`~/.config/git-tardis/config.toml`)

Keybindings can be assigned a single string or an array of strings:

```toml
[keymaps.global]
quit = ["q", "<Esc>"]
toggle_focus = ["<Tab>", "h", "l"]
view_files = "1"
view_modified = "2"
view_timeline = "3"
view_candidates = "4"
cycle_nav_mode = "m"

[keymaps.sidebar]
move_up = ["k", "<Up>"]
move_down = ["j", "<Down>"]
select = ["<CR>", "l"]

[keymaps.code_viewer]
move_up = ["k", "<Up>"]
move_down = ["j", "<Down>"]
jump_next = "]"
jump_prev = "["
jump_next_function = ["]f", "<C-f>"]
jump_prev_function = ["[f", "<C-b>"]
jump_next_line = "]l"
jump_prev_line = "[l"
inline_rewrite = "e"
edit_here = "E"
```

### Neovim Lua Configuration (`git-tardis.setup`)

```lua
require("git-tardis").setup({
  keymaps = {
    global = {
      quit = { "q", "<Esc>" },
      toggle_focus = { "<Tab>", "h", "l" },
      cycle_nav_mode = "m",
    },
    code_viewer = {
      jump_next = "]",
      jump_prev = "[",
      jump_next_function = { "]f", "<C-f>" },
      jump_prev_function = { "[f", "<C-b>" },
      jump_next_line = "]l",
      jump_prev_line = "[l",
    },
    sidebar = {
      move_up = "k",
      move_down = "j",
      select = "<CR>",
    },
  },
})
```

---

## 6. Proof-of-Concept Verification

The design has been verified in `research/keybindings-poc`:

- **Keystroke Parsing**: Correctly parses single characters, Vim special keys (`<CR>`, `<Tab>`, `<Esc>`), and control sequences (`<C-d>`).
- **Multi-Key Resolution**: Validated `AmbiguousMatch` disambiguation for `]` vs `]f`/`]l`/`]m`.
- **Scope Hierarchy**: Verified fallback from `Sidebar` scope to `Global` scope.
- **TOML Config Deserialization**: Successfully overrides default keybindings from TOML strings.

```
running 3 tests
test tests::test_key_stroke_parsing ... ok
test tests::test_sequence_resolution_and_scope_override ... ok
test tests::test_toml_keymap_override ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

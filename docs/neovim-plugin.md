# Neovim Plugin Integration: `git-tardis.nvim`

`git-tardis.nvim` is a native Neovim plugin for **Git-tardis** that launches the TUI inside a floating terminal window with automatic cursor position, active buffer path, launch mode, and color theme synchronization over Neovim RPC (`$NVIM`).

---

## 1. Launch Modes

Git-tardis supports five distinct launch modes when triggered from Neovim:

| Launch Mode | CLI Flag | User Command | Lua API Function | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Auto / Default** | *(none)* | `:GitTardis` / `:GitTardisToggle` | `tardis.open()` / `tardis.toggle()` | Opens Git-tardis at active file and line in default navigation mode |
| **Function** | `-m function` | `:InspectPrevFunctionCommitAtLine` | `tardis.inspect_prev_function_commit_at_line()` | Instantly jumps back to the previous commit modifying the enclosing function |
| **Line** | `-m line` | `:InspectPrevLineCommitAtLine` | `tardis.inspect_prev_line_commit_at_line()` | Instantly jumps back to the previous commit modifying the active cursor line |
| **File** | `-m file` | `:InspectPrevFileCommitAtLine` | `tardis.inspect_prev_file_commit_at_line()` | Instantly jumps back to the previous commit modifying the current file |
| **Commit** | `-m commit` | `:InspectPrevCommit` | `tardis.inspect_prev_commit()` | Instantly jumps back to the previous commit in general commit history |

---

## 2. Installation & Keymap Configuration

### `lazy.nvim` (Recommended)

Configure keybindings in `lazy.nvim` to quickly launch Git-tardis in any mode from your active buffer:

```lua
{
  "buxtonpaul/git-tardis",
  cmd = {
    "GitTardis",
    "GitTardisToggle",
    "InspectPrevFunctionCommitAtLine",
    "InspectPrevLineCommitAtLine",
    "InspectPrevFileCommitAtLine",
    "InspectPrevCommit",
  },
  keys = {
    -- Toggle default view
    { "<leader>gt", "<cmd>GitTardisToggle<cr>", desc = "Toggle Git-tardis" },

    -- Launch in specific jump modes directly from active cursor line
    { "<leader>gf", "<cmd>InspectPrevFunctionCommitAtLine<cr>", desc = "Git-tardis: Jump Prev Function Commit" },
    { "<leader>gl", "<cmd>InspectPrevLineCommitAtLine<cr>", desc = "Git-tardis: Jump Prev Line Commit" },
    { "<leader>gF", "<cmd>InspectPrevFileCommitAtLine<cr>", desc = "Git-tardis: Jump Prev File Commit" },
    { "<leader>gc", "<cmd>InspectPrevCommit<cr>", desc = "Git-tardis: Jump Prev Commit" },
  },
  opts = {
    binary_path = "git-tardis", -- path to binary if not in $PATH
    window = {
      width = 0.85,
      height = 0.85,
      border = "rounded",
    },
    sync_theme = true,
  },
}
```

---

### Vanilla Lua (`init.lua`)

If you configure Neovim using `vim.keymap.set`:

```lua
local tardis = require("git-tardis")

tardis.setup({
  window = { border = "rounded" },
  sync_theme = true,
})

-- Keymaps via User Commands
vim.keymap.set("n", "<leader>gt", "<cmd>GitTardisToggle<cr>", { desc = "Toggle Git-tardis" })
vim.keymap.set("n", "<leader>gf", "<cmd>InspectPrevFunctionCommitAtLine<cr>", { desc = "Git-tardis: Jump Prev Function Commit" })
vim.keymap.set("n", "<leader>gl", "<cmd>InspectPrevLineCommitAtLine<cr>", { desc = "Git-tardis: Jump Prev Line Commit" })
vim.keymap.set("n", "<leader>gF", "<cmd>InspectPrevFileCommitAtLine<cr>", { desc = "Git-tardis: Jump Prev File Commit" })
vim.keymap.set("n", "<leader>gc", "<cmd>InspectPrevCommit<cr>", { desc = "Git-tardis: Jump Prev Commit" })

-- Alternatively, using Lua module functions directly:
vim.keymap.set("n", "<leader>gf", function()
  tardis.inspect_prev_function_commit_at_line()
end, { desc = "Git-tardis: Jump Prev Function Commit" })
```

---

### `pckr.nvim` / `packer.nvim`

```lua
use({
  "buxtonpaul/git-tardis",
  config = function()
    local tardis = require("git-tardis")
    tardis.setup({ window = { border = "rounded" } })

    vim.keymap.set("n", "<leader>gt", "<cmd>GitTardisToggle<cr>", { desc = "Toggle Git-tardis" })
    vim.keymap.set("n", "<leader>gf", "<cmd>InspectPrevFunctionCommitAtLine<cr>", { desc = "Jump Prev Function Commit" })
    vim.keymap.set("n", "<leader>gl", "<cmd>InspectPrevLineCommitAtLine<cr>", { desc = "Jump Prev Line Commit" })
    vim.keymap.set("n", "<leader>gF", "<cmd>InspectPrevFileCommitAtLine<cr>", { desc = "Jump Prev File Commit" })
    vim.keymap.set("n", "<leader>gc", "<cmd>InspectPrevCommit<cr>", { desc = "Jump Prev Commit" })
  end,
})
```

---

## 3. Configuration Options

Pass custom options to `require("git-tardis").setup(opts)`:

```lua
require("git-tardis").setup({
  -- Path to the git-tardis binary executable
  binary_path = "git-tardis",

  -- Floating window geometry and border
  window = {
    width = 0.85,       -- 85% of editor width
    height = 0.85,      -- 85% of editor height
    border = "rounded", -- "single", "double", "rounded", "solid", "shadow"
  },

  -- Global Neovim keymaps
  keymaps = {
    toggle = "<leader>gt", -- Set to false to disable default keymap
  },

  -- Automatically pass active Neovim Normal highlight colors (GIT_TARDIS_BG / FG)
  sync_theme = true,
})
```

---

## 4. Commands & Lua API Reference

### User Commands

- `:GitTardis [path]` — Open Git-tardis targeting `path`, or the active buffer if no path is supplied.
- `:GitTardisToggle` — Toggle the floating terminal window open or closed.
- `:InspectPrevFunctionCommitAtLine [path]` — Open Git-tardis and instantly step back to the previous commit modifying the enclosing function at the cursor line.
- `:InspectPrevLineCommitAtLine [path]` — Open Git-tardis and instantly step back to the previous commit modifying the cursor line.
- `:InspectPrevFileCommitAtLine [path]` — Open Git-tardis and instantly step back to the previous commit modifying the current file.
- `:InspectPrevCommit [path]` — Open Git-tardis and instantly step back to the previous commit in general history.

### Lua Module API

```lua
local tardis = require("git-tardis")

-- Open/toggle with custom path or jump mode
tardis.open(custom_path, jump_mode)   -- e.g. tardis.open(nil, "function")
tardis.toggle(custom_path, jump_mode) -- e.g. tardis.toggle(nil, "line")
tardis.close()                        -- Close floating window

-- Mode-specific launch actions
tardis.inspect_prev_function_commit_at_line(custom_path)
tardis.inspect_prev_line_commit_at_line(custom_path)
tardis.inspect_prev_file_commit_at_line(custom_path)
tardis.inspect_prev_commit(custom_path)
```

---

## 5. How Editor Context & Theme Syncing Work

When `GitTardis` or any launch mode command is executed from an active Neovim buffer:
1. **Cursor & File Focus**: Automatically passes `--file <buffer_path>` and `--line <cursor_line>` to `git-tardis`, jumping directly to the corresponding file and line in the TUI.
2. **Jump Mode Resolution**: Passes `--jump-mode <mode>` (e.g. `function`, `line`, `file`, or `commit`) so Git-tardis immediately executes historical time travel upon launching.
3. **Theme Inheritance**: Queries active Neovim highlight group colors (`Normal` `fg` and `bg`) and passes `GIT_TARDIS_BG` and `GIT_TARDIS_FG` in the terminal environment so Git-tardis matches your active Neovim colorscheme.

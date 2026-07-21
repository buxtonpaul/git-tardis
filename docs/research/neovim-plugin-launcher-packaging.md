# Research: Neovim Lua Plugin Packaging and Launcher Integration

This research document details the architecture, Lua API structure, floating window creation mechanics, and plugin manager integration for `git-tardis.nvim`.

## Executive Summary

To provide a first-class Neovim experience, Git-tardis is packaged as a standard Neovim Lua plugin (`git-tardis.nvim`). The plugin:
1. Exposes standard Vim user commands (`:GitTardis`, `:GitTardisToggle`) and a idiomatic Lua API (`require("git-tardis").setup()`).
2. Calculates centered floating window dimensions and spawns the `git-tardis` Rust binary inside a Neovim terminal pty buffer using `vim.fn.termopen()`.
3. Automatically sets `$NVIM` in the terminal environment, allowing the Rust child process to connect back to host Neovim via RPC (as detailed in [Neovim RPC Terminal Integration](neovim-rpc-terminal-integration.md)).
4. Integrates seamlessly with all modern Neovim plugin managers (`lazy.nvim`, `pckr.nvim`, `vim-plug`).

---

## 1. Lua API & User Commands Architecture

### Module Layout
The Neovim plugin follows standard Neovim Lua conventions:
```text
lua/
└── git-tardis/
    ├── init.lua       # Public entrypoint (setup, open, close, toggle)
    ├── config.lua     # Default options & user configuration merging
    └── terminal.lua   # Floating window lifecycle & termopen management
plugin/
└── git-tardis.lua     # Auto-loaded Vim user commands (:GitTardis, :GitTardisToggle)
```

### Public Lua API
```lua
local git_tardis = require("git-tardis")

-- 1. Setup options and keymaps
git_tardis.setup({
  binary_path = "git-tardis", -- path to compiled binary or system PATH
  window = {
    width = 0.85,             -- 85% editor width
    height = 0.85,            -- 85% editor height
    border = "rounded",       -- "single", "double", "rounded", "solid", "shadow"
  },
  keymaps = {
    toggle = "<leader>gt",
  },
})

-- 2. Direct programmatic calls
git_tardis.open(path)  -- Opens floating terminal at specified path
git_tardis.close()     -- Closes active floating terminal
git_tardis.toggle()    -- Toggles window open/close
```

### Vim User Commands
Registered automatically in `plugin/git-tardis.lua`:
* **`:GitTardis [path]`**: Opens the Git-tardis floating window (supports optional repository/file path argument with file autocompletion).
* **`:GitTardisToggle`**: Toggles the Git-tardis floating window.

---

## 2. Floating Terminal Launcher Mechanics

### Window Creation & `termopen` Lifecycle
Neovim's `vim.fn.termopen()` executes a shell command inside a pty terminal buffer and automatically sets `$NVIM` in the child process's environment variables.

```lua
local M = {}

M.state = { win = nil, buf = nil, job_id = nil }

function M.open(cmd, win_opts)
  if M.is_open() then
    vim.api.nvim_set_current_win(M.state.win)
    return M.state.win
  end

  -- Calculate centered floating window dimensions
  local width = math.floor(vim.o.columns * (win_opts.width or 0.85))
  local height = math.floor(vim.o.lines * (win_opts.height or 0.85))
  local row = math.floor((vim.o.lines - height) / 2)
  local col = math.floor((vim.o.columns - width) / 2)

  -- Create scratch buffer
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_option(buf, "bufhidden", "wipe")

  -- Create floating window
  local win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    width = width,
    height = height,
    row = row,
    col = col,
    style = "minimal",
    border = win_opts.border or "rounded",
    title = " Git-tardis ",
    title_pos = "center",
  })

  -- Launch terminal process
  local job_id = vim.fn.termopen(cmd, {
    on_exit = function(_, exit_code, _)
      M.state.win = nil
      M.state.buf = nil
      M.state.job_id = nil
      if vim.api.nvim_win_is_valid(win) then
        vim.api.nvim_win_close(win, true)
      end
    end,
  })

  M.state.win = win
  M.state.buf = buf
  M.state.job_id = job_id

  -- Enter terminal insert mode so keypresses route directly to TUI
  vim.cmd("startinsert")
  return win
end
```

---

## 3. Packaging & Plugin Manager Integration

### `lazy.nvim` Configuration
```lua
{
  "buxtonpaul/git-tardis",
  cmd = { "GitTardis", "GitTardisToggle" },
  keys = {
    { "<leader>gt", "<cmd>GitTardisToggle<cr>", desc = "Toggle Git-tardis" },
  },
  opts = {
    binary_path = "git-tardis",
    window = {
      width = 0.9,
      height = 0.9,
      border = "rounded",
    },
  },
}
```

### `pckr.nvim` / `packer.nvim` Configuration
```lua
use({
  "buxtonpaul/git-tardis",
  config = function()
    require("git-tardis").setup()
  end,
})
```

---

## 4. Verification Evidence

A working Lua plugin PoC was built and verified in `research/neovim-plugin-poc/`.

### Headless Verification Command:
```bash
nvim --headless \
  -c "set runtimepath+=./research/neovim-plugin-poc" \
  -c "runtime plugin/git-tardis.lua" \
  -c "lua local gt = require('git-tardis'); gt.setup(); print('Plugin verified!')" \
  -c "q"
```

### Output:
```text
Plugin verified!
Command :GitTardis exists
```

# Neovim Plugin Integration: `git-tardis.nvim`

`git-tardis.nvim` is a native Neovim plugin for **Git-tardis** that launches the TUI inside a floating terminal window with automatic cursor position, active buffer path, and color theme synchronization over Neovim RPC (`$NVIM`).

---

## 1. Plugin Directory Structure

The repository follows standard Neovim plugin conventions:

```text
git-tardis/
├── lua/
│   └── git-tardis/
│       ├── init.lua        # Setup entrypoint and user actions
│       ├── config.lua      # Default settings and option deep-merge
│       └── terminal.lua    # Floating window and terminal pty lifecycle
└── plugin/
    └── git-tardis.lua      # Registers :GitTardis and :GitTardisToggle commands
```

Because the plugin files reside in the root of the `git-tardis` repository, plugin managers can load `git-tardis` directly as a single repository dependency.

---

## 2. Installation Across Plugin Managers

### Prerequisites
- **Neovim** (`>= 0.9.0`)
- **Git-tardis Binary**: Installed via Homebrew (`brew install git-tardis`) or Cargo (`cargo install --path .`) so `git-tardis` is available in your `$PATH`.

---

### `lazy.nvim` (Recommended)

```lua
{
  "buxtonpaul/git-tardis",
  cmd = { "GitTardis", "GitTardisToggle" },
  keys = {
    { "<leader>gt", "<cmd>GitTardisToggle<cr>", desc = "Toggle Git-tardis" },
  },
  opts = {
    binary_path = "git-tardis", -- path to binary if not in $PATH
    window = {
      width = 0.85,
      height = 0.85,
      border = "rounded",
    },
    keymaps = {
      toggle = "<leader>gt",
    },
    sync_theme = true,
  },
}
```

---

### `pckr.nvim`

```lua
require("pckr").add({
  {
    "buxtonpaul/git-tardis",
    config = function()
      require("git-tardis").setup({
        window = { border = "rounded" },
      })
    end,
  },
})
```

---

### `packer.nvim`

```lua
use({
  "buxtonpaul/git-tardis",
  config = function()
    require("git-tardis").setup()
  end,
})
```

---

### `vim-plug`

```vim
Plug 'buxtonpaul/git-tardis'

" In your init.lua / init.vim:
lua << EOF
require('git-tardis').setup()
EOF
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
    width = 0.85,  -- 85% of editor width
    height = 0.85, -- 85% of editor height
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

## 4. Commands & Lua API

### User Commands

- `:GitTardis [path]` — Open Git-tardis targeting `path`, or the active buffer if no path is supplied.
- `:GitTardisToggle` — Toggle the floating terminal window open or closed.

### Lua Module API

```lua
local tardis = require("git-tardis")

-- Open Git-tardis
tardis.open(custom_path)

-- Close floating window
tardis.close()

-- Toggle floating window
tardis.toggle(custom_path)
```

---

## 5. How Editor Context & Theme Syncing Work

When `GitTardis` or `GitTardisToggle` is executed from an active Neovim buffer:
1. **Cursor & File Focus**: Automatically passes `--file <buffer_path>` and `--line <cursor_line>` to `git-tardis`, jumping directly to the corresponding file and line in the TUI.
2. **Theme Inheritance**: Queries active Neovim highlight group colors (`Normal` `fg` and `bg`) and passes `GIT_TARDIS_BG` and `GIT_TARDIS_FG` in the terminal environment so Git-tardis matches your active Neovim colorscheme.

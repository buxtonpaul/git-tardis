# Installation Guide: Git-tardis

This guide covers installing **Git-tardis** on macOS and Linux via Homebrew, on Windows from a release archive, or building directly from source code.

---

## Prerequisites

- **Git** (`>= 2.30`)
- **Rust Toolchain** (required for building from source: `cargo`, `rustc >= 1.80`)
- **Neovim** (optional, `>= 0.9.0` for `git-tardis.nvim` floating window RPC integration)

---

## 1. Installation via Homebrew (macOS & Linux)

You can install Git-tardis using Homebrew via the official tap:

```bash
brew tap buxtonpaul/tap
brew install git-tardis
```

Alternatively, install directly using the formula in this repository:

```bash
brew install buxtonpaul/tap/git-tardis
```

To upgrade to the latest release:

```bash
brew upgrade git-tardis
```

---

## 2. Installation on Windows

Git-tardis needs [Git for Windows](https://git-scm.com/download/win) with `git` available on your `PATH`. [Windows Terminal](https://aka.ms/terminal) is recommended; the older console host may not draw every symbol.

### Option A: Release archive

1. Download `git-tardis-x86_64-pc-windows-msvc.zip` from the [latest release](https://github.com/buxtonpaul/git-tardis/releases/latest).
2. Extract it to a folder of your choice, for example `%LOCALAPPDATA%\Programs\git-tardis`.
3. Add that folder to your `PATH`.

In PowerShell:

```powershell
$dest = "$env:LOCALAPPDATA\Programs\git-tardis"
Invoke-WebRequest https://github.com/buxtonpaul/git-tardis/releases/latest/download/git-tardis-x86_64-pc-windows-msvc.zip -OutFile git-tardis.zip
Expand-Archive git-tardis.zip -DestinationPath $dest -Force
[Environment]::SetEnvironmentVariable("Path", "$([Environment]::GetEnvironmentVariable('Path', 'User'));$dest", "User")
```

Open a new terminal and verify the installation:

```powershell
git-tardis --version
```

The archive's SHA-256 checksum is published next to it as `git-tardis-x86_64-pc-windows-msvc.zip.sha256`. The executable is self-contained and does not need the Visual C++ redistributable.

### Option B: Cargo

With the [Rust toolchain](https://rustup.rs) and the Visual Studio C++ build tools installed:

```powershell
cargo install --git https://github.com/buxtonpaul/git-tardis
```

### Notes for Windows

- **Configuration file:** `%APPDATA%\git-tardis\config.toml`. If `HOME` is set, as it is in Git Bash, `%HOME%\.config\git-tardis\config.toml` is used instead.
- **Edit Here:** the paused-rebase shell is `cmd.exe` (from `COMSPEC`), or `$SHELL` when that is set. Type `exit` to resume.

---

## 3. Building & Installing from Source

If you prefer to compile Git-tardis manually or contribute to development:

### Step 1: Clone the Repository

```bash
git clone https://github.com/buxtonpaul/git-tardis.git
cd git-tardis
```

### Step 2: Build the Optimized Release Binary

```bash
cargo build --release
```

The compiled executable will be located at:

```text
./target/release/git-tardis
```

### Step 3: Install System-Wide

Install the binary into your Cargo binary directory (`~/.cargo/bin/`):

```bash
cargo install --path .
```

Ensure `~/.cargo/bin` is in your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Verify the installation:

```bash
git-tardis --version
```

---

## 4. Neovim Plugin Setup (`git-tardis.nvim`)

Git-tardis includes a native Neovim plugin launcher (`git-tardis.nvim`) that opens Git-tardis in a floating terminal window with automatic editor position and theme synchronization over Neovim RPC (`$NVIM`).

For complete plugin documentation, options, and commands, see [docs/neovim-plugin.md](neovim-plugin.md).

### Quick Setup by Plugin Manager

#### `lazy.nvim`
```lua
{
  "buxtonpaul/git-tardis",
  cmd = { "GitTardis", "GitTardisToggle" },
  keys = {
    { "<leader>gt", "<cmd>GitTardisToggle<cr>", desc = "Toggle Git-tardis" },
  },
  config = function()
    require("git-tardis").setup({})
  end,
}
```

#### `pckr.nvim`
```lua
require("pckr").add({
  {
    "buxtonpaul/git-tardis",
    config = function()
      require("git-tardis").setup()
    end,
  },
})
```

#### `vim-plug`
```vim
Plug 'buxtonpaul/git-tardis'

lua require('git-tardis').setup()
```

### User Commands

- `:GitTardis [path]` — Open Git-tardis targeting a specific repository or file path.
- `:GitTardisToggle` — Toggle the floating Git-tardis terminal window.

---

## 5. Basic CLI Usage

Launch Git-tardis in the current repository:

```bash
git-tardis
```

Launch targeting a specific directory:

```bash
git-tardis /path/to/repo
```

Focus directly on a specific file and line number on startup:

```bash
git-tardis -f src/main.rs -l 42
```

For a list of all keybindings and interactive shortcuts, press `?` inside Git-tardis or view the help overlay.

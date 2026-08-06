-- Automated test suite for git-tardis.nvim running under headless Neovim (nvim --headless)
vim.opt.runtimepath:append(".")

print("[TEST] Loading git-tardis Lua module...")
local git_tardis = require("git-tardis")
local terminal = require("git-tardis.terminal")

assert(git_tardis ~= nil, "git-tardis module should be non-nil")

print("[TEST] Running setup()...")
local opts = git_tardis.setup({
  binary_path = "echo",
  sync_theme = true,
})
assert(opts.binary_path == "echo", "binary_path option set correctly")

print("[TEST] Checking user commands registration...")
require("plugin.git-tardis")
local commands = vim.api.nvim_get_commands({})
assert(commands["GitTardis"] ~= nil, "User command :GitTardis should be registered")
assert(commands["GitTardisToggle"] ~= nil, "User command :GitTardisToggle should be registered")
assert(commands["InspectPrevFunctionCommitAtLine"] ~= nil, "User command :InspectPrevFunctionCommitAtLine should be registered")
assert(commands["InspectPrevLineCommitAtLine"] ~= nil, "User command :InspectPrevLineCommitAtLine should be registered")
assert(commands["InspectPrevFileCommitAtLine"] ~= nil, "User command :InspectPrevFileCommitAtLine should be registered")
assert(commands["InspectPrevCommit"] ~= nil, "User command :InspectPrevCommit should be registered")

print("[TEST] Creating test buffer and window...")
local test_buf = vim.api.nvim_create_buf(true, false)
vim.api.nvim_buf_set_name(test_buf, "src/main.rs")
vim.api.nvim_buf_set_lines(test_buf, 0, -1, false, { "fn main() {", '    println!("Hello");', "}" })
vim.api.nvim_set_current_buf(test_buf)
vim.api.nvim_win_set_cursor(0, { 2, 4 })

print("[TEST] Opening git-tardis floating terminal...")
git_tardis.open()

assert(terminal.is_open() == true, "Terminal window should be open")
local win = terminal.state.win
assert(vim.api.nvim_win_is_valid(win), "Floating window handle should be valid")

print("[TEST] Closing git-tardis floating terminal...")
git_tardis.close()
assert(terminal.is_open() == false, "Terminal window should be closed")

print("[TEST] Toggling git-tardis floating terminal...")
git_tardis.toggle()
assert(terminal.is_open() == true, "Terminal window should be open after toggle")

git_tardis.toggle()
assert(terminal.is_open() == false, "Terminal window should be closed after second toggle")

print("[TEST] Testing InspectPrevFunctionCommitAtLine...")
git_tardis.inspect_prev_function_commit_at_line()
assert(terminal.is_open() == true, "Terminal window should be open for inspect_prev_function_commit_at_line")
git_tardis.close()

print("[TEST] Testing InspectPrevLineCommitAtLine...")
git_tardis.inspect_prev_line_commit_at_line()
assert(terminal.is_open() == true, "Terminal window should be open for inspect_prev_line_commit_at_line")
git_tardis.close()

print("[TEST] Testing InspectPrevFileCommitAtLine...")
git_tardis.inspect_prev_file_commit_at_line()
assert(terminal.is_open() == true, "Terminal window should be open for inspect_prev_file_commit_at_line")
git_tardis.close()

print("[TEST] Testing InspectPrevCommit...")
git_tardis.inspect_prev_commit()
assert(terminal.is_open() == true, "Terminal window should be open for inspect_prev_commit")
git_tardis.close()

print("SUCCESS: All Neovim plugin tests passed!")
vim.cmd("qall!")

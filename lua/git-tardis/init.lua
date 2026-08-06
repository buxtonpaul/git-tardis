local config = require("git-tardis.config")
local terminal = require("git-tardis.terminal")

local M = {}

function M.setup(user_opts)
  local opts = config.setup(user_opts)

  if opts.keymaps and opts.keymaps.toggle then
    vim.keymap.set("n", opts.keymaps.toggle, function()
      M.toggle()
    end, { desc = "Toggle Git-tardis" })
  end

  return opts
end

local function build_cmd_args(binary_path, custom_path, jump_mode)
  local args = { binary_path }

  if custom_path and custom_path ~= "" then
    table.insert(args, "--file")
    table.insert(args, custom_path)
  else
    local buf_name = vim.api.nvim_buf_get_name(0)
    if buf_name ~= "" and vim.bo.buftype == "" then
      table.insert(args, "--file")
      table.insert(args, buf_name)

      local cursor = vim.api.nvim_win_get_cursor(0)
      if cursor and cursor[1] then
        table.insert(args, "--line")
        table.insert(args, tostring(cursor[1]))
      end
    end
  end

  if jump_mode and jump_mode ~= "" then
    table.insert(args, "--jump-mode")
    table.insert(args, jump_mode)
  end

  return args
end

function M.open(custom_path, jump_mode)
  local opts = config.options
  if not opts.binary_path then
    opts = config.setup()
  end

  local cmd_args = build_cmd_args(opts.binary_path, custom_path, jump_mode)
  terminal.open(cmd_args, opts.window, opts.sync_theme)
end

function M.close()
  terminal.close()
end

function M.toggle(custom_path, jump_mode)
  local opts = config.options
  if not opts.binary_path then
    opts = config.setup()
  end

  if terminal.is_open() then
    terminal.close()
  else
    local cmd_args = build_cmd_args(opts.binary_path, custom_path, jump_mode)
    terminal.open(cmd_args, opts.window, opts.sync_theme)
  end
end

function M.inspect_prev_function_commit_at_line(custom_path)
  M.open(custom_path, "function")
end

function M.inspect_prev_line_commit_at_line(custom_path)
  M.open(custom_path, "line")
end

function M.inspect_prev_file_commit_at_line(custom_path)
  M.open(custom_path, "file")
end

function M.inspect_prev_commit(custom_path)
  M.open(custom_path, "commit")
end

return M

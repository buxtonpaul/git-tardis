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

function M.open(path)
  local opts = config.options
  if not opts.binary_path then
    opts = config.setup()
  end

  local cmd = opts.binary_path
  if path then
    cmd = cmd .. " " .. vim.fn.shellescape(path)
  end

  terminal.open(cmd, opts.window)
end

function M.close()
  terminal.close()
end

function M.toggle()
  local opts = config.options
  if not opts.binary_path then
    opts = config.setup()
  end

  terminal.toggle(opts.binary_path, opts.window)
end

return M

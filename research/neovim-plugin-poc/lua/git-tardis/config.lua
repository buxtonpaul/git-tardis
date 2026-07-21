local M = {}

M.defaults = {
  binary_path = "git-tardis",
  window = {
    width = 0.85,
    height = 0.85,
    border = "rounded",
  },
  keymaps = {
    toggle = "<leader>gt",
  },
}

M.options = {}

function M.setup(user_opts)
  M.options = vim.tbl_deep_extend("force", {}, M.defaults, user_opts or {})
  return M.options
end

return M

if vim.g.loaded_git_tardis == 1 then
  return
end
vim.g.loaded_git_tardis = 1

vim.api.nvim_create_user_command("GitTardis", function(opts)
  local path = opts.args ~= "" and opts.args or nil
  require("git-tardis").open(path)
end, {
  nargs = "?",
  complete = "file",
  desc = "Open Git-tardis floating terminal window",
})

vim.api.nvim_create_user_command("GitTardisToggle", function()
  require("git-tardis").toggle()
end, {
  desc = "Toggle Git-tardis floating terminal window",
})

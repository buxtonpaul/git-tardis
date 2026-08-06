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

vim.api.nvim_create_user_command("InspectPrevFunctionCommitAtLine", function(opts)
  local path = opts.args ~= "" and opts.args or nil
  require("git-tardis").inspect_prev_function_commit_at_line(path)
end, {
  nargs = "?",
  complete = "file",
  desc = "Open Git-tardis and jump to previous function-modifying commit at current line",
})

vim.api.nvim_create_user_command("InspectPrevLineCommitAtLine", function(opts)
  local path = opts.args ~= "" and opts.args or nil
  require("git-tardis").inspect_prev_line_commit_at_line(path)
end, {
  nargs = "?",
  complete = "file",
  desc = "Open Git-tardis and jump to previous line-modifying commit at current line",
})

vim.api.nvim_create_user_command("InspectPrevFileCommitAtLine", function(opts)
  local path = opts.args ~= "" and opts.args or nil
  require("git-tardis").inspect_prev_file_commit_at_line(path)
end, {
  nargs = "?",
  complete = "file",
  desc = "Open Git-tardis and jump to previous file-modifying commit at current file",
})

vim.api.nvim_create_user_command("InspectPrevCommit", function(opts)
  local path = opts.args ~= "" and opts.args or nil
  require("git-tardis").inspect_prev_commit(path)
end, {
  nargs = "?",
  complete = "file",
  desc = "Open Git-tardis and jump to previous commit",
})

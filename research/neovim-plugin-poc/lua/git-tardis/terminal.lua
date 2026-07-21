local M = {}

M.state = {
  win = nil,
  buf = nil,
  job_id = nil,
}

function M.is_open()
  return M.state.win ~= nil and vim.api.nvim_win_is_valid(M.state.win)
end

function M.open(cmd, win_opts)
  if M.is_open() then
    vim.api.nvim_set_current_win(M.state.win)
    return M.state.win
  end

  local width = math.floor(vim.o.columns * (win_opts.width or 0.85))
  local height = math.floor(vim.o.lines * (win_opts.height or 0.85))
  local row = math.floor((vim.o.lines - height) / 2)
  local col = math.floor((vim.o.columns - width) / 2)

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_option(buf, "bufhidden", "wipe")

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

  vim.cmd("startinsert")
  return win
end

function M.close()
  if M.is_open() then
    vim.api.nvim_win_close(M.state.win, true)
    M.state.win = nil
    M.state.buf = nil
    M.state.job_id = nil
  end
end

function M.toggle(cmd, win_opts)
  if M.is_open() then
    M.close()
  else
    M.open(cmd, win_opts)
  end
end

return M

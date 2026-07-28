local M = {}

M.state = {
  win = nil,
  buf = nil,
  job_id = nil,
}

function M.is_open()
  return M.state.win ~= nil and vim.api.nvim_win_is_valid(M.state.win)
end

local function get_hl_hex(name, attr)
  local status, hl = pcall(vim.api.nvim_get_hl, 0, { name = name, link = false })
  if status and hl and hl[attr] then
    return string.format("#%06x", hl[attr])
  end
  return nil
end

function M.get_theme_env()
  local env = {}
  local bg = get_hl_hex("Normal", "bg")
  local fg = get_hl_hex("Normal", "fg")
  if bg then
    env["GIT_TARDIS_BG"] = bg
  end
  if fg then
    env["GIT_TARDIS_FG"] = fg
  end
  return env
end

function M.open(cmd_args, win_opts, sync_theme)
  if M.is_open() then
    vim.api.nvim_set_current_win(M.state.win)
    return M.state.win
  end

  win_opts = win_opts or {}
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

  local env = {}
  if sync_theme then
    env = M.get_theme_env()
  end

  local job_id = vim.fn.termopen(cmd_args, {
    env = env,
    on_exit = function(_, exit_code, _)
      M.state.win = nil
      M.state.buf = nil
      M.state.job_id = nil
      if vim.api.nvim_win_is_valid(win) then
        pcall(vim.api.nvim_win_close, win, true)
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
    pcall(vim.api.nvim_win_close, M.state.win, true)
    M.state.win = nil
    M.state.buf = nil
    M.state.job_id = nil
  end
end

function M.toggle(cmd_args, win_opts, sync_theme)
  if M.is_open() then
    M.close()
  else
    M.open(cmd_args, win_opts, sync_theme)
  end
end

return M

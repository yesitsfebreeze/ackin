---
kind: routine
description: Serve bounded path-addressed language and native editor operations in each session's configured Neovim.
uses:
  - usage: "[[run-usage]]"
    when: [changing the agent editor behavior or understanding its native operation contract]
---

# neovim-control

[[terminal-control]] loads this fence into one explicitly associated editor.
Paths are absolute and positions one-based LSP positions. The request deadline
covers attachment, querying and positive control together. Initialization and
progress are hints, not evidence that an empty answer is trustworthy.
[[terminal]] and neovim-native-control-index explain native controls.

File conflicts retain buffer text. `state` returns each conflict's `token`;
`resolve` takes `buffer`, `token`, and `choice` (`keep` or `reload`). A dirty
reload also needs `discard: true`. Both versions are rechecked before changing
text. Keep does not write or overwrite disk. Existing FileChangedShell handlers
are reported rather than silently replaced; automatic checks are disabled if
such a handler is installed. Recovery input never implies approval. Automatic
following covers only the active worktree, not a shared memo checkout; open
that checkout explicitly when it is the task. State inspects at most 32 loaded
buffers under that root, plus the current buffer, without reloading them.
Agent-created file buffers are capped at 32. At capacity, only an agent-created,
hidden, clean buffer without a pending conflict may be removed. Existing user
buffers, visible buffers and unsaved work are never evicted; if no safe slot
exists, the operation fails instead of growing the editor. The watcher forgets
paths no longer present in its current dirty-file snapshot. Unsupported LSP
methods fail before dispatch, avoiding Neovim's informational prompt.

```lua
local previous = Terminal
if previous and previous.timer then previous.timer:stop(); previous.timer:close() end
local config = vim.g.terminal_config or {}
Terminal = { owner = previous and previous.owner or '', epoch = previous and previous.epoch or 0,
  generation = config.generation or 'test', conflicts = previous and previous.conflicts or {},
  root = config.root or vim.fn.getcwd(), receipt = config.receipt }
local T = Terminal
vim.lsp.config('rust_analyzer', {
  settings = { ['rust-analyzer'] = { cargo = { targetDir = true }, checkOnSave = false } },
})
Lsp = {}
local active
local function now() local s, u = vim.uv.gettimeofday(); return s + u / 1e6 end
local function remaining()
  assert(Terminal == T, 'editor generation changed')
  local ms = active and math.floor((active.deadline - now()) * 1000) or 8000
  assert(ms > 0, 'request deadline expired')
  return ms
end
local function checkpoint()
  remaining()
  if active then
    assert(active.generation == T.generation and active.epoch == T.epoch, 'ownership generation changed')
  end
end
local function receipt(request, status, result)
  if not T.receipt then return end
  local value = { id = request.id, state = status, generation = T.generation,
    owner = T.owner, epoch = T.epoch, result = result }
  vim.fn.writefile({ vim.json.encode(value) }, T.receipt .. '.lua-tmp')
  assert(vim.uv.fs_rename(T.receipt .. '.lua-tmp', T.receipt))
end
local function disk(path)
  local f = io.open(path, 'rb')
  if not f then return { exists = false, token = 'missing' } end
  local bytes = f:read('*a'); f:close()
  return { exists = true, token = vim.fn.sha256(bytes), bytes = bytes }
end
local function version(b)
  local path = vim.api.nvim_buf_get_name(b)
  local d = disk(path)
  return tostring(vim.api.nvim_buf_get_changedtick(b)) .. ':' .. d.token, d, path
end
local observed, kept = {}, {}
local managed = previous and previous.managed or {}
T.managed = managed
local function buffer(path)
  local existing = vim.fn.bufnr(path)
  if existing >= 0 then return existing end
  local live = {}
  for _, b in ipairs(managed) do
    if vim.api.nvim_buf_is_valid(b) then live[#live + 1] = b end
  end
  managed = live
  T.managed = managed
  if #managed >= 32 then
    for i, b in ipairs(managed) do
      if not vim.bo[b].modified and vim.bo[b].buftype == ''
        and #vim.fn.win_findbuf(b) == 0 and not T.conflicts[tostring(b)] then
        local ok = pcall(vim.api.nvim_buf_delete, b, { force = false })
        if ok then
          observed[b], kept[b] = nil, nil
          table.remove(managed, i)
          break
        end
      end
    end
  end
  assert(#managed < 32, 'editor buffer limit reached; preserve visible, dirty and conflicting buffers')
  local b = vim.fn.bufadd(path)
  managed[#managed + 1] = b
  return b
end
local function conflict(b, reason)
  local token, d, path = version(b)
  T.conflicts[tostring(b)] = { buffer = b, path = path, reason = reason,
    modified = vim.bo[b].modified, disk_exists = d.exists, token = token }
end
local group = vim.api.nvim_create_augroup('KernSessionTerminal', { clear = true })
local handlers = vim.api.nvim_get_autocmds({ event = 'FileChangedShell' })
T.file_policy = #handlers == 0 and 'managed' or 'existing FileChangedShell handlers; automatic checks disabled'
if #handlers == 0 then
  vim.api.nvim_create_autocmd('FileChangedShell', { group = group, callback = function(event)
    local b = event.buf
    if vim.v.fcs_reason == 'changed' and not vim.bo[b].modified then
      vim.v.fcs_choice = 'reload'
    else
      conflict(b, vim.v.fcs_reason)
      vim.v.fcs_choice = ''
    end
  end })
end
local function inspect_file(b, reload)
  if not vim.api.nvim_buf_is_loaded(b) or vim.bo[b].buftype ~= '' then return end
  local path = vim.api.nvim_buf_get_name(b)
  if path == '' then return end
  local token, d = version(b)
  if kept[b] == token then return end
  if observed[b] == nil then observed[b] = d.token; return end
  if observed[b] == d.token then return end
  if vim.bo[b].modified or not d.exists or T.file_policy ~= 'managed' or not reload then
    conflict(b, not d.exists and 'deleted' or 'changed')
    return
  end
  checkpoint()
  vim.cmd('silent checktime ' .. b)
  local content = table.concat(vim.api.nvim_buf_get_lines(b, 0, -1, false), '\n')
  if vim.bo[b].endofline then content = content .. '\n' end
  if vim.bo[b].fileformat == 'dos' then content = content:gsub('\n', '\r\n') end
  if content ~= d.bytes then conflict(b, 'reload deferred or content differs'); return end
  observed[b] = d.token
  T.conflicts[tostring(b)] = nil
end
vim.api.nvim_create_autocmd({ 'BufReadPost', 'BufWritePost' }, { group = group, callback = function(event)
  if vim.bo[event.buf].buftype == '' then observed[event.buf] = disk(vim.api.nvim_buf_get_name(event.buf)).token end
end })
for _, b in ipairs(vim.api.nvim_list_bufs()) do
  if vim.api.nvim_buf_is_loaded(b) then observed[b] = disk(vim.api.nvim_buf_get_name(b)).token end
end
local function safe_focus(requesting)
  if Terminal ~= T or T.owner ~= '' or (active and not requesting) or vim.api.nvim_get_mode().blocking
    or vim.api.nvim_get_mode().mode ~= 'n' or vim.bo.modified or vim.bo.buftype ~= '' then return false end
  for _, win in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
    if vim.api.nvim_win_get_config(win).relative ~= '' then return false end
  end
  return true
end
local function show(path, line, requesting)
  if not safe_focus(requesting) then return false end
  checkpoint()
  local b = buffer(path)
  if vim.bo[b].modified then return false end
  vim.fn.bufload(b)
  if not safe_focus(requesting) then return false end
  inspect_file(b, true)
  if T.conflicts[tostring(b)] or not safe_focus(requesting) then return false end
  checkpoint()
  vim.api.nvim_win_set_buf(0, b)
  if line then pcall(vim.api.nvim_win_set_cursor, 0, { line, 0 }) end
  return true
end
local prepared = {}
local function bufof(path)
  checkpoint()
  if prepared[path] then return prepared[path] end
  local b = buffer(path)
  vim.fn.bufload(b)
  inspect_file(b, safe_focus(true))
  assert(not T.conflicts[tostring(b)], 'pending file conflict')
  vim.wait(remaining(), function() return #vim.lsp.get_clients({ bufnr = b }) > 0 end, 20)
  checkpoint()
  assert(#vim.lsp.get_clients({ bufnr = b }) > 0, 'no language server attached')
  prepared[path] = b
  return b
end
local function ask(b, method, params)
  checkpoint()
  local supported = false
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = b })) do
    if client:supports_method(method, b) then supported = true; break end
  end
  assert(supported, 'language server does not support ' .. method)
  local results, failure = vim.lsp.buf_request_sync(b, method, params, remaining())
  checkpoint()
  assert(results, failure or 'language server did not answer')
  for _, response in pairs(results) do assert(not response.error, vim.inspect(response.error)) end
  return results
end
local CONTROL = 'ChunkPartKind'
local function answer(text, path)
  if text ~= '' then return text end
  local b = bufof(path)
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = b })) do
    if client.name ~= 'rust_analyzer' then
      return 'unavailable: empty result has no positive control for ' .. tostring(client.name)
    end
  end
  for _, response in pairs(ask(b, 'workspace/symbol', { query = CONTROL })) do
    if next(response.result or {}) then return '' end
  end
  return ('indexing: control %s answers nothing; %s'):format(CONTROL, vim.lsp.status():sub(1, 60))
end
local function short(uri)
  local prefix = 'file://' .. T.root .. '/'
  return (uri or ''):sub(1, #prefix) == prefix and uri:sub(#prefix + 1) or uri
end
local function at(path, line, col, method, extra)
  local b = bufof(path)
  local p = { textDocument = { uri = vim.uri_from_bufnr(b) }, position = { line = line - 1, character = col - 1 } }
  for k, v in pairs(extra or {}) do p[k] = v end
  return ask(b, method, p)
end
local function locations(res)
  local out = {}
  for _, v in pairs(res or {}) do
    local items = v.result or {}
    if items.uri or items.targetUri then items = { items } end
    for _, l in ipairs(items) do
      local range = l.range or l.targetSelectionRange
      out[#out + 1] = ('%s:%d'):format(short(l.uri or l.targetUri), range.start.line + 1)
    end
  end
  return table.concat(out, '\n')
end
function Lsp.references(path, line, col)
  return answer(locations(at(path, line, col, 'textDocument/references', { context = { includeDeclaration = false } })), path)
end
function Lsp.definition(path, line, col)
  return answer(locations(at(path, line, col, 'textDocument/definition')), path)
end
function Lsp.hover(path, line, col)
  for _, v in pairs(at(path, line, col, 'textDocument/hover')) do
    local c = v.result and v.result.contents
    if c then return type(c) == 'table' and (c.value or vim.inspect(c)) or tostring(c) end
  end
  return answer('', path)
end
function Lsp.rename_preview(path, line, col, newname)
  local out = {}
  for _, v in pairs(at(path, line, col, 'textDocument/rename', { newName = newname })) do
    for uri, edits in pairs((v.result or {}).changes or {}) do
      for _, e in ipairs(edits) do out[#out + 1] = ('%s:%d'):format(short(uri), e.range.start.line + 1) end
    end
    for _, change in ipairs((v.result or {}).documentChanges or {}) do
      for _, e in ipairs(change.edits or {}) do out[#out + 1] = ('%s:%d'):format(short(change.textDocument.uri), e.range.start.line + 1) end
    end
  end
  return answer(table.concat(out, '\n'), path)
end
function Lsp.workspace_symbols(path, query)
  local out = {}
  for _, v in pairs(ask(bufof(path), 'workspace/symbol', { query = query })) do
    for _, s in ipairs(v.result or {}) do
      out[#out + 1] = ('%s:%d  %s'):format(short(s.location.uri), s.location.range.start.line + 1, s.name)
    end
  end
  return answer(table.concat(out, '\n'), path)
end
function Lsp.document_symbols(path)
  local b, out = bufof(path), {}
  for _, v in pairs(ask(b, 'textDocument/documentSymbol', { textDocument = { uri = vim.uri_from_bufnr(b) } })) do
    for _, s in ipairs(v.result or {}) do
      local range = s.range or s.location.range
      out[#out + 1] = ('%d  %s'):format(range.start.line + 1, s.name)
    end
  end
  return answer(table.concat(out, '\n'), path)
end
function Lsp.diagnostics(path)
  local b, out = bufof(path), {}
  for _, d in ipairs(vim.diagnostic.get(b)) do
    out[#out + 1] = ('%d:%d %s %s'):format(d.lnum + 1, d.col + 1, vim.diagnostic.severity[d.severity], d.message:gsub('\n', ' '))
  end
  return answer(table.concat(out, '\n'), path)
end
function Lsp.index_status(path)
  local b = vim.fn.bufnr(path)
  return { clients = vim.tbl_map(function(c)
    return { name = c.name, initialized = c.initialized, attached = c.attached_buffers[b] == true }
  end, vim.lsp.get_clients()), progress = vim.lsp.status(), evidence = 'run a semantic query; progress is not readiness' }
end
local function mappings(mode, local_only)
  local maps = local_only and vim.api.nvim_buf_get_keymap(0, mode) or vim.api.nvim_get_keymap(mode)
  return vim.tbl_map(function(m)
    return { lhs = m.lhs, rhs = m.rhs, description = m.desc, callback = tostring(m.callback) }
  end, maps)
end
local function state()
  local b, mode = vim.api.nvim_get_current_buf(), vim.api.nvim_get_mode()
  inspect_file(b, false)
  local root, count = vim.uv.fs_realpath(T.root) or T.root, 0
  for _, other in ipairs(vim.api.nvim_list_bufs()) do
    if other ~= b and count < 32 and vim.api.nvim_buf_is_loaded(other)
      and vim.api.nvim_buf_get_name(other):sub(1, #root + 1) == root .. '/' then
      inspect_file(other, false)
      count = count + 1
    end
  end
  return { path = vim.api.nvim_buf_get_name(b), buffer = b, cursor = vim.api.nvim_win_get_cursor(0),
    mode = mode.mode, blocking = mode.blocking, modified = vim.bo[b].modified, owner = T.owner, epoch = T.epoch,
    cwd = vim.fn.getcwd(), root = T.root, lines = vim.api.nvim_buf_get_lines(b, 0, 200, false),
    conflicts = T.conflicts, file_policy = T.file_policy, mappings = mappings('n', true),
    windows = vim.tbl_map(function(w) return { id = w, buffer = vim.api.nvim_win_get_buf(w), config = vim.api.nvim_win_get_config(w) } end, vim.api.nvim_tabpage_list_wins(0)) }
end
local function capabilities(request)
  local filter, limit = request.filter or '', math.max(1, math.min(request.limit or 40, 100))
  local function selected(items, name)
    local out, total = {}, 0
    for _, item in ipairs(items) do
      local key = name and tostring(item[name] or '') or tostring(item)
      if key:lower():find(filter:lower(), 1, true) then
        total = total + 1
        if #out < limit then out[#out + 1] = item end
      end
    end
    return { items = out, total = total, truncated = total > limit }
  end
  local plugins = {}
  local ok, lazy = pcall(require, 'lazy.core.config')
  if ok then
    for name, p in pairs(lazy.plugins) do plugins[#plugins + 1] = { name = name, dir = p.dir,
      loaded = p._.loaded ~= nil, commands = p.cmd, url = p.url } end
    table.sort(plugins, function(a, b) return a.name < b.name end)
  end
  local api = vim.fn.api_info()
  return { version = api.version, api = selected(api.functions, 'name'),
    commands = selected(vim.fn.getcompletion('', 'command')),
    functions = selected(vim.fn.getcompletion('', 'function')),
    help = selected(vim.fn.getcompletion('', 'help')),
    plugins = selected(plugins, 'name'), mappings = { normal = selected(mappings('n'), 'lhs'),
      visual = selected(mappings('v'), 'lhs'), buffer = selected(mappings('n', true), 'lhs') },
    clients = vim.tbl_map(function(c) return { name = c.name, initialized = c.initialized,
      root = c.config.root_dir, capabilities = c.server_capabilities, attached = c.attached_buffers } end, vim.lsp.get_clients()),
    progress = vim.lsp.status() }
end
local function serializable(value, seen)
  if type(value) == 'function' or type(value) == 'thread' or type(value) == 'userdata' then return tostring(value) end
  if type(value) ~= 'table' then return value end
  seen = seen or {}
  if seen[value] then return '<cycle>' end
  seen[value] = true
  local result = {}
  for key, item in pairs(value) do result[key] = serializable(item, seen) end
  seen[value] = nil
  return result
end
function T.request(request)
  if active then return vim.json.encode({ ok = false, error = 'busy: editor request active' }) end
  if request.generation ~= T.generation or request.epoch ~= T.epoch or not request.deadline or request.deadline <= now() then
    receipt(request, 'expired')
    return vim.json.encode({ ok = false, error = 'expired request or ownership generation' })
  end
  active, prepared = request, {}
  receipt(request, 'started')
  local ok, result = pcall(function()
    checkpoint()
    if request.op == 'state' then return state() end
    if request.op == 'capabilities' then return capabilities(request) end
    if request.op == 'hold' then T.owner, T.epoch = request.owner, request.next_epoch; return T.owner end
    if request.op == 'scope' then
      assert(safe_focus(true), 'busy: scope change needs an unowned normal editor')
      T.root = request.path
      return T.root
    end
    if request.op == 'show' or request.op == 'query' then
      assert(type(request.path) == 'string' and request.path:sub(1, 1) == '/', 'absolute path required')
      show(request.path, request.line or (request.args or {})[1], true)
      if request.op == 'show' then return state() end
      assert(Lsp[request.method], 'unknown language query')
      return Lsp[request.method](request.path, unpack(request.args or {}))
    end
    assert(T.owner ~= '' and request.owner == T.owner, 'claim terminal before interacting')
    if request.op == 'resolve' then
      local b = request.buffer
      assert(vim.api.nvim_buf_is_valid(b), 'buffer no longer exists')
      local pending = T.conflicts[tostring(b)]
      assert(pending and pending.token == request.token, 'conflict token required')
      local token, d = version(b)
      assert(token == request.token, 'buffer or disk changed; inspect a fresh conflict')
      assert(request.choice == 'keep' or request.choice == 'reload', 'choice must be keep or reload')
      if request.choice == 'reload' then
        assert(d.exists, 'disk file missing; preserve buffer')
        assert(not vim.bo[b].modified or request.discard == true, 'dirty reload requires discard: true')
        checkpoint()
        vim.api.nvim_buf_call(b, function() vim.cmd('edit!') end)
        observed[b] = d.token
      else kept[b] = token end
      T.conflicts[tostring(b)] = nil
      return state()
    end
    if request.op == 'lua' then
      local fn, failure = loadstring(request.code)
      assert(fn, failure)
      return fn()
    end
    if request.op == 'command' then return vim.api.nvim_exec2(request.command, { output = true }).output end
    error('unknown editor operation')
  end)
  local response = ok and { ok = true, value = result == nil and vim.NIL or serializable(result) }
    or { ok = false, error = tostring(result) }
  active = nil
  receipt(request, 'completed', response)
  return vim.json.encode(response)
end
local seen, polling = {}, false
local function changed(root, output)
  local current = {}
  local rows, skip, newest, newest_at = vim.split(output, '\0', { plain = true, trimempty = true }), false
  for _, row in ipairs(rows) do
    if skip then skip = false else
      local status, relative = row:sub(1, 2), row:sub(4)
      skip = status:find('R') ~= nil or status:find('C') ~= nil
      local path = root .. '/' .. relative
      local stat = vim.uv.fs_stat(path)
      if stat and stat.type == 'file' then
        local at = stat.mtime.sec + stat.mtime.nsec / 1e9
        if seen[path] ~= at and (not newest_at or at > newest_at) then newest, newest_at = path, at end
        current[path] = at
      end
    end
  end
  seen = current
  return newest, newest_at
end
local function poll()
  if polling or not safe_focus() then return end
  polling = true
  local root_at_start = T.root
  local ok = pcall(vim.system, { 'git', '-C', root_at_start, 'status', '--porcelain=v1', '-z', '--untracked-files=all' },
    { timeout = 1500 }, function(result)
      vim.schedule(function()
        polling = false
        if Terminal ~= T or T.root ~= root_at_start or not safe_focus() then return end
        pcall(function()
          if result.code ~= 0 then return end
          local path = changed(root_at_start, result.stdout)
          if path and safe_focus() then show(path) end
        end)
      end)
    end)
  if not ok then polling = false end
end
T.timer = vim.uv.new_timer()
T.timer:start(2000, 2000, vim.schedule_wrap(poll))
```

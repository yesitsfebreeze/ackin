---
kind: routine
description: Open and control each session's visible Neovim through bounded native RPC and tmux input.
uses:
  - usage: "[[run-usage]]"
    when: [opening the task panes, inspecting their screens, or driving Neovim and lazygit from either agent]
---

# terminal-control

`just terminal open --session SESSION` binds the calling tmux pane to its own
right-hand editor. `--source-pane`, `--cwd`, and the equivalent
`KERN_TERMINAL_SOURCE`, `KERN_TERMINAL_SESSION` identify an interactive caller.
A hook must not supply its parent's association to a background agent.
Repeated opens preserve the binding and neighboring panes. `--lazygit` opens
an optional git UI; `--editor-pane` plus `--socket` explicitly bootstraps an
inspected, caller-owned editor, never an editor discovered by proximity.
Read [[terminal]] and neovim-native-control-index before operating.
[[neovim-control]] is the executable editor sibling.

Normal operations have nonblocking admission. Status, capture, and
owner-validated keys/text bypass admission so a prompt cannot lock out its
own recovery. For an unclaimed editor, `keys --recover --owner SESSION`
authorizes only input by its associated session; it does not claim the editor.
A timeout is an unknown outcome, not cancellation: inspect the screen and
receipt, resolve the identified prompt deliberately, and wait for completion.
Never replay a timed-out command. `status` exposes the last receipt without
asking a blocked editor. `open --cwd LANE` changes watcher scope only when the
editor is idle. A new session in an occupied source pane is refused; use a new
source pane, not silent adoption. Private runtime files are caches.

```python
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time
import uuid
from contextlib import contextmanager


def run(*argv, timeout=10):
    result = subprocess.run(argv, text=True, capture_output=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or str(argv))
    return result.stdout.strip()


def source(leaf, language):
    path = ROOT / '.kern' / 'memos' / 'system' / (leaf + '.md')
    marker = '```' + language + '\n'
    text = path.read_text()
    if text.count(marker) != 1:
        raise RuntimeError('expected one executable fence: ' + leaf)
    return text.split(marker, 1)[1].split('\n```', 1)[0]


def tmux(*argv):
    return run('tmux', *argv)


def read(path, default=None):
    return json.loads(path.read_text()) if path.exists() else default


def save(path, value):
    temporary = path.with_suffix('.tmp-' + uuid.uuid4().hex)
    temporary.write_text(json.dumps(value))
    temporary.replace(path)


@contextmanager
def admission():
    with (RUNTIME / 'lock').open('a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise RuntimeError('busy: another request is in flight')
        yield


def native(method, args=(), timeout=1):
    payload = json.dumps({'socket': str(SOCKET), 'method': method, 'args': args})
    code = '''local p=vim.json.decode(vim.env.KERN_RPC)
local c=vim.fn.sockconnect('pipe',p.socket,{rpc=true})
assert(c>0,'socket unavailable')
local ok,value=pcall(vim.rpcrequest,c,p.method,unpack(p.args))
io.stdout:write(vim.json.encode({ok=ok,value=value})); io.stdout:flush()
vim.cmd('qa!')'''
    result = subprocess.run(['nvim', '--headless', '-u', 'NONE', '-i', 'NONE', '-n', '-c', 'lua ' + code],
                            env=dict(os.environ, KERN_RPC=payload, NVIM_LOG_FILE=str(RUNTIME / 'client.log')),
                            text=True, capture_output=True, timeout=timeout)
    if result.returncode or not result.stdout:
        raise RuntimeError(result.stderr.strip() or 'native RPC failed')
    reply = json.loads(result.stdout)
    if not reply['ok']:
        raise RuntimeError(str(reply['value']))
    return reply['value']


def receipt():
    return read(RUNTIME / 'receipt.json', {})


def remote(request, budget=8):
    previous = receipt()
    if previous.get('state') in ('submitted', 'started'):
        raise RuntimeError('unknown outcome: inspect screen/recover; request ' + previous['id'] + ' has no completion receipt')
    mode = native('nvim_get_mode')
    if mode['blocking'] or mode['mode'] not in ('n', 'nt'):
        raise RuntimeError('busy: editor mode ' + mode['mode'] + '; inspect screen and use owned recovery input')
    owner = read(RUNTIME / 'owner.json', {'owner': '', 'epoch': 0})
    envelope = dict(request, id=uuid.uuid4().hex, generation=BINDING['generation'],
                    epoch=owner['epoch'], deadline=time.time() + budget)
    save(RUNTIME / 'receipt.json', {'id': envelope['id'], 'state': 'submitted'})
    try:
        output = native('nvim_exec_lua', ['return Terminal.request(...)', [envelope]], timeout=budget + 0.5)
    except (subprocess.TimeoutExpired, RuntimeError) as error:
        raise RuntimeError('unknown outcome: request ' + envelope['id'] + '; do not retry; ' + str(error))
    result = json.loads(output)
    if not result['ok']:
        raise RuntimeError(result['error'])
    return result.get('value')


def pane_info(pane):
    row = tmux('display-message', '-p', '-t', pane,
               '#{pane_id}\t#{pane_dead}\t#{@terminal_binding}\t#{pane_current_command}').split('\t')
    if len(row) != 4 or row[0] != pane or row[1] != '0':
        raise RuntimeError('associated pane unavailable; inspect rather than replace it')
    return row


def panes():
    result = {}
    for role, pane in BINDING.get('panes', {}).items():
        if pane_info(pane)[2] != BINDING['generation']:
            raise RuntimeError('pane binding changed; refusing adoption')
        result[role] = pane
    return result


def capture(role):
    pane = panes().get(role)
    if not pane:
        raise RuntimeError(role + ' pane unavailable')
    return tmux('capture-pane', '-p', '-t', pane)


def launch(options):
    global BINDING, SOCKET
    if BINDING:
        panes()
        if options.editor_pane or options.socket:
            if options.editor_pane != BINDING['panes']['nvim'] or options.socket != BINDING['socket']:
                raise RuntimeError('existing binding differs; no replacement permitted')
        native('nvim_get_mode')
        if BINDING['active'] != str(ACTIVE):
            remote({'op': 'scope', 'path': str(ACTIVE)})
            BINDING['active'] = str(ACTIVE)
            save(RUNTIME / 'binding.json', BINDING)
    else:
        if bool(options.editor_pane) != bool(options.socket):
            raise RuntimeError('--editor-pane and --socket are required together')
        generation = uuid.uuid4().hex
        SOCKET = Path(options.socket) if options.socket else RUNTIME / 'nvim.sock'
        lua = RUNTIME / 'neovim-control.lua'
        config = {'generation': generation, 'root': str(ACTIVE), 'memos': str(ROOT / '.kern' / 'memos'),
                  'receipt': str(RUNTIME / 'receipt.json')}
        bootstrap = 'vim.g.terminal_config = vim.json.decode(' + json.dumps(json.dumps(config)) + ')\n' + source('neovim-control', 'lua')
        bootstrap += '\nvim.g.terminal_ready = ' + json.dumps(generation) + '\n'
        lua.write_text(bootstrap)
        if options.editor_pane:
            info = pane_info(options.editor_pane)
            if info[2] or info[3] != 'nvim' or options.editor_pane == SOURCE:
                raise RuntimeError('explicit editor must be an unbound nvim pane distinct from source')
            mode = native('nvim_get_mode')
            if mode['blocking'] or mode['mode'] != 'n':
                raise RuntimeError('editor is not idle; inspect before bootstrap')
            native('nvim_exec_lua', ["local s,p,f=...; assert(vim.env.TMUX_PANE==p,'socket pane mismatch'); assert(vim.tbl_contains(vim.fn.serverlist(),s),'socket is not a listener'); dofile(f)",
                                     [str(SOCKET), options.editor_pane, str(lua)]], timeout=2)
            pane = options.editor_pane
        else:
            if SOCKET.exists():
                raise RuntimeError('unbound socket exists; inspect it rather than replace it')
            command = shlex.join(['nvim', '--listen', str(SOCKET), '-c', 'luafile ' + str(lua)])
            pane = tmux('split-window', '-h', '-d', '-P', '-F', '#{pane_id}', '-t', SOURCE, '-c', str(ACTIVE), 'exec ' + command)
        BINDING = {'session': SESSION, 'generation': generation, 'source': SOURCE, 'socket': str(SOCKET),
                   'active': str(ACTIVE), 'panes': {'nvim': pane}}
        tmux('set-option', '-p', '-t', pane, '@terminal_binding', generation)
        tmux('set-option', '-p', '-t', SOURCE, '@terminal_session', SESSION)
        save(RUNTIME / 'binding.json', BINDING)
        save(RUNTIME / 'owner.json', {'owner': '', 'epoch': 0})
        deadline = time.monotonic() + 10
        while not SOCKET.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        if not SOCKET.exists():
            raise RuntimeError('editor opened no socket; inspect its pane; binding retained')
        ready = native('nvim_get_var', ['terminal_ready'], timeout=2)
        if ready != generation:
            raise RuntimeError('editor bootstrap incomplete; inspect its pane; binding retained')
    if options.lazygit and 'lazygit' not in BINDING['panes']:
        directory = Path(run('lazygit', '--print-config-dir'))
        configs = [str(directory / 'config.yml')] if (directory / 'config.yml').exists() else []
        launcher = RUNTIME / 'terminal.py'
        launcher.write_text(source('terminal-control', 'python'))
        callback = shlex.join(['env', 'KERN_TERMINAL_ROOT=' + str(ROOT), 'python3', str(launcher),
                              'nvim', '--session', SESSION, '--source-pane', SOURCE, '--file']) + ' {{filename}}'
        overlay = RUNTIME / 'lazygit.yml'
        overlay.write_text('os:\n  edit: ' + json.dumps(callback) + '\n  editAtLine: ' + json.dumps(callback + ' --line {{line}}')
                           + '\n  editInTerminal: false\n  openDirInEditor: ' + json.dumps(callback) + '\n')
        configs.append(str(overlay))
        command = shlex.join(['lazygit', '--path', str(ACTIVE), '--use-config-file', ','.join(configs)])
        pane = tmux('split-window', '-h', '-b', '-d', '-P', '-F', '#{pane_id}', '-t', SOURCE, '-c', str(ACTIVE), command)
        tmux('set-option', '-p', '-t', pane, '@terminal_binding', BINDING['generation'])
        BINDING['panes']['lazygit'] = pane
        save(RUNTIME / 'binding.json', BINDING)
    return dict(BINDING, runtime=str(RUNTIME))


def context(options):
    global ROOT, ACTIVE, SOURCE, SESSION, RUNTIME, SOCKET, BINDING
    ACTIVE = Path(options.cwd or os.getcwd()).resolve()
    common = Path(run('git', '-C', os.environ.get('KERN_TERMINAL_ROOT', str(ACTIVE)), 'rev-parse', '--path-format=absolute', '--git-common-dir'))
    ROOT = common.parent
    SOURCE = options.source_pane or os.environ.get('KERN_TERMINAL_SOURCE') or os.environ.get('TMUX_PANE', '')
    if not os.environ.get('TMUX') or not SOURCE:
        raise RuntimeError('visible integration unavailable: tmux and an originating pane are required')
    pane_info(SOURCE)
    associated = tmux('show-option', '-pqv', '-t', SOURCE, '@terminal_session')
    SESSION = options.session or os.environ.get('KERN_TERMINAL_SESSION') or associated or uuid.uuid4().hex
    if associated and associated != SESSION:
        raise RuntimeError('source pane belongs to another session; use a new source pane, no adoption permitted')
    server = tmux('display-message', '-p', '-t', SOURCE, '#{socket_path}')
    identity = hashlib.sha256((str(ROOT) + '\0' + server + '\0' + SOURCE).encode()).hexdigest()[:20]
    RUNTIME = Path('/tmp') / ('kern-terminal-' + str(os.getuid()) + '-' + identity)
    RUNTIME.mkdir(mode=0o700, parents=True, exist_ok=True)
    if RUNTIME.is_symlink() or RUNTIME.stat().st_uid != os.getuid() or RUNTIME.stat().st_mode & 0o077:
        raise RuntimeError('runtime must be private and owned by this user')
    BINDING = read(RUNTIME / 'binding.json', {})
    if BINDING and (BINDING['session'] != SESSION or BINDING['source'] != SOURCE):
        raise RuntimeError('binding belongs to another session; no adoption permitted')
    SOCKET = Path(BINDING['socket']) if BINDING else RUNTIME / 'nvim.sock'


def main():
    parser = argparse.ArgumentParser(description='Session native terminal, implemented in memos')
    parser.add_argument('action', choices=['open', 'status', 'screen', 'nvim', 'keys', 'text', 'claim', 'release'])
    parser.add_argument('values', nargs='*')
    for flag in ('owner', 'session', 'source-pane', 'cwd', 'editor-pane', 'socket', 'file'):
        parser.add_argument('--' + flag, default='')
    parser.add_argument('--line', type=int, default=1)
    parser.add_argument('--lazygit', action='store_true')
    parser.add_argument('--recover', action='store_true')
    options = parser.parse_intermixed_args()
    context(options)
    if options.action == 'status':
        return dict(BINDING, owner=read(RUNTIME / 'owner.json', {}), receipt=receipt(), runtime=str(RUNTIME))
    if options.action == 'screen':
        role, = options.values
        return capture(role)
    current = read(RUNTIME / 'owner.json', {'owner': '', 'epoch': 0})
    if options.action in ('keys', 'text'):
        recovery = options.recover and not current['owner'] and options.owner == SESSION and BINDING
        if not recovery and (not options.owner or current['owner'] != options.owner):
            raise RuntimeError('claim the terminal with --owner before interactive operations')
        role, *keys = options.values
        if role not in ('nvim', 'lazygit') or not keys:
            raise RuntimeError('specify nvim or lazygit and keys/text')
        pane = panes().get(role)
        if not pane:
            raise RuntimeError(role + ' pane unavailable')
        tmux('send-keys', '-t', pane, *(['-l'] if options.action == 'text' else []), '--', *keys)
        return capture(role)
    with admission():
        if options.action == 'open':
            return launch(options)
        if not BINDING:
            raise RuntimeError('no editor bound; run just terminal open')
        panes()
        current = read(RUNTIME / 'owner.json', {'owner': '', 'epoch': 0})
        if options.action in ('claim', 'release'):
            if not options.owner:
                raise RuntimeError('--owner is required')
            if current['owner'] and current['owner'] != options.owner:
                raise RuntimeError('terminal held by ' + current['owner'])
            next_owner = {'owner': options.owner if options.action == 'claim' else '', 'epoch': current['epoch'] + 1}
            remote(dict(next_owner, op='hold', next_epoch=next_owner['epoch']))
            save(RUNTIME / 'owner.json', next_owner)
            return next_owner
        payload = options.values[0] if options.values else '{}'
        request = json.loads(payload)
        if not isinstance(request, dict):
            raise ValueError('nvim request must be a JSON object')
        if options.file:
            request = {'op': 'show', 'path': str(Path(options.file).resolve()), 'line': options.line}
        if request.get('op') not in ('query', 'show', 'capabilities', 'state'):
            if not options.owner or current['owner'] != options.owner:
                raise RuntimeError('claim the terminal with --owner before interactive operations')
        request['owner'] = current['owner'] if options.owner == current['owner'] else ''
        return remote(request)


if __name__ == '__main__':
    try:
        result = main()
        print(result if isinstance(result, str) else json.dumps(result, ensure_ascii=False))
    except (RuntimeError, ValueError, OSError, subprocess.TimeoutExpired) as error:
        print(json.dumps({'error': str(error)}), file=sys.stderr)
        sys.exit(1)
```

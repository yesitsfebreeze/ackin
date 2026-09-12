---
kind: routine
description: Both Codex and Claude work visibly through Neovim RPC, installed editor plugins, and lazygit keyboard input with screen readback.
uses:
  - usage: "[[run-usage]]"
    when: [using Neovim or lazygit for any task in Codex, Claude, or another agent]
---

# terminal

## Open and discover

For a task that benefits from editor tools, run `just panes` from its own
originating tmux pane. [[terminal-control]] creates a session-associated editor
to its right or reuses that session's exact association. Use `just terminal`
from the trunk or the session's active lane; never discover another session's
editor by repository, provider name, or PID ordering. Read `just terminal status`
and inspect the editor screen. Without tmux or a distinct session association,
report visible integration unavailable and continue useful source inspection.
A background worker must not silently borrow its parent's editor.

```
just terminal nvim '{"op":"capabilities"}'
just terminal screen nvim
```

Open lazygit explicitly when needed with `just terminal open --lazygit`.
It connects back to this session's editor and preserves other panes' layouts.
The SessionStart hook's tracked source is `scripts/nvim-session.py`; the
installed `~/.local/bin/nvim-socket-brief` serves both existing user-profile
registrations. Install an updated copy only after its startup checks pass.

Capabilities come from the running editor: installed plugins, lazy-load
commands, mappings, and attached servers. Read a plugin's installed help or
source in its reported directory when choosing an unfamiliar operation.
Do not assume a plugin is loaded or install everything in a catalog.
The installed config is used as-is, including Telescope, Gitsigns, Conform,
Oil, Treesitter, completion, and Claude's editor plugin when present.
Lazygit's `?` menu is the authority for its current context and configured
bindings. Its existing global config and custom commands are preserved.

## Ask and show

Read this memo before editing or navigating through the terminal. Use
absolute paths in your working tree. Ask the warm LSP for references,
definitions, hover,
workspace/document symbols, rename impact, and diagnostics before scanning
whole source files. A missing server, timeout, or indexing answer is not
proof of no references: fall back to source inspection and say why.

```
just terminal nvim '{"op":"show","path":"/absolute/lane/src/lib.rs","line":12}'
just terminal nvim '{"op":"query","method":"references","path":"/absolute/lane/src/lib.rs","args":[12,5]}'
just terminal nvim '{"op":"query","method":"document_symbols","path":"/absolute/lane/src/lib.rs"}'
just terminal nvim '{"op":"query","method":"diagnostics","path":"/absolute/lane/src/lib.rs"}'
```

`query` methods are `references`, `definition`, `hover`, `rename_preview`, `workspace_symbols`, `document_symbols`,
`diagnostics`, and `index_status`; their arguments follow the path in [[neovim-control]].
`rename_preview` reports affected locations without applying changes. Queries
show their target only when the designated editor window is safe to follow.
The watcher follows this session's active worktree, not every peer's worktree
or the shared memo checkout. To work in the record, explicitly open with that
checkout as `--cwd`; ordinary source sessions do not follow unrelated memo imports.
Dirty buffers, interactive owners, and overlays
prevent automatic focus changes. neovim-native-control-index indexes native
functions, installed help, file conflicts, and prompt-specific recovery.

## Operate natively

One interactive owner at a time. Claim with a unique owner name, perform a
short interaction, inspect the result, and release even after an error.
A held terminal reports its owner rather than stealing their picker or prompt.
There is no lease timeout that could take a human's active editor away.
If an owner died, establish that its interaction ended before releasing as
that owner. A claim pauses file-following and query-driven window changes.

```
just terminal claim --owner review
just terminal nvim --owner review '{"op":"command","command":"Telescope buffers"}'
just terminal screen nvim
just terminal keys --owner review nvim Escape
just terminal release --owner review
```

Native `command` accepts Ex commands, including plugin commands. Native `lua`
executes Lua in this editor and returns its result, so plugins with Lua APIs
are available without adding wrappers. For example, after opening your file
under a claim, use `return require("gitsigns").get_hunks()` to inspect hunks,
`require("gitsigns").preview_hunk()` to show one, or
`return require("conform").list_formatters(0)` to inspect formatters. Formatting
and buffer edits use the same claim and file ownership as ordinary writes.
Use `nvim` `state` to read the current path, cursor, mode, lines, and modified
flag. Check the changed diff before saving. LSP rename and code actions that
touch files this agent does not own need that ownership widened first.

`keys` sends native tmux key names with mappings; `text` sends literal text.
Both return the visible pane afterward. Inspect again when an action opens an
asynchronous picker or popup. Do not send long blind key sequences.

A busy editor rejects additional normal requests rather than queuing them.
Screen capture and owned recovery keys remain available independently of the
RPC request. A timeout is an unknown outcome, not proof of cancellation; never
retry a mutation automatically. Inspect the mode and screen, identify the
prompt, then send only its intended response. Do not clear an unrelated picker,
auto-accept a Claude diff, or discard unsaved buffers to make progress. Native
RPC expiry can prevent a delayed request from starting, but cannot roll back
arbitrary plugin code that has already run.

External changes reload clean buffers. Conflicting or deleted buffers preserve
text and expose a pending decision. Review the current buffer and disk state
before explicitly choosing keep or reload. Reloading dirty contents needs
permission to discard those edits; a terminal claim is not that permission.

## Operate lazygit

The agent doing the task owns interactive git work. When several agents
share the box, each holds the UI only for as long as its own review takes.
Start with `screen lazygit`, claim, then `keys --owner <name> lazygit '?'`
and read the current bindings. Navigate files, hunks, history, branches,
worktrees, stash, and custom commands with those
bindings. Inspect the screen after each transition. Use its worktrees panel
to enter a lane before inspecting or staging that lane's edits; trunk status
cannot show them. Verify the displayed repository and branch before every
mutation, stage only owned paths or hunks, and verify the result with git.
Opening a file from lazygit goes to the same visible Neovim via its socket.
Git operations invoking a blocking commit-message editor retain the user's
normal editor behavior; Neovim's remote interface has no wait operation.

Use the available tools when they answer the current question: LSP for
semantic navigation, Telescope for search, Gitsigns for hunks/blame, Conform
for formatting, lazygit for visible diff/history/staging/commit review.
A tool need not be exercised gratuitously on every pass. External actions
such as push still follow the user's authorized task. Native UI access does
not widen which files an agent may change, or authorize another agent
conversation.

## Agent entry points

Every agent reaches this procedure through the `terminal` skill, and Claude's
`/terminal` opens it directly. A routine that wants the terminal links to this
memo and passes it to whatever agents it dispatches, rather than restating
the procedure; [[work]] is one such caller and holds no editor rules of its
own. Passing it on is half of it: a caller told to work through the editor
works through it for its own reads too — lane state, gate output, a symbol it
is about to judge — rather than handing the instruction to subagents and
falling back to grep itself. `just panes` and one `capabilities` call are not
the editor being used. Neither host needs a private editor plugin protocol: both call the
native Neovim RPC and the tmux controls through their shell tool.

Neovim's [remote interface](https://neovim.io/doc/user/remote/) and lazygit's
[configuration contract](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md)
define these connections. The executable implementation lives in memos,
not in root Neovim scripts. Supersedes the usage prohibition in
the-editor-is-for-asking-not-for-writing.
each-session-owns-its-editor supersedes the shared-instance policy in
one-editor-on-the-trunk-serves-every-lane. Sockets live in private short
runtime paths, and a wedged editor is reported without killing unsaved work.
[[neovim-control]] and [[terminal-control]] are the executable siblings.

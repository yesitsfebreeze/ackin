# Scope

Scope is the base cartridge's search and editing interface over ASP and disk.
The upper half previews the selected item, the input sits in the center, and
the lower half contains the result list. F4 switches the preview between detail
and the usage waterfall. The agent conversation and editor use the upper half.
Plugins supply data, summaries and detail documents through declared events.

## Launch

Start the project host if it is not already running, then open the browser:

```sh
cartridge daemon
# In another terminal, in the same project:
cartridge scope
```

The binary embeds the Python package. By default `uv` supplies Python 3.11 and
Textual 8.2.8 and RapidFuzz 3.14.3, downloading them on first use. Subsequent launches use its cache.
To use an existing environment, run `cartridge scope --python /path/to/python`.
That Python must be version 3.11 or newer with Textual 8.2.8 and RapidFuzz 3.14.3 installed.
`cartridge scope --once` prints a JSON snapshot without opening the UI.
`cartridge --dir /path/to/project scope` selects another project.

With the optional tmux launcher cartridge enabled,
`cartridge call scope '{"op":"open"}'` opens the same base browser in a new
pane. No separate UI implementation lives in that cartridge.

For source development:

```sh
uv run --project cartridge.ctg/ui cartridge-scope --dir "$PWD"
PYTHONPATH=cartridge.ctg/ui uv run --project cartridge.ctg/ui python -m unittest discover -s cartridge.ctg/ui/tests
```

## Interaction

The center input searches locally indexed metadata and the public ASP search
providers. Type `memo jev`, `memos from jev`, or `show me memos containing jev`.
Memo search includes public memo bodies and metadata such as authors.
`type:memo`, `type:task`, and every declared scheme also work. Several type
filters form a union. Provider search is bounded at 256 hits; expanding an
entity exposes its real relationships, including provider pagination nodes.
The status distinguishes loaded matches from an exhaustive inventory.

Only 300 list rows are materialized: the current 100-item block and its two
neighbors. Stable entity IDs preserve selection while blocks move. Page Up and
Page Down move 100 items; arrows and the wheel move through block boundaries.
Unchanged refreshes reuse projections and cells; searchable text is indexed
once per changed entity. ASP returns bounded ranked results rather than asking
the terminal to scan every provider.

The list groups types by aggregate relevance, then orders items within each
type by ASP search score plus `ln(1 + observed uses)`. The summary column is
provider-owned text. No summary is inferred from the item's type. Unknown
types work immediately through the generic inspector.

The waterfall keeps timestamp order independently of list ranking. Every row
is an observed use, and selecting it highlights the related list item and
shows the occurrence in detail. A point means a timestamped call; a bar requires
an explicit end timestamp. Unknown time stays unknown. The base ASP activity
feed currently publishes call timestamps, not execution durations or payloads.
Opening Scope and fetching detail documents do not create usage observations.

Preview and results split the available height evenly at every width, with the
input and filter-chain bar between them. F4 selects the waterfall preview; F3
enlarges the focused results or preview. Resizing retains selection and the
chain. Colors inherit the terminal palette. Only the focused selection is
inverted; pane backgrounds, fixed columns and footer keys remain unfilled.

| Key | Action |
| --- | --- |
| Ctrl-Space | Switch between list and agent conversation, preserving both. |
| Page Up / Page Down | Move 100 list items. |
| Ctrl-Home / Ctrl-End | First / last list item. |
| Ctrl-F | Focus the shared search field. |
| Tab / Shift-Tab | In the input: complete or chain / remove the current stage. Elsewhere: change focus. |
| Up / Down, Ctrl-K / Ctrl-N | Select results while keeping the input focused. |
| Ctrl-T | Mark or unmark a result; Tab passes marked results. |
| F4 | Switch detail and waterfall in the upper preview. |
| Ctrl-E | Edit the preview in the upper half. |
| Ctrl-S | Save an edit; in a filter chain, toggle case sensitivity. |
| Ctrl-W / Ctrl-X | In the filter input, toggle whole-word / regex grep. |
| F6 | Open the draft with $VISUAL, $EDITOR or micro. |
| Arrow keys | Select a row or field; navigate detail values. |
| Enter | Inspect a list cell, expand a detail value or follow an entity link. |
| Alt-Left | Return from an entity link to the previous item and filter. |
| F2 | Toggle all items and items with observed use. This is not an active-now filter. |
| F3 | Enlarge or restore the focused pane. |
| F5 | Refresh graph, search, usage and selected item. |
| Ctrl-Left / Ctrl-Right | Pan the timeline. |
| Ctrl-Up / Ctrl-Down | Zoom the timeline. |
| Ctrl-L | Follow current time. |
| Ctrl-P | Freeze the displayed context while collection continues. |
| Escape | Restore an enlarged pane, or clear the search. |
| Ctrl-Q | Quit and restore the terminal. |

## Plugin contract

Publish normal ASP nodes and observations. The common list and waterfall have
no plugin-specific branches. The owner of a scheme may declare an attribute
named `<owner>.scope` with the value `{"summary":"Specced · 3/8 checks"}`.
Declare its object schema under `asp.attributes`. This text is displayed in
the Summary column and participates in local search. It is plain data, never
terminal markup or executable code.

For custom details, declare and listen to `scope.detail.<scheme>` in the
plugin's `cartridge.json`. Scope discovers the event through ASP types and
uses it only when its owner is also the scheme owner. The plugin can implement
the listener with a Python helper, or any other language, inside its own
sandbox. Its request is:

```json
{"op":"describe","version":1,"item":{"id":"task:root/example","attributes":{}},"selection":{"field":"summary","observation":null},"viewport":{"width":48,"height":20}}
```

The item includes the available ASP metadata and common list fields. The
selected occurrence, when present, is under `selection.observation`.
The response is a declarative document:

```json
{"version":1,"title":"Example task","sections":[{"label":"State","value":"specced"},{"label":"Progress","value":{"done":3,"total":8}},{"label":"Parent","value":{"$entity":"plan:root"}}]}
```

Values may be JSON scalars, arrays or objects. Scope owns keyboard input,
layout, escaping, lazy tree expansion and navigation. An object with
`$entity` offers a read-only link to another ASP entity; it never executes a
tool. Documents are limited to 64 sections and 256 KiB. A tree branch exposes
up to 1000 immediate children. Detail requests have a two-second deadline;
failure or invalid output shows an error alongside the generic inspector.
Late replies cannot overwrite a newly selected item's detail. These are
read-only rendering events; providers must not use them to mutate state.

The retained context holds up to 10000 entities and 8192 edges. Usage is the
host's retained 1024-record snapshot, replaced on every poll. Provider errors
remain visible without erasing the last successful context.

## Agent conversation

Type `/agent` or press Ctrl-Space to switch the same center input into an
actual host agent session. Enter sends the question. `/list` returns to the
three panes. `/ask memo jev` asks the agent to interpret that search, retrieve
ASP evidence and return the items that fit. Normal filter typing never starts
a model request. `/help` shows the commands.

The host's agent, router and sessions cartridges must be active for conversation.
The browser displays completed assistant messages and the current run step.
It preserves the real session across turns and reads only that session's
transcript. Policy approvals are shown with their tool arguments; `/allow` and
`/deny` answer them explicitly. `/cancel` stops the current turn. Quitting also
requests cancellation of the turn started by this browser.

An assistant may return a fenced `scope` JSON block with a `query` and optional
`entities` array of at most 256 actual ASP IDs. Scope validates this data and
applies it to the list; it never executes model text as a command. Explicit
IDs allow intent matches whose wording differs. Missing entities are reported.
A result cannot replace a filter the user changed while the agent worked.
Editing the filter clears the agent's result selection. A missing agent leaves
ordinary filtering, navigation and plugin details usable.

## Composable finder

The interaction follows [finder.nvim](https://github.com/yesitsfebreeze/finder.nvim):
filters declare accepted and produced result types. Later stages narrow the
previous result set, including results outside the 300 visible rows.

```
Files src > Grep TODO > Fuzzy retry
Dirs cartridge.ctg > Files apppy > Grep class
ASP memo jev > Grep deployment
All configuration > Type memo file
```

`ASP` searches providers and fuzzy-matches loaded entity names. `Files` and
`Dirs` fuzzy-match project paths. `All` merges ASP and file results. `Grep`
searches file contents, or narrows previous content/entity matches. `Fuzzy`
narrows labels and matched line content, `Type` narrows schemes, and `Expand`
loads ASP neighborhoods. Quotes preserve literal `>` characters in a query.
Only compatible filters appear after Tab. Type a filter prefix and Tab to
complete it. Tab after a query captures its result set for the next stage;
select with arrows first to pass one item, or mark several with Ctrl-T.
Shift-Tab removes a stage; Backspace at the start of the input goes back.
Editing an earlier stage invalidates its downstream captured scope.

Path enumeration is cached for ten seconds and honors ignore files, including
outside Git repositories. F5 invalidates it immediately. Ripgrep does content
search asynchronously and is killed and reaped on cancellation. Fuzzy matching
uses RapidFuzz's native subsequence candidate filtering and boundary/contiguity
ranking off the UI thread. Regex, case and word flags are visible in the chain
bar. ASP results remain provider-bounded at 256 remote hits. Expansion is capped
at 256 input entities; disk grep retains at most 10000 matches and reports that
limit. Backend responses are capped at 16 MiB and 15 seconds; exceeding those
limits fails explicitly. Disk previews read at most 128 KiB. No partial preview
is used as the document to save.

## Editing and update events

Ctrl-E opens a complete text document up to 64 KiB in the upper editor. Ctrl-S
saves, and Escape returns to the preview. F6 hands a temporary draft to the
user's `$VISUAL` or `$EDITOR`, falling back to `micro`; this external application
uses the terminal until it exits. Its draft returns to the inline editor, where
Ctrl-S explicitly publishes it. The real file is never handed to an external
editor before the guarded save.

File editing registers a real session, obtains a read observation through
`tool.read`, checks the opening bytes, and saves with `tool.write`. In GitFS
mode it then uses guarded `tool.gitfs materialize` for this path. Those existing
tools publish the normal session change evidence and events. A conflict retains
the draft and does not overwrite the changed file. Scope does not synthesize
an agent completion or duplicate an update event.

A detail document can declare `edit` with a `tool`, `read` and `write` request,
plus optional `read_text_key` (default `text`) and `write_text_key` (default
`body`). The tool must belong to the selected entity's ASP owner. Scope reads
the complete document and passes its revision as `expected_revision` on save.
Memo supplies this descriptor so its frontmatter and body are validated by
`tool.memo`. For an item with no direct edit descriptor, Ctrl-E opens an editable
JSON proposal; saving submits the original and proposed information to the
actual agent, which applies supported changes through the item's owner tools
and reports the outcome. This is an asynchronous change request, not a direct
record overwrite.

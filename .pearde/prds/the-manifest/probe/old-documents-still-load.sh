#!/bin/sh
# Every field the document gained is optional, so a manifest written before the
# capability request existed still reads. The real manifests of ~/dev/sys are
# the population: each is copied into a fixture folder made at run time.
set -eu
LANE=${LANE:-/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest}
BIN="$LANE/target/debug/zirkle"
SRC=${SRC:-$HOME/dev/sys/builtin}
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT

: > "$ROOT/entries"
for m in "$SRC"/*/cartridge.json; do
	name=$(basename "$(dirname "$m")")
	mkdir -p "$ROOT/$name"
	cp "$m" "$ROOT/$name/cartridge.json"
	echo 'return {apply=function(ctx) end}' > "$ROOT/$name/init.lua"
	echo "  {id=\"$name\", path=\"$name\"}," >> "$ROOT/entries"
done
{ echo 'return {'; cat "$ROOT/entries"; echo '}'; } > "$ROOT/init.lua"
rm "$ROOT/entries"

# The fixture deliberately copies no file a manifest merely *names* — no Lua
# entry that works, no `ui:` module. Every document must still read, and `list`
# exits non-zero if one does not, so the count below is checked and not eyeballed.
echo "--- $(ls "$SRC"/*/cartridge.json | wc -l | tr -d ' ') documents copied; list exits 0 only if every document reads ---"
"$BIN" --dir "$ROOT" --profile "$ROOT" list

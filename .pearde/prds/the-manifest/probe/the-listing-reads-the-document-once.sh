#!/bin/sh
# `zirkle list` is the one readout of the document, so what a cartridge asked
# for is listed beside what it provides. Five properties, through the real
# binary: a blank grant path is refused rather than printed empty; a document
# that will not read prints why and no declaration columns and does not exit 0,
# whether its entry is enabled or disabled; a file the document merely *names*
# going missing is none of those things; every declaration prints exactly once
# on one line; and a disabled cartridge still prints everything it declared.
# Fixtures are built in a directory made at run time — never under .pearde/prds.
set -eu
LANE=${LANE:-/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest}
BIN="$LANE/target/debug/zirkle"
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT
INERT='return {apply=function(ctx) end}'

one() { # one <folder> <json>
	mkdir -p "$ROOT/$1"
	printf '%s\n' "$2" > "$ROOT/$1/cartridge.json"
	printf '%s\n' "$INERT" > "$ROOT/$1/init.lua"
}

listed() { # listed -> sets OUT and RC, never exits
	set +e
	OUT=$("$BIN" --dir "$ROOT" --profile "$ROOT" list 2>&1)
	RC=$?
	set -e
	printf '%s\nexit %s\n' "$OUT" "$RC"
}

count() { # count <needle> -> occurrences in OUT
	printf '%s\n' "$OUT" | grep -o "$1" | wc -l | tr -d ' '
}

echo "--- 1. a blank grant path is refused, not printed as an empty column ---"
one blank '{"name":"blank","entry":"init.lua","grant":{"read":["   "]}}'
echo 'return {{id="blank", path="blank"}}' > "$ROOT/init.lua"
listed
[ "$(count 'must be a nonempty exact path')" -eq 1 ] || { echo "FAIL: no validation message"; exit 1; }
[ "$(count 'reads')" -eq 0 ] || { echo "FAIL: printed a grant column anyway"; exit 1; }
[ "$RC" -ne 0 ] || { echo "FAIL: exited 0 on a document that will not read"; exit 1; }
rm -rf "${ROOT:?}/blank"

echo "--- 2. a document that will not read declares nothing knowable, enabled or not ---"
one bad '{"name":"bad","entry":"init.lua","export":["nobody.provides.this"],"grant":{"net":["api.example.com"]}}'
echo 'return {{id="bad", path="bad"}, {id="off", path="bad", disabled=true}}' > "$ROOT/init.lua"
listed
# Once per entry, and the disabled one is not exempt: a disabled cartridge has
# still declared, so a document it cannot declare through is still worth saying.
[ "$(count 'nothing inside this cartridge offers it')" -eq 2 ] || { echo "FAIL: not said once for each entry"; exit 1; }
[ "$(count '(disabled)  error')" -eq 1 ] || { echo "FAIL: the disabled entry exited 1 saying nothing"; exit 1; }
[ "$(count 'exports')" -eq 0 ] || { echo "FAIL: an unreadable document listed a re-export"; exit 1; }
[ "$(count 'net api')" -eq 0 ] || { echo "FAIL: an unreadable document listed a grant"; exit 1; }
[ "$RC" -ne 0 ] || { echo "FAIL: exited 0 on a document that will not read"; exit 1; }
rm -rf "${ROOT:?}/bad"

echo "--- 3. a missing file the document only names is not a document that will not read ---"
one ui '{"name":"ui","entry":"init.lua","ui":"ui/index.ts","provide":["ui.key"],"grant":{"read":["data"]}}'
echo 'return {{id="ui", path="ui"}}' > "$ROOT/init.lua"
listed
[ "$(count 'reads data')" -eq 1 ] || { echo "FAIL: a missing ui file blanked the declarations"; exit 1; }
[ "$(count 'index.ts')" -eq 1 ] || { echo "FAIL: the missing file was not reported"; exit 1; }
[ "$RC" -eq 0 ] || { echo "FAIL: exited $RC on a document that read perfectly well"; exit 1; }
rm -rf "${ROOT:?}/ui"

echo "--- 4. every declaration is printed, once, on one line ---"
one on '{"name":"on","entry":"init.lua","provide":["on.key"],"export":["in.key"],"grant":{"read":["data"],"write":["cache"],"net":["api.example.com"],"exec":["rg"]}}'
one on/in '{"name":"in","entry":"init.lua","provide":["in.key"]}'
# Enabled, because `provides` is filled from the evaluated component: it is the
# one column a disabled entry cannot show, so section 5 alone cannot prove this.
printf '%s\n' 'return {apply=function(ctx) ctx:provide("on.key", function() return 1 end) end}' > "$ROOT/on/init.lua"
echo 'return {{id="on", path="on"}}' > "$ROOT/init.lua"
listed
[ "$RC" -eq 0 ] || { echo "FAIL: exited $RC"; exit 1; }
[ "$(printf '%s\n' "$OUT" | wc -l | tr -d ' ')" -eq 1 ] || { echo "FAIL: more than one line"; exit 1; }
for column in 'provides on.key' 'exports in.key' 'reads data' 'writes cache' 'net api.example.com' 'execs rg'; do
	[ "$(count "$column")" -eq 1 ] || { echo "FAIL: '$column' is printed $(count "$column") times"; exit 1; }
done

echo "--- 5. a disabled cartridge still declares what it asked for ---"
echo 'return {{id="off", path="on", disabled=true}}' > "$ROOT/init.lua"
listed
[ "$RC" -eq 0 ] || { echo "FAIL: exited $RC"; exit 1; }
for column in '(disabled)' 'exports in.key' 'reads data' 'writes cache' 'net api.example.com' 'execs rg'; do
	[ "$(count "$column")" -eq 1 ] || { echo "FAIL: '$column' is printed $(count "$column") times"; exit 1; }
done

echo "--- all five hold ---"

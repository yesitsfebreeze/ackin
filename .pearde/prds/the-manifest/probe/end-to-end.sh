#!/bin/sh
# The document, read by the real binary rather than by a unit test: a nested
# tree with a re-export and a capability request, listed through `zirkle list`.
# Fixtures are built in a directory made at run time — never under .pearde/prds.
set -eu
LANE=${LANE:-/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest}
BIN="$LANE/target/debug/zirkle"
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT

mkdir -p "$ROOT/outer/inner"
cat > "$ROOT/outer/inner/cartridge.json" <<'J'
{"name":"inner","entry":"init.lua","provide":["inner.store"],
 "grant":{"read":["data"],"net":["api.example.com"]}}
J
echo 'return {apply=function(ctx) ctx:provide("inner.store", function() return 1 end) end}' > "$ROOT/outer/inner/init.lua"

cat > "$ROOT/outer/cartridge.json" <<'J'
{"name":"outer","entry":"init.lua","needs":["inner.store"],"export":["inner.store"],
 "grant":{"write":["cache"],"exec":["rg"]}}
J
echo 'return {apply=function(ctx) end}' > "$ROOT/outer/init.lua"

# A sibling of the whole tree providing the same key: no collision, because
# neither subtree passes it into the other.
mkdir -p "$ROOT/other"
cat > "$ROOT/other/cartridge.json" <<'J'
{"name":"other","entry":"init.lua","provide":["inner.store"]}
J
echo 'return {apply=function(ctx) ctx:provide("inner.store", function() return 2 end) end}' > "$ROOT/other/init.lua"

cat > "$ROOT/init.lua" <<'J'
return {{id="outer", path="outer"}, {id="other", path="other"}}
J

echo "--- list ---"
"$BIN" --dir "$ROOT" --profile "$ROOT" list

echo "--- the same document with a re-export nothing offers ---"
BAD=$(mktemp -d)
mkdir -p "$BAD/lone"
cat > "$BAD/lone/cartridge.json" <<'J'
{"name":"lone","entry":"init.lua","export":["nobody.key"]}
J
echo 'return {apply=function(ctx) end}' > "$BAD/lone/init.lua"
echo 'return {{id="lone", path="lone"}}' > "$BAD/init.lua"
"$BIN" --dir "$BAD" --profile "$BAD" list || true
rm -rf "$BAD"

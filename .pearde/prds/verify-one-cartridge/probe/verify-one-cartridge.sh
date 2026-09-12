#!/bin/sh
# verify-one-cartridge, pass one: `zirkle verify <cartridge>` against a real
# tree, no profile anywhere. Fixture is made at run time in mktemp -d, never
# under .pearde/prds/.
set -u
BIN="$(cd "$(dirname "$0")" && pwd)/../../../.lanes/verify-one-cartridge/target/debug/zirkle"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ROOT="$WORK/tree"
mkdir -p "$ROOT/store" "$ROOT/log" "$ROOT/broken" "$ROOT/store/inner"

# A provider whose document declares what its Lua keeps providing.
cat > "$ROOT/log/cartridge.json" <<'EOF'
{"name": "log", "entry": "init.lua", "provide": ["log.write"]}
EOF
cat > "$ROOT/log/init.lua" <<'EOF'
return { provide = {"log.write"}, apply = function(ctx)
	ctx:provide("log.write", function() return "logged" end)
end }
EOF

# The cartridge just written: one selftest, one integration that reaches the
# real dependency, one need the ledger must resolve outward.
cat > "$ROOT/store/cartridge.json" <<'EOF'
{"name": "store", "entry": "init.lua", "provide": ["store.get", "store.check", "store.wire"],
 "needs": ["log.write"], "selftest": "store.check",
 "integration": "store.wire"}
EOF
cat > "$ROOT/store/init.lua" <<'EOF'
return { provide = {"store.check", "store.wire"}, apply = function(ctx)
	ctx:provide("store.check", function() return true end)
	ctx:provide("store.wire", function() return ctx["log.write"]() == "logged" end)
end }
EOF

# A neighbour that must stay out of the run: it declares a contract that would
# fail, and the one-cartridge ask never grades it.
cat > "$ROOT/broken/cartridge.json" <<'EOF'
{"name": "broken", "entry": "init.lua", "selftest": "broken.check"}
EOF
cat > "$ROOT/broken/init.lua" <<'EOF'
return { provide = {"broken.check"}, apply = function(ctx)
	ctx:provide("broken.check", function() return false end)
end }
EOF

echo "== 1. verify the one cartridge, no profile anywhere"
"$BIN" --dir "$ROOT" verify store; echo "exit: $?"

echo "== 2. the failing neighbour is kept out of the one-cartridge run"
"$BIN" --dir "$ROOT" verify store >/dev/null 2>&1; echo "exit: $?"

echo "== 3. the same tree with a broken contract: the run names it and fails"
mv "$ROOT/store/init.lua" "$ROOT/store/init.lua.bak"
cat > "$ROOT/store/init.lua" <<'EOF'
return { provide = {"store.check", "store.wire"}, apply = function(ctx)
	ctx:provide("store.check", function() return false end)
	ctx:provide("store.wire", function() return true end)
end }
EOF
"$BIN" --dir "$ROOT" verify store; echo "exit: $?"
mv "$ROOT/store/init.lua.bak" "$ROOT/store/init.lua"

echo "== 4. a need nothing offers is named before anything loads"
python3 - "$ROOT" <<'EOF'
import json, sys
root = sys.argv[1]
p = root + "/store/cartridge.json"
doc = json.load(open(p))
doc["needs"] = ["absent.key"]
open(p, "w").write(json.dumps(doc))
EOF
"$BIN" --dir "$ROOT" verify store; echo "exit: $?"
python3 - "$ROOT" <<'EOF'
import json, sys
root = sys.argv[1]
p = root + "/store/cartridge.json"
doc = json.load(open(p))
doc["needs"] = ["log.write"]
open(p, "w").write(json.dumps(doc))
EOF

echo "== 5. a name that is not a cartridge is an error, not a run"
"$BIN" --dir "$ROOT" verify absent; echo "exit: $?"

echo "== 6. every contract of the whole profile still behaves as before"
"$BIN" --dir "$ROOT" verify; echo "exit: $?"

echo "== 7. the ledger still lists the tree"
"$BIN" --dir "$ROOT" ledger; echo "exit: $?"
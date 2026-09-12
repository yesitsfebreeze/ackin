#!/bin/sh
# spec01 box: an ambiguous need is named and the run never loads either side.
set -u
W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT
R="$W/tree"
BIN=/Users/feb/dev/cartridge/.pearde/.lanes/verify-one-cartridge/target/debug/zirkle
mkdir -p "$R/store" "$R/provider" "$R/twin"

cat > "$R/provider/cartridge.json" <<'EOF'
{"name": "provider", "entry": "init.lua", "provide": ["provider"]}
EOF
cat > "$R/provider/init.lua" <<'EOF'
return { provide = {"provider"}, apply = function(ctx)
	ctx:provide("provider", function() return 1 end)
end }
EOF

cat > "$R/twin/cartridge.json" <<'EOF'
{"name": "twin", "entry": "init.lua", "provide": ["provider"]}
EOF
cat > "$R/twin/init.lua" <<'EOF'
return { provide = {"provider"}, apply = function(ctx)
	ctx:provide("provider", function() return 2 end)
end }
EOF

cat > "$R/store/cartridge.json" <<'EOF'
{"name": "store", "entry": "init.lua", "provide": [], "needs": ["provider"],
 "selftest": "store.check", "integration": "store.check"}
EOF
cat > "$R/store/init.lua" <<'EOF'
return { provide = {"store.check"}, apply = function(ctx)
	ctx:provide("store.check", function() return true end)
end }
EOF

"$BIN" --dir "$R" verify store
echo "exit: $?"
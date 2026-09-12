#!/bin/sh
# Write, wire, run — the Lua author surface end-to-end, over real processes.
#
# Builds the lane's binary, then writes a two-cartridge chain with nothing but
# files: three folders, three cartridge.json documents, three init.lua
# entries. One ask brings the chain up, and the probe calls through the top
# node's socket — a need answered one node down, a cartridge the tool composed
# itself answering through the same socket, and a dependency's death taking
# its dependent along.
#
# Run from anywhere:  sh .pearde/prds/lua-interface/probe/write-wire-run.sh
set -eu

LANE="${ZIRKLE_LANE:-/Users/feb/dev/cartridge/.pearde/.lanes/lua-interface}"
cd "$LANE"

echo "== build"
cargo build -q -p zirkle 2>&1 | tail -1 || true
ZIRKLE="$LANE/target/debug/zirkle"

ROOT="$(mktemp -d)"
cleanup() {
	for f in "$ROOT"/.nodes/*; do
		[ -f "$f" ] && kill -9 "$(cat "$f")" 2>/dev/null || true
	done
	rm -rf "$ROOT"
}
trap cleanup EXIT
echo "root: $ROOT"

echo "== write: six files, no Rust, no profile"
mkdir -p "$ROOT/echo" "$ROOT/tool/child"

cat > "$ROOT/echo/cartridge.json" <<'EOF'
{"name":"echo","entry":"init.lua","provide":["echo"]}
EOF
cat > "$ROOT/echo/init.lua" <<'EOF'
return {apply=function(ctx)
	ctx:provide("echo", function(args)
		return {said = args.word, from = "echo"}
	end)
end}
EOF

cat > "$ROOT/tool/cartridge.json" <<'EOF'
{"name":"tool","entry":"init.lua","provide":["tool.hello"],"needs":["echo"],"config":{"via":"document"}}
EOF
cat > "$ROOT/tool/init.lua" <<'EOF'
return {apply=function(ctx, config)
	local echo = ctx:get("echo")
	ctx:cartridge("tool/child", {})
	ctx:provide("tool.hello", function(args)
		local r = echo({word = args.word})
		r.via = config.via
		return r
	end)
end}
EOF

cat > "$ROOT/tool/child/cartridge.json" <<'EOF'
{"name":"child","entry":"init.lua","provide":["child.note"],"needs":["tool.hello"]}
EOF
cat > "$ROOT/tool/child/init.lua" <<'EOF'
return {apply=function(ctx)
	local hello = ctx:get("tool.hello")
	ctx:provide("child.note", function(args)
		local r = hello(args)
		r.via = "child"
		return r
	end)
end}
EOF

echo "== wire: the ledger reads the tree"
"$ZIRKLE" ledger --dir "$ROOT"

echo "== run: one ask, the chain up from the bottom"
mkdir -p "$ROOT/.nodes"
OUT="$(ZIRKLE_NODES="$ROOT/.nodes" "$ZIRKLE" up tool.hello --dir "$ROOT")"
printf '%s\n' "$OUT"
SOCK="$(printf '%s\n' "$OUT" | python3 -c "
import sys, json
for line in sys.stdin:
    line = line.strip()
    if line.startswith('{'):
        print(json.loads(line)['socket'])
")"
sleep 2
for f in "$ROOT"/.nodes/*; do
	echo "node $(basename "$f") pid $(cat "$f")"
done

call() {
	python3 -c "
import json, socket, sys
path, request = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(path)
s.sendall(request.encode() + b'\n')
buf = b''
while True:
    chunk = s.recv(65536)
    if not chunk:
        break
    buf += chunk
    for line in buf.decode().splitlines():
        m = json.loads(line)
        if 'reply' in m or 'error' in m:
            print(json.dumps(m))
            sys.exit(0)
" "$1" "$2"
}

echo "== call tool.hello: the tool's handler calls its need, one node down"
call "$SOCK" '{"call":"tool.hello","args":{"word":"hello"},"id":1}'
echo "== call child.note: the composed cartridge answers, reaching up through the tool"
call "$SOCK" '{"call":"child.note","args":{"word":"nested"},"id":1}'

echo "== call nowhere: a key nothing provides is refused by name"
call "$SOCK" '{"call":"nowhere","args":null,"id":1}'

echo "== kill the dependency: the dependent follows"
kill -9 "$(cat "$ROOT/.nodes/echo")"
sleep 2
TOOL_PID="$(cat "$ROOT/.nodes/tool")"
if kill -0 "$TOOL_PID" 2>/dev/null; then
	echo "FAIL: the tool outlived its dependency"
	exit 1
fi
echo "the tool went with its dependency (pid $TOOL_PID gone)"
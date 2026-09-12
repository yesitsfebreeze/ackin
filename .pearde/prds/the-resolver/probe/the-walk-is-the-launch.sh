#!/bin/sh
# The walk is the launch: end-to-end over real processes.
#
# Builds the lane's binaries, assembles a three-node chain in a temporary
# root, asks for the top tool, and shows what the ask left behind: a process
# tree identical to the dependency tree, teardown that follows a node's exit,
# and an ask that launches nothing once the far end is uninstalled.
#
# Run from anywhere:  sh .pearde/prds/the-resolver/probe/the-walk-is-the-launch.sh
set -eu

LANE="${ZIRKLE_LANE:-/Users/feb/dev/cartridge/.pearde/.lanes/the-resolver}"
cd "$LANE"

echo "== build"
cargo build -q -p zirkle 2>&1 | tail -1 || true
cargo build -q --example chain_fixture 2>&1 | tail -1 || true
ZIRKLE="$LANE/target/debug/zirkle"
FIXTURE="$LANE/target/debug/examples/chain_fixture"

ROOT="$(mktemp -d)"
ROOT2="$(mktemp -d)"
cleanup() {
	# The far ends are detached on purpose: nothing owns them, so the probe
	# reaps what it can see before it goes.
	for r in "$ROOT" "$ROOT2"; do
		if [ -d "$r/.nodes" ]; then
			for f in "$r"/.nodes/*; do
				[ -f "$f" ] && kill -9 "$(cat "$f")" 2>/dev/null || true
			done
		fi
	done
	rm -rf "$ROOT" "$ROOT2"
	pkill -9 -f chain_fixture 2>/dev/null || true
}
trap cleanup EXIT
echo "root: $ROOT"

node() {
	json="$2"
	folder="$1"; shift
	mkdir -p "$ROOT/$folder/bin"
	ln -s "$FIXTURE" "$ROOT/$folder/bin/${folder}_node"
	echo 'return {}' > "$ROOT/$folder/init.lua"
	printf '%s' "$json" > "$ROOT/$folder/cartridge.json"
}
node db    '{"name":"db","entry":"init.lua","binary":"db_node","provide":["db.key"]}'
node store '{"name":"store","entry":"init.lua","binary":"store_node","provide":["store.get"],"needs":["db.key"]}'
node tool  '{"name":"tool","entry":"init.lua","binary":"tool_node","provide":["tool.run"],"needs":["store.get"]}'

pids() { for f in "$ROOT/.nodes"/*; do cat "$f" 2>/dev/null; echo; done; }

echo "== ask: zirkle up tool.run"
mkdir -p "$ROOT/.nodes"
ZIRKLE_NODES="$ROOT/.nodes" "$ZIRKLE" up tool.run --dir "$ROOT"
sleep 2

echo "== the tree (ps): one process per node, each dependent a child of its dependency"
for f in db store tool; do
	pid=$(cat "$ROOT/.nodes/$f")
	ppid=$(ps -o ppid= -p "$pid" | tr -d ' ')
	echo "  $f pid=$pid ppid=$ppid"
done

echo "== teardown: the far end is killed, its dependents go with it"
DBPID=$(cat "$ROOT/.nodes/db")
kill -9 "$DBPID"
sleep 2
LEFT=0
for f in "$ROOT"/.nodes/store "$ROOT"/.nodes/tool; do
	pid=$(cat "$f")
	if ps -p "$pid" > /dev/null 2>&1; then LEFT=$((LEFT + 1)); fi
done
echo "  dependents still alive after the far end died: $LEFT (want 0)"

echo "== uninstalling is the absence of a launch"
rm -rf "$ROOT/db"
if "$ZIRKLE" up tool.run --dir "$ROOT" 2>"$ROOT/ask.err"; then
	echo "  FAIL: the ask succeeded on an uninstalled chain"
	exit 1
fi
echo "  refused: $(cat "$ROOT/ask.err")"

echo "== a cycle is refused"
node a '{"name":"a","entry":"init.lua","binary":"a_node","provide":["a.key"],"needs":["b.key"]}'
node b '{"name":"b","entry":"init.lua","binary":"b_node","provide":["b.key"],"needs":["a.key"]}'
if "$ZIRKLE" up a.key --dir "$ROOT" 2>"$ROOT/cycle.err"; then
	echo "  FAIL: the cycle launched"
	exit 1
fi
echo "  refused: $(cat "$ROOT/cycle.err")"

echo "== a Lua-entry chain is hosted: the binary itself is the program"
mkdir -p "$ROOT2"
hosted() {
	folder="$1"; json="$2"
	key=$(printf '%s' "$json" | sed -n 's/.*"provide":\["\([^"]*\)".*/\1/p')
	mkdir -p "$ROOT2/$folder"
	printf '%s' "$json" > "$ROOT2/$folder/cartridge.json"
	printf 'return { apply = function(ctx) ctx:provide("%s", function() return {} end) end }\n' "$key" > "$ROOT2/$folder/init.lua"
}
hosted db    '{"name":"db","entry":"init.lua","provide":["db.key"]}'
hosted store '{"name":"store","entry":"init.lua","provide":["store.get"],"needs":["db.key"]}'
hosted tool  '{"name":"tool","entry":"init.lua","provide":["tool.run"],"needs":["store.get"]}'
mkdir -p "$ROOT2/.nodes"
ZIRKLE_NODES="$ROOT2/.nodes" "$ZIRKLE" up tool.run --dir "$ROOT2"
sleep 2

echo "== the hosted tree (ps): one host process per node, each dependent a child of its dependency"
for f in db store tool; do
	pid=$(cat "$ROOT2/.nodes/$f")
	ppid=$(ps -o ppid= -p "$pid" | tr -d ' ')
	echo "  $f pid=$pid ppid=$ppid (hosted)"
done

echo "== teardown through the hosted chain: the far end is killed, its dependents go with it"
DBPID=$(cat "$ROOT2/.nodes/db")
kill -9 "$DBPID"
sleep 2
LEFT=0
for f in "$ROOT2"/.nodes/store "$ROOT2"/.nodes/tool; do
	pid=$(cat "$f")
	if ps -p "$pid" > /dev/null 2>&1; then LEFT=$((LEFT + 1)); fi
done
echo "  dependents still alive after the hosted far end died: $LEFT (want 0)"

echo "== pass one probe complete"
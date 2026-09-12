#!/bin/sh
# End-to-end probe for the-telemetry-channels, run from the lane root:
#   cd .pearde/.lanes/the-telemetry-channels
#   sh ../../prds/the-telemetry-channels/probe/end-to-end.sh
# Builds the real binary in the lane, starts a daemon over a throwaway
# profile, publishes from the CLI and through a Lua cartridge, follows the
# channel as a second client, and prints what it received.
set -e
LANE=$(cd "$(dirname "$0")/../../../.lanes/the-telemetry-channels" && pwd)
[ -f "$LANE/Cargo.toml" ] || { echo "lane not found at $LANE"; exit 1; }
cd "$LANE"
# A probe-dedicated target dir: a build started under the prds tree can walk up
# to another Cargo.toml and would otherwise poison a shared target dir with a
# foreign binary.
export CARGO_TARGET_DIR=/tmp/zirkle-telemetry-probe-target
cargo build --offline --bin zirkle >/dev/null
BIN=/tmp/zirkle-telemetry-probe-target/debug/zirkle
[ -x "$BIN" ] || { echo "no zirkle binary built"; exit 1; }
"$BIN" --help | grep -q follow || { echo "stale binary without follow"; exit 1; }

PROBE=$(mktemp -d /tmp/zirkle-telemetry-probe.XXXXXX)
cat > "$PROBE/builder.lua" <<'LUA'
return { apply = function(ctx)
	ctx:on("build", function(d) ctx:publish("build", { from = "builder", step = d }) end)
end }
LUA
cat > "$PROBE/init.lua" <<'LUA'
return { { id = "builder", path = "builder.lua" } }
LUA

"$BIN" --profile "$PROBE" --dir "$PROBE" daemon 2>"$PROBE/daemon.err" &
DAEMON=$!
sleep 2

echo "== follow (a second client) joins, then two publishes arrive =="
"$BIN" --profile "$PROBE" follow build > "$PROBE/follow.out" 2>&1 &
FOLLOW=$!
sleep 1
"$BIN" --profile "$PROBE" publish build '{"from":"cli","step":1}'
sleep 1
"$BIN" --profile "$PROBE" send build 2
sleep 1
kill $FOLLOW 2>/dev/null || true
sleep 1
kill "$DAEMON" 2>/dev/null || true
wait "$FOLLOW" 2>/dev/null || true
wait "$DAEMON" 2>/dev/null || true

cat "$PROBE/follow.out"
rm -rf "$PROBE"
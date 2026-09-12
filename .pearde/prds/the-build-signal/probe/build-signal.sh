#!/bin/sh
# The build signal: is a freshly linked Mach-O on this host able to reach main,
# and does the crate build, link, run and test through it?
#
# Every stage is bounded. A stage that hangs is the fault of
# [[a-pegged-syspolicyd-is-a-machine-fault-that-looks-exactly-li]] and prints
# HUNG rather than waiting, so this script never becomes the thing it measures.
#
# Usage: sh build-signal.sh [crate-dir]   (default: the cartridge repo root)

crate=${1:-/Users/feb/dev/cartridge}
fail=0
cap() { perl -e 'alarm shift @ARGV; exec(@ARGV)' "$@"; }
say() { printf '%-22s %s\n' "$1" "$2"; }

# 1. dyld — a binary linked seconds ago must reach main.
d=$(mktemp -d)
printf 'fn main(){println!("hi");}\n' > "$d/h.rs"
if rustc -o "$d/h" "$d/h.rs" 2>/dev/null; then
  out=$(cap 20 "$d/h" 2>&1); rc=$?
  [ "$rc" = 0 ] && [ "$out" = hi ] && say dyld OK || { say dyld "HUNG rc=$rc"; fail=1; }
else
  say dyld "rustc failed"; fail=1
fi
rm -rf "$d"

# 2. proc-macro and build.rs — what actually stalled, not rustc itself.
if cap 900 cargo build --manifest-path "$crate/Cargo.toml" >/tmp/bs-build.$$ 2>&1; then
  say cargo-build OK
else
  say cargo-build "FAILED rc=$? (tail: $(tail -1 /tmp/bs-build.$$))"; fail=1
fi
rm -f /tmp/bs-build.$$

# 3. the freshly linked crate binary itself reaches main.
bin="$crate/target/debug/zirkle"
if [ -x "$bin" ] && cap 25 "$bin" --help >/dev/null 2>&1; then
  say zirkle-help OK
else
  say zirkle-help "HUNG or missing"; fail=1
fi

# 4. the test binaries link and run.
if cap 900 cargo test --manifest-path "$crate/Cargo.toml" >/tmp/bs-test.$$ 2>&1; then
  # Count from the per-binary summary lines, not from the individual
  # "test ... ok" lines: libtest writes those from several threads and they
  # interleave, which silently undercounts (seen as 52 on a 53-test run).
  n=$(sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' /tmp/bs-test.$$ |
      awk '{t+=$1} END{print t+0}')
  if [ "$n" -gt 0 ]; then say cargo-test "OK ($n passed)"
  else say cargo-test "FAILED 0 passed"; fail=1; fi
else
  say cargo-test "FAILED rc=$?"; fail=1
fi
rm -f /tmp/bs-test.$$

[ "$fail" = 0 ] && echo "signal=live" || echo "signal=dead"
exit $fail

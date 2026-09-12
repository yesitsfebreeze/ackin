#!/bin/sh
bin=/Users/feb/dev/cartridge/.pearde/.lanes/the-host-binary/target/debug/zirkle
d=$(mktemp -d); cd "$d" || exit 1
perl -e 'alarm 30; exec(@ARGV)' "$bin" daemon >daemon.log 2>&1 &
pid=$!
i=0; while [ $i -lt 60 ]; do grep -q serving daemon.log 2>/dev/null && break; i=$((i+1)); perl -e 'select(undef,undef,undef,0.1)'; done
echo "=== status ==="; perl -e 'alarm 15; exec(@ARGV)' "$bin" status 2>&1 | head -20
echo "=== list ==="; perl -e 'alarm 15; exec(@ARGV)' "$bin" list 2>&1 | head -20
echo "=== daemon.log ==="; cat daemon.log
kill $pid 2>/dev/null

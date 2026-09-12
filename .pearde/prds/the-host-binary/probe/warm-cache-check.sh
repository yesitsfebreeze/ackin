#!/bin/sh
# Is there a warm cargo target dir worth borrowing? Run BEFORE trying the
# CARGO_TARGET_DIR workaround -- it is an hour of work if the answer is no.
# A proc-macro dylib created during the peg has no cached verdict and will
# hang on dlopen no matter how you link it into place.
T=${1:-$HOME/dev/sys/target}
PEG=${2:-$(ps -eo lstart,command | awk '/syspolicyd/ && !/awk/ {print $2" "$3" "$4}')}
echo "target: $T"
echo "peg began around: $PEG"
n=$(find "$T/debug/deps" -name '*.dylib' 2>/dev/null | wc -l | tr -d ' ')
echo "proc-macro dylibs present: $n"
echo "  -> if every one postdates the peg, this route is closed."
find "$T/debug/deps" -name '*.dylib' -exec stat -f '%Sm %N' -t '%Y-%m-%d %H:%M' {} \; 2>/dev/null \
  | awk '{print $1}' | sort | uniq -c

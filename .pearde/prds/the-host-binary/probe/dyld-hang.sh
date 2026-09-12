#!/bin/sh
# A freshly linked binary that never reaches main => syspolicyd is pegged.
d=$(mktemp -d)
printf 'fn main(){println!("hi");}\n' > "$d/h.rs"
rustc -o "$d/h" "$d/h.rs" || exit 1
perl -e 'alarm 20; exec($ARGV[0])' "$d/h"
echo "exit=$?  (0 healthy, 142 = hung in dyld)"

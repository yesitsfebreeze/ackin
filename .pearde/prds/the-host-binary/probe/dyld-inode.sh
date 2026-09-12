#!/bin/sh
# Why a pegged syspolicyd hangs some binaries and not others.
# The dyld code-signing verdict is cached per INODE, not per path.
# Subject must be a binary that ran successfully BEFORE the peg began,
# or it has no cached verdict and "refutes" the rule for the wrong reason.
K=${1:-$HOME/.cargo/bin/kache}
[ -x "$K" ] || { echo "subject $K not executable; pass one as \$1"; exit 1; }
d=$(mktemp -d)
run() { perl -e 'alarm 15; exec(@ARGV)' "$1" --version >/dev/null 2>&1; echo "$2 exit=$?"; }
run "$K"                                  "original path+inode :"
ln "$K" "$d/hard" 2>/dev/null && run "$d/hard" "hardlink same inode:"
cp -c "$K" "$d/copy" 2>/dev/null || cp "$K" "$d/copy"
run "$d/copy"                             "copy    new inode  :"
echo "(0 = reached main, 142 = hung in dyld)"
echo "healthy machine: all three 0.  pegged: original/hardlink 0, copy 142."

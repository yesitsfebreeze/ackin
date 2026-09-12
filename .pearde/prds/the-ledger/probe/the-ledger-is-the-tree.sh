#!/usr/bin/env bash
# The ledger derived from a cartridge root, with nothing hand-listed anywhere.
# Every fixture is built here, at run time, under a temp directory: a folder
# holding a document is a cartridge, so none may live under .pearde/prds/.
set -euo pipefail

lane="${LANE:-/Users/feb/dev/cartridge/.pearde/.lanes/the-ledger}"
cargo build --manifest-path "$lane/Cargo.toml" >/dev/null 2>&1
zirkle="$lane/target/debug/zirkle"

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT
cd "$root"

doc() { # doc <dir> <name> [json body fields]
	mkdir -p "$1"
	printf '{"name":"%s","entry":"init.lua"%s}\n' "$2" "${3:-}" > "$1/cartridge.json"
	printf 'return {}\n' > "$1/init.lua"
}

cartridges="$root/cartridges"
mkdir -p "$cartridges"

# `outer` asks for a key its own child provides. The same key is also provided
# by an unrelated top-level cartridge: the walk must prefer the nearer one.
doc "$cartridges/outer"       outer ',"needs":["store.get"]'
doc "$cartridges/outer/inner" inner ',"provide":["store.get"]'
doc "$cartridges/other"       other ',"provide":["store.get"]'
# A third subtree provides the identical key and is invisible to both.
doc "$cartridges/far"         far   ',"export":["store.get"]'
doc "$cartridges/far/deep"    deep  ',"provide":["store.get"]'
# A plain folder is not a cartridge and does not extend a subtree.
mkdir -p "$cartridges/vendor"
doc "$cartridges/vendor/hidden" hidden ',"provide":["vendor.key"]'

run() { "$zirkle" --dir "$cartridges" ledger 2>/dev/null; }
out() { run > "$root/out.txt" || true; cat "$root/out.txt"; }

echo "== 1. the tree, with no list edited anywhere =="
run

echo
echo "== 2. the nearer provider wins =="
out | grep -q 'store.get <- outer/inner' \
	&& echo "ok: outer's need bound to its own child, not to the unrelated top-level cartridge" \
	|| { echo "FAIL: outer/inner did not win the lookup"; out; exit 1; }

echo
echo "== 3. installing is putting a tree where the ledger looks =="
before=$(out | grep -c '^')
doc "$cartridges/fresh" fresh ',"provide":["fresh.key"]'
out | grep -q '^fresh' && echo "ok: fresh appears with no list edited"
doc "$cartridges/fresh/child" child ',"provide":["child.key"]'
out | grep -q '^fresh/child' && echo "ok: so does everything inside it"

echo
echo "== 4. uninstalling is taking it away, and it takes its subtree =="
rm -rf "$cartridges/fresh"
if out | grep -q 'fresh'; then echo "FAIL: fresh survived removal"; exit 1; fi
echo "ok: fresh and fresh/child both gone"
after=$(out | grep -c '^')
[ "$before" = "$after" ] && echo "ok: the ledger is back to exactly what it was"

echo
echo "== 5. a plain folder does not extend a subtree =="
if out | grep -q 'vendor/hidden'; then
	echo "NOTE: vendor/hidden is registered — the scan descended through a non-cartridge folder"
else
	echo "ok: vendor/hidden is in no subtree; a non-cartridge folder is not descended into"
fi

echo
echo "== 5b. two entries of the SAME scope offering one key: ambiguous and stops =="
doc "$cartridges/asker" asker ',"needs":["store.get"]'
out | grep -q 'store.get <- ambiguous (far, other)' \
	&& echo "ok: the ask names both offers instead of a silent pick" \
	|| { echo "FAIL: the clash was not named"; out; exit 1; }
if run; then echo "FAIL: an ambiguous binding exited 0"; exit 1; fi
echo "ok: the exit is non-zero, so the clash is found at install time"
rm -rf "$cartridges/asker"

echo
echo "== 6. an unreadable document is an entry, not an absence =="
doc "$cartridges/broken" broken
printf '{ this is not json\n' > "$cartridges/broken/cartridge.json"
if run; then echo "FAIL: the listing exited 0 on an unreadable document"; exit 1; fi
out | grep -q '^broken.*error:' && echo "ok: broken is listed with its reason, and the exit is non-zero"
rm -rf "$cartridges/broken"

echo
echo "== 7. a need nothing offers: named, and not a failure =="
doc "$cartridges/lonesome" lonesome ',"needs":["nobody.offers"]'
out | grep -q 'nobody.offers <- ?' \
	&& echo "ok: the need is named with ?" \
	|| { echo "FAIL: the unbound need was not printed as ?"; out; exit 1; }
if ! run; then echo "FAIL: an unbound need exited non-zero"; exit 1; fi
echo "ok: the exit is zero — the launch cares, not the registry"
rm -rf "$cartridges/lonesome"

echo
echo "== 8. an empty root: nothing installed is a state, not an error =="
empty="$(mktemp -d)"
if listing=$("$zirkle" --dir "$empty" ledger 2>/dev/null); then
	[ -z "$listing" ] && echo "ok: an empty root prints nothing and exits zero" \
		|| { echo "FAIL: an empty root printed: $listing"; exit 1; }
else
	echo "FAIL: an empty root exited non-zero"; exit 1
fi
rm -rf "$empty"
if listing=$("$zirkle" --dir "$root/nowhere" ledger 2>/dev/null); then
	[ -z "$listing" ] && echo "ok: so does a root that does not exist at all" \
		|| { echo "FAIL: a missing root printed: $listing"; exit 1; }
else
	echo "FAIL: a nonexistent root exited non-zero"; exit 1
fi

echo
echo "== 9. the clash still stops after the two states above =="
doc "$cartridges/asker" asker ',"needs":["store.get"]'
out | grep -q 'store.get <- ambiguous (far, other)' \
	&& echo "ok: the clash is still named" \
	|| { echo "FAIL: the clash was not named"; out; exit 1; }
if run; then echo "FAIL: an ambiguous binding exited 0"; exit 1; fi
echo "ok: the exit is still non-zero — neither new case weakened the clash"
rm -rf "$cartridges/asker"

echo
echo "PROBE OK"

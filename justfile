set positional-arguments

# Available development commands.
help:
    @just --list

# Build all repositories or one module.
build target='all' *args:
    @python3 scripts/workspace.py build "$@"

# Format-check and lint all repositories or one module.
check target='all' *args:
    @python3 scripts/workspace.py check "$@"

# Test a module; pass a Cargo/Bun filter after its name.
test target='all' *args:
    @python3 scripts/workspace.py test "$@"

# List module names and purposes.
modules:
    @python3 scripts/workspace.py modules

# Show a cartridge manifest, or a support module's README.
describe module:
    @python3 scripts/workspace.py describe "$@"

links:
    @python3 scripts/workspace.py links

# Start the terminal interface.
run: build
    @./cartridge run ui '{}'

ledger:
    @./cartridge ledger

# Run a profile's daemon in the foreground; Ctrl-C stops it.
daemon profile='default':
    @./cartridge --profile "$1" daemon

# Start an authenticated local development proxy.
proxy port='4242':
    @python3 scripts/workspace.py proxy "$@"

# MCP stdio transport; stdout is reserved for protocol messages.
mcp:
    @python3 scripts/workspace.py mcp

# Run isolated offline protocol checks: proxy, mcp, policy, or all.
smoke target='all':
    @python3 scripts/smoke.py "$@"

# Launch an agent through the proxy (for example: just launch claude -- -p hi).
launch agent *args:
    @./cartridge launch "$@"

# Invoke a service in a temporary host, then dispose it.
invoke service payload='null' profile='default':
    @./cartridge --profile "$3" run "$1" "$2"

# Call a service of an already-running profile.
call service payload='null' profile='default':
    @./cartridge --profile "$3" call "$1" "$2"

status profile='default':
    @./cartridge --profile "$1" status

tail profile='default':
    @./cartridge --profile "$1" tail

# Run a Rust cartridge's SDK process directly (or pass hello for capabilities).
process module *args:
    @python3 scripts/workspace.py process "$@"

# Run a declared cartridge contract; default is the tools profile contract.
verify module='':
    @if [ -n "$1" ]; then ./cartridge verify "$1"; else ./cartridge --profile tools verify; fi

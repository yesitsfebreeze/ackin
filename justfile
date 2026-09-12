help:
    @just --list

# Build the runtime, every Rust cartridge, and install the UI dependencies.
build:
    python3 scripts/workspace.py build

check:
    python3 scripts/workspace.py check

test:
    python3 scripts/workspace.py test

links:
    python3 scripts/workspace.py links

# Start the terminal interface.
run: build
    ./cartridge run ui '{}'

ledger:
    ./cartridge ledger

daemon: build
    ./cartridge daemon

verify:
    ./cartridge --profile tools verify

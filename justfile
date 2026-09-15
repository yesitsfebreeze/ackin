build *args:
    cargo build --workspace {{args}}

# One test per process: the unit tests mutate CARTRIDGE_HOME and other
# process-global state, which nextest isolates per test.
test *args:
    cargo nextest run --workspace {{args}}

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets

fmt:
    cargo fmt --all

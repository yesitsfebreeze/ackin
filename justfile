# The host and the transport crate. This repository is the host, not a
# composition: recipes that drive a set of cartridges belong to that set.

build *args:
    cargo build --workspace {{args}}

test *args:
    cargo test --workspace {{args}}

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets

fmt:
    cargo fmt --all

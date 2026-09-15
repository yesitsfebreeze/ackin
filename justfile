build *args:
    cargo build --workspace {{args}}

test *args:
    cargo test --workspace {{args}}

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets

fmt:
    cargo fmt --all

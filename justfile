run *ARGS:
    cargo run -- {{ ARGS }}

clippy *ARGS:
    cargo clippy {{ ARGS }}

build:
    cargo build

test:
    cargo test

fmt:
    cargo fmt

release LEVEL:
    cargo release -x {{ LEVEL }}

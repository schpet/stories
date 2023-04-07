run *ARGS:
    cargo run -- {{ ARGS }}

clippy *ARGS:
    cargo clippy {{ ARGS }}

build:
    cargo build

test:
    cargo test

release LEVEL:
    cargo release -x {{ LEVEL }}

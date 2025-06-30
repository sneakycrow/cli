set dotenv-load := true

default:
    just --list

build:
    cargo build --release

test:
    cargo test --release

clean:
    cargo clean

run *FLAGS:
    cargo run -- {{ FLAGS }}

install:
    cargo install --path .

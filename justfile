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
    cargo install --path packages/cli

serve-web:
    cargo run -p sc -- web serve

build-web:
    cargo run -p sc -- web build

clean-build:
    rm -rf build/

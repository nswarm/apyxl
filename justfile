# Default: list available recipes
default:
    @just --list

# Build the entire workspace
build:
    cargo build --workspace

# Run all tests with insta snapshot support
test:
    cargo insta test --workspace

# Interactively review pending snapshots
review-ss:
    cargo insta review --workspace

# Run tests and immediately review any new/changed snapshots
test-review-ss: test
    just review-snapshots

# Check formatting and lints without compiling
check:
    cargo fmt --all -- --check
    cargo clippy --workspace -- -D warnings

# Run a specific example (e.g. just example basic_rust_parser)
example name:
    cargo run --package apyxl --example {{ name }}

# Run the rust fake platform CLI example
example-rust:
    ./examples/rust_fake_platform.sh

# Run the csharp fake platform CLI example
example-csharp:
    ./examples/csharp_fake_platform.sh

# Clean build artifacts
clean:
    cargo clean

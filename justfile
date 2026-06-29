# Engram monorepo task runner. Run `just` to list recipes.

default:
    @just --list

# Build everything (Rust crates + web)
build: build-rust build-web

build-rust:
    cargo build --workspace

build-web:
    bun --filter './apps/web' build

# Run all tests
test: test-rust

test-rust:
    cargo test --workspace

# Lint + format checks (CI-style, non-mutating)
lint: lint-rust

lint-rust:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

# Auto-format
fmt:
    cargo fmt --all

# Install JS workspace deps
install:
    bun install

# Run the landing page in dev
web-dev:
    bun --filter './apps/web' dev

# What CI runs
ci: lint test build-web

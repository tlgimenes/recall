# Recall monorepo task runner. Run `just` to list recipes.

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

# Copy the canonical skills into each plugin (run after editing /skills)
sync-plugins:
    rm -rf plugins/claude-code/skills plugins/codex/skills
    cp -R skills plugins/claude-code/skills
    cp -R skills plugins/codex/skills

# CI: fail if the synced skills have drifted from /skills
sync-plugins-check: sync-plugins
    git diff --exit-code -- plugins/claude-code/skills plugins/codex/skills

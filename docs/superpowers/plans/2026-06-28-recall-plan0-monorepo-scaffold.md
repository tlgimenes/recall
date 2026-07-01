# Recall Plan 0 — Monorepo Scaffold + CI

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the polyglot monorepo skeleton — a Cargo workspace and a Bun workspace side by side, a `just` task runner, a minimal building `apps/web`, and a green CI — so every later plan drops into the right slot.

**Architecture:** Dual workspaces in one repo (Cargo `crates/*` + Bun `packages/*`/`apps/*`), governed by a `justfile`. CI runs Rust (fmt/clippy/test) and JS (bun install + web build) as separate, separately-cached jobs.

**Tech Stack:** Rust stable (rustfmt, clippy), Bun (workspaces + text `bun.lock`), Vite 6 + React 19 + React Compiler + TailwindCSS v4 (web placeholder), GitHub Actions.

## Global Constraints

- **Prerequisites on the executor machine:** a Rust stable toolchain (`cargo`, `rustfmt`, `clippy`) and **Bun ≥ 1.2** (`bun --version`). If missing, install before starting (`curl -fsSL https://bun.com/install | bash`; `rustup` for Rust).
- **Lockfiles:** commit `Cargo.lock` and the text `bun.lock`. Never commit `bun.lockb`, `package-lock.json`, or `yarn.lock`.
- **Workspace globs stay narrow:** Bun `workspaces = ["packages/*","apps/*"]` (never `crates/`); Cargo `members = ["crates/*"]`.
- **Binary name is `recall`** (set later in Plan 1); the published npm/crate names are deferred to Plan 4 (registry collisions noted in the architecture spec). Private packages here use the `@recall/*` scope and need no registry.
- **Web toolchain pins (stability for the scaffold):** `vite@^6`, `@vitejs/plugin-react@^4` (Babel-based, so React Compiler wires inline). The Vite 8 / plugin-react v6 + Vite+ path is a Plan 5 upgrade.
- **Commit style:** end every commit body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- **Reconciliation:** this plan creates the root `Cargo.toml` workspace and a stub `crates/recall-core`. Plan 1's Task 1 is therefore reduced to *fleshing out* `recall-core` (adding deps + the domain model), not creating the workspace.

---

### Task 1: Rust workspace skeleton

**Files:**
- Create: `.gitignore`
- Create: `rust-toolchain.toml`
- Create: `Cargo.toml` (workspace root)
- Create: `crates/recall-core/Cargo.toml`
- Create: `crates/recall-core/src/lib.rs`

**Interfaces:**
- Produces: a building virtual Cargo workspace with one member (`recall-core` stub) and shared `[workspace.dependencies]` consumed by Plan 1+.

- [ ] **Step 1: Create `.gitignore`**

```gitignore
# Rust
**/target

# JS
**/node_modules
**/dist
**/build

# Lockfiles we don't use
bun.lockb
package-lock.json
yarn.lock

# misc
.DS_Store
*.log
```

- [ ] **Step 2: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: Create the workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
edition = "2021"
rust-version = "1.82"
license = "MIT"
repository = "https://github.com/tlgimenes/recall"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde", "clock"] }
uuid = { version = "1", features = ["v4", "serde"] }
anyhow = "1"
tempfile = "3"
```

- [ ] **Step 4: Create `crates/recall-core/Cargo.toml`**

```toml
[package]
name = "recall-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
```

- [ ] **Step 5: Create `crates/recall-core/src/lib.rs`** (stub with a toolchain smoke test)

```rust
//! Recall core — the domain model for the convention brain.
//! Filled in by Plan 1 (model, scoping, dedup, supersession).

#[cfg(test)]
mod tests {
    #[test]
    fn toolchain_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 6: Verify the workspace builds and tests pass**

Run: `cargo test --workspace`
Expected: compiles; `1 passed` (the smoke test).

- [ ] **Step 7: Verify formatting and lints are clean**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no output, exit 0.

- [ ] **Step 8: Commit**

```bash
git add .gitignore rust-toolchain.toml Cargo.toml Cargo.lock crates/recall-core
git commit -m "chore: scaffold Cargo workspace + recall-core stub

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Bun workspace + web placeholder app

**Files:**
- Create: `package.json` (root, private)
- Create: `apps/web/package.json`
- Create: `apps/web/index.html`
- Create: `apps/web/vite.config.ts`
- Create: `apps/web/tsconfig.json`
- Create: `apps/web/src/main.tsx`
- Create: `apps/web/src/App.tsx`
- Create: `apps/web/src/index.css`
- Generate (committed): `bun.lock`

**Interfaces:**
- Produces: a Bun workspace whose `apps/web` builds to `apps/web/dist` via Vite, with React 19 + React Compiler + Tailwind v4 wired.

- [ ] **Step 1: Create the root `package.json`**

```json
{
  "name": "recall-monorepo",
  "private": true,
  "workspaces": ["packages/*", "apps/*"]
}
```

- [ ] **Step 2: Create `apps/web/package.json`**

```json
{
  "name": "@recall/web",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "@tailwindcss/vite": "^4.0.0",
    "tailwindcss": "^4.0.0",
    "babel-plugin-react-compiler": "^1.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0"
  }
}
```

- [ ] **Step 3: Create `apps/web/index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Recall — teach your AI once</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 4: Create `apps/web/vite.config.ts`**

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// base: "/" for a custom domain; set PAGES_BASE="/recall/" for the project page.
export default defineConfig({
  base: process.env.PAGES_BASE ?? "/",
  plugins: [
    react({ babel: { plugins: ["babel-plugin-react-compiler"] } }),
    tailwindcss(),
  ],
});
```

- [ ] **Step 5: Create `apps/web/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true
  },
  "include": ["src"]
}
```

- [ ] **Step 6: Create `apps/web/src/main.tsx`**

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

- [ ] **Step 7: Create `apps/web/src/App.tsx`**

```tsx
export default function App() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center bg-zinc-950 text-zinc-100">
      <h1 className="text-4xl font-bold tracking-tight">Recall</h1>
      <p className="mt-3 text-zinc-400">
        Teach your AI once. It writes code like you — in every repo, every agent.
      </p>
    </main>
  );
}
```

- [ ] **Step 8: Create `apps/web/src/index.css`**

```css
@import "tailwindcss";
```

- [ ] **Step 9: Install deps (generates `bun.lock`)**

Run: `bun install`
Expected: resolves; creates a text `bun.lock` at the repo root.

- [ ] **Step 10: Verify the web app builds**

Run: `bun --filter './apps/web' build`
Expected: Vite build succeeds; `apps/web/dist/index.html` exists.

- [ ] **Step 11: Verify the React Compiler actually ran** (optional sanity)

Run: `grep -rl "react.memo_cache_sentinel\|c(\|_c =" apps/web/dist/assets/*.js | head -1`
Expected: a matching JS asset (React Compiler injects the `useMemoCache` runtime). If empty, confirm `babel-plugin-react-compiler` is in `vite.config.ts` before continuing.

- [ ] **Step 12: Commit**

```bash
git add package.json bun.lock apps/web
git commit -m "chore: bun workspace + Vite/React19/React-Compiler/Tailwind4 web placeholder

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: `justfile` task runner

**Files:**
- Create: `justfile`

**Interfaces:**
- Produces: `just build|test|lint|fmt|install|web-dev|ci` recipes spanning both ecosystems. Later plans add `sync-plugins` (Plan 3) and release recipes (Plan 4).

- [ ] **Step 1: Create `justfile`**

```just
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
```

- [ ] **Step 2: Verify `just` lists recipes**

Run: `just --list`
Expected: shows `build`, `test`, `lint`, `fmt`, `install`, `web-dev`, `ci`, etc.
(If `just` isn't installed: `cargo install just` or `brew install just`.)

- [ ] **Step 3: Verify the full local CI recipe passes**

Run: `just ci`
Expected: fmt check + clippy + `cargo test` + web build all succeed.

- [ ] **Step 4: Commit**

```bash
git add justfile
git commit -m "chore: add justfile task runner (build/test/lint/ci)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: GitHub Actions CI

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: a CI workflow with two independent jobs (rust, web) on push-to-main and PRs.

- [ ] **Step 1: Create `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Format
        run: cargo fmt --all --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Test
        run: cargo test --workspace

  web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
        with:
          bun-version: latest
      - name: Cache Bun store
        uses: actions/cache@v4
        with:
          path: ~/.bun/install/cache
          key: ${{ runner.os }}-bun-${{ hashFiles('bun.lock') }}
          restore-keys: ${{ runner.os }}-bun-
      - name: Install
        run: bun install --frozen-lockfile
      - name: Typecheck
        run: bun --filter './apps/web' typecheck
      - name: Build web
        run: bun --filter './apps/web' build
```

- [ ] **Step 2: Validate the workflow YAML locally**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('ci.yml OK')"`
Expected: `ci.yml OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: rust (fmt/clippy/test) + web (bun build) workflow

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 4: Push and confirm CI is green**

```bash
git push -u origin HEAD
```
Then check the Actions run (`gh run watch` or the GitHub UI). Both `rust` and `web` jobs must pass. If anything fails, fix it before marking Plan 0 complete.

---

## Self-Review

**Spec coverage (architecture §2, §6, roadmap Plan 0):**
- Dual cargo + Bun workspace with narrow globs → Tasks 1, 2. ✅
- `justfile` runner → Task 3. ✅
- `rust-toolchain.toml`, `.gitignore` (`**/`-prefixed), root configs → Task 1. ✅
- `apps/web` placeholder that builds (Vite + React 19 + React Compiler + Tailwind v4) → Task 2. ✅
- Green `ci.yml` (separate rust/web jobs, per-ecosystem caches) → Task 4. ✅
- Committed `bun.lock` text lockfile → Task 2. ✅
- Release configs (`dist-workspace.toml`, `release-plz.toml`, release workflows) → correctly **deferred to Plan 4**. ✅
- `skills/`, `plugins/`, `sync-plugins` → correctly **deferred to Plan 3**. ✅

**Placeholder scan:** No TBD/TODO. Web pins are intentional (stability), with the upgrade path documented in Global Constraints.

**Type consistency:** Workspace member path (`crates/recall-core`) matches Plan 1's expectations; `[workspace.dependencies]` versions match those Plan 1 Task 2+ consume. `apps/web` build output (`apps/web/dist`) matches the Plan 5 Pages deploy path.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-recall-plan0-monorepo-scaffold.md`. Two execution options:

**1. Subagent-Driven (recommended)** — fresh subagent per task, review between tasks.

**2. Inline Execution** — run tasks here in batches with checkpoints.

Which approach?

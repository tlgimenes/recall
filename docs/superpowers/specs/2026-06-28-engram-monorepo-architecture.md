# Engram — Monorepo Architecture & Build/Release Spec

**Status:** Proposed, under review
**Date:** 2026-06-28
**Complements:** `2026-06-28-engram-convention-brain-design.md` (the product) and
`../plans/2026-06-28-engram-core-mcp.md` (Plan 1). This document defines how the
*whole* repository is structured, built, tested, released, and deployed.

---

## 1. What lives in this monorepo

Engram is not just an MCP server. The repo is a polyglot monorepo containing:

1. **The Rust core** — the `engram` binary (MCP server + CLI) and its crates.
2. **`agent-cli`** — a standalone, reusable Rust crate that drives Claude Code
   and Codex as an LLM backend ("Rust bindings for the agent CLIs"). Published on
   its own to crates.io; `engram` is its first consumer.
3. **Skills** — canonical `SKILL.md` sources shared by both plugin targets.
4. **Plugin packaging** — Claude Code plugin + Codex plugin + marketplace
   catalogs, so users install Engram from each ecosystem's plugin store.
5. **npm distribution** — a launcher package so `npx -y engram` / `npm i -g`
   work; this is also what the plugins' `.mcp.json` invokes.
6. **Landing page** — a Vite + React 19 (React Compiler) + TailwindCSS v4
   marketing site, served on GitHub Pages.
7. **CI/CD** — GitHub Actions for test/lint, cross-platform release, package
   publishing, and landing-page deploy.

---

## 2. Directory layout

A **dual-workspace polyglot layout** (the *repository structure* used by Biome,
Oxc, and Tauri — the layout pattern, not the Biome tool): a Cargo workspace and a
**Bun** workspace coexist at the root, managing disjoint directories. `just` is
the language-agnostic task runner. JS lint/format/test/build run on the **Oxc
stack via Vite+** (see §7) — explicitly not Biome.

```
/
├── Cargo.toml                     # [workspace] resolver="2", members=["crates/*"]
├── Cargo.lock                     # committed
├── rust-toolchain.toml            # pinned stable toolchain
├── dist-workspace.toml            # cargo-dist config (release matrix + installers)
├── release-plz.toml               # release-plz config (version/changelog/tag)
│
├── package.json                   # private root; "workspaces": ["packages/*","apps/*"] (NARROW)
├── bun.lock                       # committed (text lockfile, Bun ≥1.2)
│
├── justfile                       # `just build|test|lint|ci|sync-plugins|demo`
├── .gitignore                     # **/target  **/node_modules  **/dist  **/build
├── README.md                      # the launch landing-page README (Plan 6)
│
├── crates/                        # ── Cargo workspace members ──
│   ├── engram-core/               # pure domain model, scoping, dedup/supersede
│   ├── engram-store/              # SQLite persistence
│   ├── engram-inject/             # selection + render + git context detection
│   ├── engram-mcp/                # rmcp stdio server
│   ├── engram-cli/                # the `engram` binary (CLI + `mcp` subcommand)
│   ├── agent-cli/                 # standalone: drive claude -p / codex exec from Rust
│   ├── engram-llm/                # provider trait wiring (uses agent-cli)   [Plan 2]
│   └── engram-capture/            # session distillation + curation          [Plan 2]
│
├── packages/                      # ── Bun workspace: JS distribution ──
│   └── engram/                    # npm launcher (optionalDependencies + bin shim)
│       └── (per-platform pkgs generated at release time, not committed)
│
├── apps/                          # ── Bun workspace: web ──
│   └── web/                       # Vite + React + Tailwind v4 landing page
│
├── plugins/                       # ── plugin packaging ──
│   ├── claude-code/
│   │   ├── .claude-plugin/plugin.json
│   │   ├── skills/                # synced from /skills by `just sync-plugins`
│   │   ├── commands/              # /engram-learn etc.
│   │   ├── hooks/hooks.json       # SessionStart, Stop (+ PreToolUse fast-follow)
│   │   └── .mcp.json              # runs `npx -y engram mcp`
│   └── codex/
│       ├── .codex-plugin/plugin.json
│       ├── skills/                # synced from /skills
│       ├── hooks/hooks.json
│       └── .mcp.json
│
├── skills/                        # canonical SKILL.md sources (single source of truth)
│   └── engram/SKILL.md
│
├── .claude-plugin/
│   └── marketplace.json           # our Claude Code marketplace catalog → ./plugins/claude-code
├── .agents/plugins/
│   └── marketplace.json           # Codex marketplace catalog → ./plugins/codex
│
├── .mcp.json                      # DEV ONLY: points at ./target/debug/engram (dogfooding)
│
├── docs/
│   └── superpowers/{specs,plans}/ # these documents
│
└── .github/
    └── workflows/
        ├── ci.yml                 # PR: cargo fmt/clippy/test + bun build + vite lint/test
        ├── release-plz.yml        # push main: version PR; on merge → crates.io + tag
        ├── release.yml            # cargo-dist generated: tag → binaries/npm/brew
        └── deploy-web.yml         # apps/web changes → GitHub Pages
```

**Boundary rules (the real footguns, from research):**
- Bun `workspaces` globs MUST stay narrow (`packages/*`, `apps/*`) — never glob
  `crates/` or use top-level `*`/`**`, or Bun tries to manage Rust dirs.
- Cargo `members = ["crates/*"]`; use `exclude` for any stray sub-`Cargo.toml`.
- `.gitignore` uses `**/`-prefixed globs so nested `target`/`node_modules`/`dist`
  are all ignored. Commit `Cargo.lock` AND `bun.lock` (text lockfile); never
  `package-lock.json`/`yarn.lock`/`bun.lockb` (binary, legacy).

---

## 3. The `agent-cli` crate (Rust ↔ agent-CLI bindings)

A first-class, independently useful crate — not just an Engram internal. It lets
any Rust program use the user's installed Claude Code / Codex as an LLM backend.

- **Public surface:**
  ```rust
  #[async_trait] pub trait AgentProvider {
      async fn complete_json(&self, prompt: &str, schema: &serde_json::Value)
          -> Result<serde_json::Value>;
      async fn health(&self) -> ProviderHealth;
  }
  pub struct ClaudeCli { /* model, timeout, … */ }   // spawns `claude -p`
  pub struct CodexCli  { /* … */ }                    // spawns `codex exec`
  pub struct ApiKey    { /* anthropic/openai direct */ } // fallback
  pub fn detect() -> Option<Box<dyn AgentProvider>>;  // prefer installed+authed CLI
  ```
- **Guardrails baked in** (from feasibility research): spawn only the genuine
  `claude`/`codex` binaries; never extract OAuth tokens or use the Agent SDK with
  subscription OAuth; bounded calls (`--max-turns 1`, no tools, hard timeout,
  validated JSON, retry on malformed); `ApiKey` is a first-class fallback;
  surface quota consumption.
- **Why standalone:** reusable, separately star-able, and forces a clean seam
  between Engram's logic and "how we get cheap inference." `engram-llm` is a thin
  adapter over it; `engram-capture` depends on `engram-llm`.
- Published to crates.io independently (its own version line under release-plz).

---

## 4. Skills: one source, two targets

`SKILL.md` is byte-identical across Claude Code and Codex. To stay DRY:
- Canonical sources live in `/skills/<skill>/SKILL.md`.
- `just sync-plugins` copies `/skills/*` into `plugins/claude-code/skills/` and
  `plugins/codex/skills/`. CI runs `just sync-plugins --check` to fail if the
  copies drift. (Copy, not symlink — Windows + plugin-install tarballs.)

The shared `.mcp.json` in each plugin invokes the npm launcher:
```json
{ "mcpServers": { "engram": { "command": "npx", "args": ["-y", "engram", "mcp"] } } }
```
So an end user who installs the plugin gets the MCP server with no manual binary
step — npm resolves the right platform binary.

---

## 5. Distribution & release pipeline

**Two-tool flow (the documented 2026 standard):**

1. **release-plz** (`release-plz/release-plz-action@v0.5`) on push to `main`:
   opens a Release PR that bumps versions + changelogs. On merge it publishes
   crates to crates.io and **creates the git tag** — `engram` and `agent-cli`
   are versioned independently in the workspace.
   ```toml
   # release-plz.toml
   [workspace]
   git_tag_enable = true        # the tag is the cargo-dist trigger
   git_release_enable = false   # cargo-dist owns the GitHub Release
   ```
2. **cargo-dist** (`dist` v0.32, config in `dist-workspace.toml`) on tag push:
   cross-compiles the matrix and publishes installers.
   ```toml
   [dist]
   cargo-dist-version = "0.32.0"
   ci = ["github"]
   installers = ["shell", "powershell", "homebrew", "npm"]
   targets = ["x86_64-apple-darwin","aarch64-apple-darwin",
              "x86_64-unknown-linux-gnu","aarch64-unknown-linux-gnu",
              "x86_64-pc-windows-msvc"]
   tap = "tlgimenes/homebrew-engram"
   npm-scope = "@tlgimenes"
   publish-jobs = ["homebrew", "npm"]
   ```

**Install surfaces users get:**
- `curl -fsSL https://github.com/tlgimenes/engram/releases/latest/download/engram-installer.sh | sh`
- `brew install tlgimenes/engram/engram`
- `npx -y engram` / `npm i -g engram` (also how the plugins launch it)
- `cargo install engram-cli`

**Decisions / flags:**
- **npm strategy:** start with cargo-dist's npm installer (one package +
  postinstall download). Flagged upgrade: move to the **Biome-style
  per-platform `optionalDependencies`** pattern (lockfile integrity, offline,
  no postinstall) once it matters. Tracked as a Plan 4 follow-up, not a blocker.
- **cargo-dist maintenance:** axodotdev wound down; v0.32 still works. Track the
  `astral-sh/cargo-dist` fork (now under OpenAI) and be ready to switch.
- **Secrets required:** `RELEASE_PLZ_TOKEN` (PAT/App token — the default
  `GITHUB_TOKEN` won't re-trigger `release.yml` on the new tag),
  `CARGO_REGISTRY_TOKEN` (or crates.io Trusted Publishing OIDC), `NPM_TOKEN`,
  `HOMEBREW_TAP_TOKEN`.

---

## 6. CI (pull requests)

`ci.yml` runs two independent, separately-cached jobs (never share cache keys
across ecosystems):

- **rust:** `Swatinem/rust-cache@v2`; `cargo fmt --check`, `cargo clippy
  -D warnings`, `cargo test --workspace`.
- **js:** `oven-sh/setup-bun@v2` → cache `~/.bun/install/cache` (key =
  `hashFiles('bun.lock')`) → `bun install --frozen-lockfile` →
  `bun --filter './apps/web' build` → lint/format/test via **Vite+**
  (`vite lint`, `vite fmt --check`, `vite test`) or the Oxc fallback
  (`oxlint`, `oxfmt --check`, `vitest run`) → `just sync-plugins --check`
  (skills not drifted).

Path filters keep web-only and plugin-only changes from running the full matrix
where it adds nothing, but `cargo test` always runs on any `crates/**` change.

---

## 7. Landing page (`apps/web`)

- **Stack:** **React 19 + React Compiler** on Vite, with **TailwindCSS v4** via
  `@tailwindcss/vite` (CSS-first: one `@import "tailwindcss";`, no
  `tailwind.config.js`/`postcss.config.js`, customization in a `@theme {}`
  block). Build/lint/format/test via **Vite+** (VoidZero's Oxc-based toolchain)
  if available at Plan 5 time; otherwise plain **Vite + Vitest + Oxlint/Oxfmt**
  — same Oxc tools, so the choice is transparent to the code.
- **React Compiler wiring:** React Compiler 1.0 (stable, requires React 19) via
  `@vitejs/plugin-react`; on the plugin-react v6 / Vite 8 path add
  `@rolldown/plugin-babel`. Lint rules come from `eslint-plugin-react-hooks`
  (the standalone `eslint-plugin-react-compiler` is deprecated). Exact import
  for the compiler preset is verified against plugin-react release notes at
  code-writing time (it's still settling).
- **Content (Plan 5):** hero with the one-liner + VHS demo GIF above the fold,
  one-command install block, the "teach once → applies everywhere" story,
  feature cards, links to plugin stores + GitHub. The README and the site share
  the same demo GIF (generated by VHS, regenerated in CI so it never goes stale).
- **Deploy:** `deploy-web.yml` builds `apps/web` with **Bun** + Vite and
  publishes to **GitHub Pages** (`actions/configure-pages` →
  `upload-pages-artifact` (`path: apps/web/dist`) → `deploy-pages`), with
  `permissions: pages:write, id-token:write` and a `pages` concurrency group.
  `base` is env-driven: `/engram/` for the default project page
  (`tlgimenes.github.io/engram/`), or `/` once a custom domain (`engram.dev`) is
  configured in repo Settings → Pages. Triggered only on `apps/web/**` (+
  `bun.lock`) changes. One-time manual step: Settings → Pages → Source = "GitHub
  Actions".

---

## 8. Roadmap — the full sequence of plans

Each plan produces working, testable software and is detailed into bite-sized
TDD tasks **just before** it's executed (so earlier results inform later detail).
Plan 1 is already fully detailed; the rest are scoped here.

| Plan | Title | Deliverable (done = ) | Depends on |
|---|---|---|---|
| **0** | **Monorepo scaffold + CI** | Dual cargo + Bun workspace, `justfile`, `rust-toolchain`, `.gitignore`, root configs, empty `apps/web` placeholder building, green `ci.yml`. | — |
| **1** | **Dogfoodable core** (detailed) | `engram` binary: model + store + inject + rmcp MCP server + CLI (`learn/list/why/forget/status`); dev `.mcp.json`. Registerable on a live session. | 0 |
| **2** | **`agent-cli` + capture/curate** | Standalone `agent-cli` crate (claude/codex/apikey); `engram-llm`; `engram-capture` (session distillation, dedup→supersession); `engram review`. | 1 |
| **3** | **Plugin packaging** | `plugins/claude-code` + `plugins/codex` (manifests, synced skills, hooks: SessionStart inject + Stop capture, `/engram-learn`), marketplace catalogs, `.mcp.json`→npx. Dogfood the real plugin. | 2 |
| **4** | **Release pipeline** | `release-plz.yml` + cargo-dist `release.yml`, crates.io publish (`engram-cli`, `agent-cli`), npm launcher, homebrew tap. First tagged release. | 1 (3 ideal) |
| **5** | **Landing page** | `apps/web` Vite (Vite+/Oxc) + React 19 + React Compiler + Tailwind v4 site with real content + demo GIF; `deploy-web.yml` to GitHub Pages. | 0 |
| **6** | **Launch** | README landing page, VHS demo tape in CI, MCP Registry + Anthropic plugin directory + awesome-claude-code listings, coordinated launch. | 3,4,5 |
| **FF** | **Enforcement** (fast-follow) | `PreToolUse` hook blocking edits that violate active conventions. | 3 |

**Reconciliation note for Plan 1:** Plan 0 creates the root `Cargo.toml`
workspace (with `members = []` initially) and `.gitignore`. Plan 1's Task 1 is
therefore reduced to *adding* `crates/engram-core` to `members` (an edit, not a
file create) and creating the crate — the rest of Plan 1 is unchanged. This
edit will be applied to the Plan 1 doc when Plan 0 is approved.

**Critical-path ordering for "stars ASAP":** 0 → 1 → 3 → 4 → 5 → 6, with 2 and
FF folded in where they unblock the demo. The launch (6) needs the plugin (3),
a real install (4), and the site (5). Plan 2's LLM auto-capture makes the demo
magical but manual teaching (Plan 1) is enough to ship a working plugin, so 2
can land just before or right after the launch depending on momentum.

---

## 9. Open questions (resolve during planning)

- npm package name: is `engram` free on npm? If not, `@tlgimenes/engram` (scope
  is set in `dist-workspace.toml` anyway) or `engram-mcp`.
- crate names on crates.io: `engram-cli` (binary), `agent-cli` (likely taken →
  candidates: `agent-cli-rs`, `claude-codex-cli`, `agentshell`).
- Domain: register `engram.dev`? Custom domain → Vite `base: '/'`; otherwise the
  GitHub Pages project page → `base: '/engram/'`.
- Vite+ availability at Plan 5 time: if its preview isn't usable/licensed for us,
  fall back to Vite + Vitest + Oxlint/Oxfmt (same Oxc tools, no rework of intent).
- Whether `agent-cli` ships in v1 or is extracted after Engram proves the seam.
- Homebrew: personal tap (`tlgimenes/homebrew-engram`) for launch; homebrew-core
  later once popular.

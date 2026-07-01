# Recall — Design Spec

> Teach your AI once. It writes code like you in every repo, every branch, every agent.

**Status:** Approved direction, spec under review
**Date:** 2026-06-28
**Supersedes:** the original "Repo Lens" concept in `README.md` (see "Relationship to the original concept")

---

## 1. What we're building

Recall is a **personal convention brain for AI coding agents**. It learns how *you*
like code written — your conventions, your corrections, your taste — and applies
that knowledge automatically in **every repository, every branch, and every
agent** (Claude Code, Codex, and beyond).

It is local-first, written in Rust, ships as a single static binary, and installs
as a one-line plugin in both the Claude Code and Codex plugin stores.

### The 30-second wow (the launch demo)

1. In **repo A**, using **Claude Code**, you correct the agent once:
   *"stop creating barrel files, import directly from the module."*
2. A week later, in a **different repo B**, using **Codex**, you start a task.
3. Without you saying anything, the agent already follows the rule.

That cross-agent, cross-repo "it just knows me now" moment is the screenshot that
earns stars. It is the single feature the demo, README GIF, and launch all orbit.

---

## 2. Why this, and why now (the wedge)

The "memory for agents" space is crowded — mem0 (~60k★), agentmemory (~24k★),
Supermemory (~28k★), Zep/Graphiti, cognee, Letta. But competitive research shows
**nobody owns the exact intersection** Recall targets:

> **personal** (keyed to the developer, not the project)
> × **cross-repo** × **cross-branch** × **cross-agent**
> × **curated conventions** (compact rules, not raw activity logs)
> × **enforced** (fast-follow).

- **agentmemory** has cross-agent reach but stores *activity logs*, not curated
  conventions, and has no branch model (and suffers "cross-project pollution").
- **Supermemory** has the right *personal-convention scoping* (git-email = user
  scope) but only works in Claude Code + OpenCode — not cross-agent.

The two leaders each hold *half* the wedge. Recall is the half nobody ships:
**curated + universal-agent + branch-aware + (later) enforced.**

Two technical gaps in the whole field become our moat:

1. **Staleness / conflict.** Vector memories confidently inject last year's
   convention. Recall models conventions with **temporal supersession** — a new
   rule cleanly retires the old one.
2. **Enforcement.** Even with perfect memory, agents treat rules as soft
   suggestions ("ignored by message 8"). Recall's fast-follow turns retrieved
   conventions into a `PreToolUse` gate that *blocks* violations.

---

## 3. Goals and non-goals

### Goals (MVP)
- Learn durable, personal coding conventions from how the developer works.
- Scope each convention correctly: global / language / repo / branch.
- Inject the *relevant* conventions into any agent session, cheaply and freshly.
- Work in both Claude Code and Codex from one codebase.
- Be local-first, inspectable, and editable — never a silent black box.
- Use the developer's *existing* Claude Code / Codex subscription for the small
  LLM tasks (distillation, dedup), with a bring-your-own-API-key fallback.
- Be a single fast Rust binary with a one-line install.

### Non-goals (explicitly cut from MVP — YAGNI)
- **RAG / semantic search over a repo** (`repo.search`, `context_pack`,
  embeddings, `architecture_map`). This space is saturated (claude-context,
  repomix, aider). We do not compete here.
- A chat UI. Recall has no conversation surface of its own.
- A cloud sync service or team/shared memory. MVP is single-developer, local.
  (Cloud sync is a possible future, not now.)
- Running a local model. We use the agent CLIs as the LLM provider instead.

---

## 4. Architecture overview

Recall is one Rust binary that operates in three modes, over one local store.

```
                         ┌──────────────────────────────────────┐
   Claude Code  ─hooks──▶│                                        │
   Codex        ─hooks──▶│   recall daemon / CLI (Rust, tokio)    │
   any agent    ─MCP───▶ │                                        │
                         │  ┌──────────┐  ┌──────────────────┐   │
                         │  │ Capture  │  │ Inject (MCP +     │   │
                         │  │ + Curate │  │ SessionStart)     │   │
                         │  └────┬─────┘  └────────┬─────────┘   │
                         │       │                 │             │
                         │  ┌────▼─────────────────▼─────────┐   │
                         │  │   Convention Store (SQLite)     │   │
                         │  └─────────────────────────────────┘  │
                         │       │ (distillation/dedup)           │
                         │  ┌────▼───────────────────────────┐   │
                         │  │  LLM Provider trait             │   │
                         │  │  Claude(`claude -p`) /          │   │
                         │  │  Codex(`codex exec`) / ApiKey   │   │
                         │  └─────────────────────────────────┘  │
                         └──────────────────────────────────────┘
                                   ~/.recall/  (SQLite + logs)
```

### Components (each independently understandable and testable)

| Component | Responsibility | Depends on |
|---|---|---|
| `recall-core` | The convention domain model, scoping, supersession logic. Pure, no I/O. | nothing |
| `recall-store` | SQLite persistence of conventions + provenance. | `recall-core`, `rusqlite` |
| `recall-llm` | The `LlmProvider` trait + Claude/Codex/ApiKey backends (subprocess). | `tokio`, `serde_json` |
| `recall-capture` | Distill sessions/corrections into candidate conventions; dedup/conflict-check. | core, store, llm |
| `recall-inject` | Select relevant conventions for a context (repo/branch/lang) and render them. | core, store |
| `recall-mcp` | MCP server exposing inject/inspect tools, built on `rmcp` (official Rust SDK). | inject, store, `rmcp` |
| `recall-cli` | `recall` binary: daemon, hook entrypoints, and human inspection commands. | all of the above |

Boundaries: `recall-core` is pure logic (the hard part — scoping and
supersession — is unit-testable with no I/O). Each adapter (`store`, `llm`,
`mcp`) hides one external dependency behind a trait so it can be faked in tests.

---

## 5. Data model

A **Convention** is a compact, curated rule — the unit of memory.

```rust
struct Convention {
    id: Uuid,
    rule: String,            // imperative, compact: "Import directly; no barrel files."
    rationale: Option<String>,
    scope: Scope,            // see below
    tags: Vec<String>,       // e.g. ["imports", "typescript"]
    provenance: Provenance,  // where/when/how learned
    status: Status,          // Active | Superseded | Retired | Pending
    superseded_by: Option<Uuid>,
    confidence: f32,         // 0..1, raised by repeated corroboration
    created_at: DateTime,
    updated_at: DateTime,
}

enum Scope {
    Global,                          // applies to the developer everywhere
    Language(String),                // e.g. "typescript"
    Repo { remote_id: String },      // keyed by normalized git remote, not path
    Branch { remote_id: String, branch: String },
}

enum Status { Pending, Active, Superseded, Retired }

struct Provenance {
    source: Source,          // SessionDistill | ManualTeach | ImportedRules
    repo: Option<String>,
    branch: Option<String>,
    agent: Option<String>,   // "claude-code" | "codex"
    excerpt: Option<String>, // the moment it was learned from
    learned_at: DateTime,
}
```

**Scoping rules:**
- Repos are identified by **normalized git remote URL** (not filesystem path), so
  the same repo cloned twice shares conventions, and a personal global rule
  follows the developer across every repo.
- Branch scope is derived from current git state at capture/inject time. Branch
  conventions layer *on top of* repo → language → global (most specific wins;
  conflicts resolved by scope specificity then recency).

**Supersession (the anti-staleness mechanism):** when a new convention conflicts
with an existing active one in the same scope, the curator marks the old one
`Superseded` (linked via `superseded_by`) rather than keeping both. Injection
never serves `Superseded`/`Retired` rules. The history is preserved for `recall
why`.

Storage: SQLite at `~/.recall/recall.db`. Conventions are also exportable to/
importable from plain Markdown (`~/.recall/conventions.md`) so the brain is
human-portable and git-trackable if the user wants — addressing the "plain
markdown vs SQLite" open question with **both** (SQLite is source of truth,
Markdown is an export/import view).

---

## 6. Core flows

### 6.1 Capture → Curate

**Primary (automatic): session distillation.**
- A `Stop` / session-end hook hands the daemon the session transcript path.
- `recall-capture` sends the transcript to the `LlmProvider` with a strict
  JSON-schema prompt: *"Extract durable, personal coding conventions the user
  expressed or corrected. For each: the rule (imperative, <140 chars), a scope
  suggestion, tags, and the excerpt it came from. Ignore one-off task details."*
- Each candidate is **deduped and conflict-checked** against existing
  conventions (lexical + a cheap LLM "is this the same/contradictory?" check).
  New → `Pending`; corroborates existing → raise confidence; contradicts → open
  a supersession.
- `Pending` conventions are surfaced for one-tap confirmation (via `recall
  review` CLI and a notification), then promoted to `Active`. Auto-promote rules
  that reach a confidence threshold from repeated corroboration.

**Secondary (manual): explicit teaching.**
- `recall learn "always use early returns" --scope global` — instant `Active`.
- A `/recall-learn` slash command (shipped in the plugin) for in-agent teaching.

Manual teaching is the robust fallback that always works; session distillation is
the magic. Both exist in MVP.

### 6.2 Inject

- A **SessionStart hook** calls the daemon with cwd → daemon resolves git
  remote + branch + detected languages → `recall-inject` selects the relevant
  `Active` conventions (most-specific-scope-wins, capped to a token budget) →
  returns them as `additionalContext` the agent sees before its first action.
- An **MCP tool** `recall_conventions(context)` lets the agent pull relevant
  rules on demand mid-session.
- Injection is small and fresh: only relevant, only active, only what fits the
  budget. This is the antidote to a static, ever-growing, stale `CLAUDE.md`.

### 6.3 Inspect / Edit (memory must be auditable)

CLI surface (`recall …`):
- `recall list [--scope …] [--tag …]` — show active conventions.
- `recall why <id|query>` — show provenance: where/when/how it was learned, and
  what it superseded.
- `recall review` — confirm/reject `Pending` conventions.
- `recall learn "<rule>" [--scope …]` — manually add.
- `recall forget <id>` — retire a convention.
- `recall export [--md]` / `recall import <file>` — Markdown round-trip.
- `recall status` — daemon + provider health, quota disclosure.

---

## 7. LLM provider abstraction

The small LLM tasks (distill, dedup, conflict-check) run through a trait so we
are never hard-wired to one vendor or one billing model.

```rust
#[async_trait]
trait LlmProvider {
    async fn complete_json(&self, prompt: &str, schema: &JsonSchema)
        -> Result<serde_json::Value>;
    async fn health(&self) -> ProviderHealth;
}
```

Backends (subprocess to the *official, unmodified* CLIs — the sanctioned path):

- **ClaudeBackend** — `claude -p --output-format json --json-schema <schema>
  --max-turns 1 --allowedTools ""`. Uses the user's existing Claude login.
- **CodexBackend** — `codex exec --json --output-schema <file> --sandbox
  read-only --skip-git-repo-check`. Uses the user's existing Codex login.
- **ApiKeyBackend** — direct Anthropic/OpenAI API with the user's own key.

**Critical policy guardrails (from feasibility research):**
- We only ever spawn the genuine `claude` / `codex` binaries. We never extract or
  reuse OAuth tokens, and we do not use the Agent SDK with subscription OAuth
  (Anthropic forbids both).
- Subscription-backed `claude -p` is **policy-volatile** — Anthropic announced,
  then paused, metering it onto a paid pool. So: provider selection is explicit,
  the **ApiKeyBackend is a first-class fallback**, and `recall status`
  **discloses** that distillation consumes the user's Claude/Codex plan quota.
- Provider auto-detection at startup: prefer an installed+authed CLI; degrade
  gracefully (manual teaching still works with no provider at all).
- Calls are bounded: single turn, no tools, hard timeout, validated JSON output
  (retry on malformed), worker-pool concurrency cap, and throttling to protect
  the user's interactive quota.

---

## 8. Distribution & packaging

One repository produces both plugins; the heavy artifact is shared.

```
recall/
├── crates/                         # the Rust workspace (core, store, llm, …)
├── npm/                            # cross-compiled binary published to npm
├── .claude-plugin/
│   ├── plugin.json                 # Claude Code manifest
│   └── marketplace.json            # our Claude Code marketplace catalog
├── .codex-plugin/
│   └── plugin.json                 # Codex manifest (near-identical)
├── .agents/plugins/marketplace.json # Codex marketplace catalog
├── skills/recall/SKILL.md          # SHARED verbatim by both hosts
├── commands/                       # /recall-learn etc.
├── hooks/hooks.json                # SessionStart, Stop (+ PreToolUse fast-follow)
└── .mcp.json                       # SHARED: launches the binary as MCP server
```

- `SKILL.md` and `.mcp.json` use identical formats across Claude Code and Codex —
  authored once.
- The Rust daemon is **cross-compiled and published to npm** (platform
  sub-packages + JS shim); both `.mcp.json` files run `npx -y recall-mcp` (or a
  scoped name if `recall` is taken). Alternative installs: `cargo install
  recall`, Homebrew, and a `curl | sh` script.
- Install is one line per host:
  - Claude Code: `/plugin marketplace add tlgimenes/recall` →
    `/plugin install recall@recall`
  - Codex: `codex plugin marketplace add tlgimenes/recall` → install via
    `codex /plugins`
- Pre-launch listings: MCP Registry, Anthropic plugin directory,
  awesome-claude-code, awesome-mcp-servers.

---

## 9. Testing strategy

- **`recall-core`**: pure unit tests for scope resolution, conflict detection,
  and supersession — the logic most likely to be subtly wrong. No I/O.
- **`recall-store`**: tests against a temp SQLite db; round-trip + migration.
- **`recall-llm`**: a `FakeProvider` for deterministic capture tests; a small set
  of `#[ignore]` integration tests that actually shell to `claude`/`codex` when
  available in CI.
- **`recall-capture`**: golden-transcript tests — given a recorded session,
  assert the extracted conventions (using `FakeProvider` with canned output, plus
  schema-validation tests on real provider output).
- **`recall-inject`**: given a store + a (repo, branch, lang) context, assert the
  selected + ordered conventions and the token-budget cap.
- **End-to-end**: a scripted scenario reproducing the launch demo (teach in repo
  A, inject in repo B) as an integration test — this *is* the product promise, so
  it gets a guarding test.

TDD throughout: tests precede implementation per the project's workflow.

### 9.1 Development workflow: dogfood Recall as a local MCP server

Recall is built by *using* Recall. The `recall-mcp` server (with at least a
runnable stub answering `tools/list` and the inject/inspect tools) is an **early
milestone**, not a final step, so we can register the local debug build as an MCP
server on the developer's own Claude Code / Codex session and exercise the real
tools while building.

Dev registration points the shipped `.mcp.json` schema at the local binary:

```jsonc
// dev .mcp.json (or `claude mcp add recall -- ./target/debug/recall mcp`)
{ "mcpServers": {
    "recall": { "command": "./target/debug/recall", "args": ["mcp"] }
} }
```

This gives a tight loop: implement a tool → rebuild → call it live in-session →
assert behavior. The same config, repointed at `npx -y recall-mcp`, is what end
users get — so dogfooding also continuously validates the real integration
surface. Implication for the plan: bring up a thin end-to-end skeleton (store +
one MCP tool) before fleshing out capture/curate, so there is something to
dogfood from day one.

---

## 10. MVP scope vs. fast-follow

**MVP (the launch):** capture (session distillation + manual teach) → curate
(dedup/conflict/supersession) → inject (SessionStart + MCP) → inspect (CLI), with
the Claude + Codex + ApiKey provider trait, shipped as both plugins, with a
landing-page README and a VHS demo GIF. Claude Code is the polished launch
target; Codex ships the same week.

**Fast-follow #1 — Enforcement.** A `PreToolUse` hook that checks a proposed
edit against active conventions and warns or blocks on violation. This converts
"memory" into "changes outcomes" and is the natural second launch beat.

**Later (not committed):** cloud sync across machines, team/shared conventions,
importing existing `.cursorrules` / `CLAUDE.md` / `AGENTS.md` as seed
conventions, an inspection web UI.

---

## 11. Launch plan (so the work actually earns stars)

- README as a landing page: ≤10-word problem-first one-liner, **VHS demo GIF
  above the fold** (regenerated in CI so it never goes stale), one-command
  install, quantified badges, the memorable "teach once, applies everywhere" hook.
- A single coordinated 48-hour push to clear GitHub Trending velocity: HN front
  page (Tue–Thu, 9am ET), r/LocalLLaMA + r/commandline + r/rust, an X demo
  thread, Console.dev, same-week TLDR.
- Pre-list everywhere (MCP Registry, Anthropic plugin directory,
  awesome-claude-code) before launch day.
- A recurring re-spike hook to manufacture (post-MVP): e.g. a public "convention
  pack" gallery or a shareable `recall export`.

---

## 12. Relationship to the original concept

The original `README.md` framed "Repo Lens" as a broad local repo-intelligence
layer (context packs, semantic search, architecture maps, many MCP tools). That
surface is **saturated** and unfocused for a star-driven launch. Recall keeps the
original's best instincts — local-first, MCP-first, layered memory (global /
project / branch), inspectable memory, deterministic git tooling, and *using
stronger agents rather than competing with them* — and concentrates them into the
one unowned wedge: the **personal, cross-repo, cross-branch, cross-agent, curated
convention brain.** The README will be rewritten as the launch landing page.

The GitHub repository will be renamed `repo-lens` → `recall`.

---

## 13. Open questions (to resolve during planning)

- Exact npm package name if `recall` is taken (`recall-cli`? scoped?).
- Confidence threshold for auto-promoting `Pending` → `Active` without review.
- How aggressive session distillation should be by default (privacy: it reads
  transcripts — must be local-only, opt-outable, and never sent anywhere except
  the user's own chosen LLM provider).
- Whether the SessionStart injection should also write a transient `AGENTS.md`
  fragment for agents that don't support hook-based context injection.

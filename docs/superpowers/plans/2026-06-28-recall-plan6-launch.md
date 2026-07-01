# Recall Plan 6 — Launch

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Tasks 1–3 are concrete deliverables (TDD-style verification where it applies); Tasks 4–6 are an operational launch checklist to run with the maintainer, not autonomous code.

**Goal:** Convert a working product into GitHub stars — a landing-page README with a demo GIF, listings everywhere developers discover tools, and a single coordinated 48-hour launch that clears GitHub Trending velocity.

**Architecture:** The README is the conversion surface (problem-first one-liner → demo GIF → one-command install → the cross-agent story). A VHS tape regenerates the demo GIF in CI so it never goes stale. Directory listings (MCP Registry, Anthropic plugin directory, awesome-lists) are seeded before launch day so the launch traffic finds Recall everywhere. The launch fires all channels in one window.

## Global Constraints

- **Prereqs:** Plans 1–5 done and a real release exists (Plan 4) — install commands in the README must actually work before launch.
- **Honesty:** every claim in the README must be true and verifiable. No fake metrics. If a number isn't real yet (stars, installs), don't show it.
- **Don't buy stars / don't spam.** Lead with the problem, be present in comments.
- **Commit style:** end every commit body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

### Task 1: The launch README (replaces the old Repo Lens concept)

**Files:**
- Modify: `README.md` (full rewrite as the landing page)

- [ ] **Step 1: Replace `README.md`** with:

```markdown
<div align="center">

# 🧠 Recall

### Teach your AI once. It writes code like you — in every repo, every branch, every agent.

[![CI](https://github.com/tlgimenes/recall/actions/workflows/ci.yml/badge.svg)](https://github.com/tlgimenes/recall/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[Install](#install) · [How it works](#how-it-works) · [Why Recall](#why-recall)

![Recall demo](docs/assets/demo.gif)

</div>

---

**Recall is a personal coding-convention brain for AI agents.** Correct your AI
once — "stop using barrel files", "always early-return", "we use snake_case here"
— and Recall remembers it and applies it in **every repo, every branch, and every
agent**. Claude Code, Codex, and beyond.

It's local-first, a single fast Rust binary, and it uses the agent you already
have (Claude Code / Codex) to learn — no extra model to run or pay for.

## The problem

Your AI coding agent forgets. Every new repo, every new branch, every switch from
Claude Code to Codex — you re-explain the same conventions. `CLAUDE.md` files go
stale, cost tokens forever, and don't travel across tools.

## How it works

1. **Correct it once.** Tell any agent how you like things — or just work, and
   Recall distills your conventions from the session.
2. **Recall remembers.** Curated, compact rules scoped to you, a language, a repo,
   or a branch. Stale rules are superseded, never piled up.
3. **Applied everywhere.** Every new session in every repo and every agent starts
   already knowing your conventions.

## Install

**Claude Code**
```
/plugin marketplace add tlgimenes/recall
/plugin install recall@recall
```

**Codex**
```
codex plugin marketplace add tlgimenes/recall
```
then install Recall from `codex /plugins`.

**Or the CLI directly**
```bash
npx -y @tlgimenes/recall            # run / try it
brew install tlgimenes/recall/recall
```

## Why Recall

| | Recall | Static CLAUDE.md | Other memory tools |
|---|:---:|:---:|:---:|
| Personal, cross-repo | ✅ | ❌ | partial |
| Cross-branch | ✅ | ❌ | ❌ |
| Cross-agent (Claude + Codex) | ✅ | ❌ | partial |
| Curated rules (not activity logs) | ✅ | ✅ | ❌ |
| Auto-supersedes stale rules | ✅ | ❌ | ❌ |
| Enforces (blocks violations) | ✅ opt-in | ❌ | ❌ |
| Local-first, single binary | ✅ | n/a | mixed |

## Inspect everything

```bash
recall list           # what Recall knows
recall why <id>       # where it learned a rule
recall forget <id>    # retire one
```

## How it learns without a separate model

Recall drives your installed Claude Code / Codex (`claude -p` / `codex exec`) for
the small distillation tasks — so inference uses the subscription you already
have. Bring an API key instead if you prefer. Conventions never leave your machine
except to your own chosen provider.

## License

MIT.
```

- [ ] **Step 2: Verify links + that install commands match the shipped names**

Run: `grep -n "@tlgimenes/recall\|tlgimenes/recall/recall\|recall@recall" README.md`
Expected: matches Plan 3/4 names exactly. (The `docs/assets/demo.gif` is produced in Task 2.)

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: launch README landing page

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: VHS demo GIF (auto-regenerated in CI)

**Files:**
- Create: `docs/demo.tape` (VHS script)
- Create: `.github/workflows/demo.yml` (regenerate the GIF on change)
- Create: `docs/assets/.gitkeep`

- [ ] **Step 1: Create `docs/demo.tape`** (the cross-agent story as a terminal recording)

```tape
# Recall demo — render with: vhs docs/demo.tape
Output docs/assets/demo.gif
Set FontSize 18
Set Width 1100
Set Height 600
Set Theme "Catppuccin Mocha"
Set Padding 24

Type "# repo: acme/api · Claude Code" Enter
Sleep 600ms
Type "you: stop creating barrel files — import directly" Enter
Sleep 800ms
Type "recall ✓ learned: \"Import directly; no barrel files\" (global)" Enter
Sleep 1s
Type "" Enter
Type "# a week later · repo: acme/web · Codex" Enter
Sleep 600ms
Type "you: add a users service" Enter
Sleep 800ms
Type "codex: created users.service.ts — imported directly from ./user" Enter
Sleep 600ms
Type "↳ already follows your convention. you never said a word." Enter
Sleep 2s
```

> Note: this scripted recording tells the story deterministically. Once the real
> binary + plugins are installed, re-record against a live session for authenticity
> and replace this tape's `Type` lines with real `claude`/`codex` invocations.

- [ ] **Step 2: Create `.github/workflows/demo.yml`**

```yaml
name: Demo GIF

on:
  push:
    branches: [main]
    paths: ["docs/demo.tape", ".github/workflows/demo.yml"]
  workflow_dispatch:

permissions:
  contents: write

jobs:
  vhs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: charmbracelet/vhs-action@v2
        with:
          path: docs/demo.tape
      - name: Commit regenerated GIF
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add docs/assets/demo.gif
          git diff --cached --quiet || git commit -m "chore: regenerate demo GIF"
          git push
```

- [ ] **Step 3: Create `docs/assets/.gitkeep`** (empty placeholder so the path exists)

- [ ] **Step 4: Generate locally if VHS is installed** (optional)

Run: `vhs docs/demo.tape && ls -la docs/assets/demo.gif`
Expected: a GIF is produced (or rely on the CI workflow to generate it).

- [ ] **Step 5: Commit**

```bash
git add docs/demo.tape .github/workflows/demo.yml docs/assets/.gitkeep
git commit -m "docs: VHS demo tape + CI regeneration of demo GIF

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: LICENSE + repo metadata polish

**Files:**
- Create: `LICENSE` (MIT)
- (Manual) GitHub repo: description, topics, social preview

- [ ] **Step 1: Create `LICENSE`** (MIT, `Copyright (c) 2026 tlgimenes`).

- [ ] **Step 2: Set repo description + topics** (manual / `gh`)

```bash
gh repo edit tlgimenes/recall \
  --description "Teach your AI once — it writes code like you in every repo and every agent. A local-first coding-convention brain for Claude Code & Codex." \
  --add-topic mcp --add-topic claude-code --add-topic codex --add-topic ai-agents \
  --add-topic developer-tools --add-topic rust --add-topic memory
```

- [ ] **Step 3: Commit the LICENSE**

```bash
git add LICENSE
git commit -m "chore: MIT license

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Pre-launch directory listings (seed BEFORE launch day)

Checklist — complete and verify each (no code):

- [ ] **MCP Registry** — submit the Recall MCP server to `registry.modelcontextprotocol.io` (server.json per their schema). Confirm it appears.
- [ ] **Anthropic plugin directory** — submit the Claude Code plugin (claude.ai/admin-settings/directory or platform.claude.com/plugins/submit). Inclusion has driven thousands of stars for comparable plugins.
- [ ] **`awesome-claude-code`** — open a PR adding Recall.
- [ ] **`awesome-mcp-servers`** — open a PR adding Recall.
- [ ] **PulseMCP / Smithery** — list the server.
- [ ] **Codex plugin directory** — submit per OpenAI's process when available.
- [ ] Confirm the **GitHub Pages site** is live (`tlgimenes.github.io/recall` or custom domain) and the README demo GIF renders.

---

### Task 5: The 48-hour coordinated launch

Run all in one window (Tue–Thu, ~9am ET / 13:00 UTC). The goal is to clear GitHub Trending's ~200★/day velocity so Trending compounds the rest.

- [ ] **Hacker News (Show HN)** — title like "Show HN: Recall – teach your AI once, it codes like you in every repo and agent". Post early in the window; **be present in comments all day** (comments matter more than upvotes).
- [ ] **Reddit** — r/LocalLLaMA, r/commandline, r/rust, r/ChatGPTCoding. Lead with the problem; respect each sub's self-promo rules; one post each, native framing.
- [ ] **X/Twitter** — a thread with the 20-second demo GIF and the cross-agent "wow". Tag/notify relevant builders. Ask for honest feedback, not stars.
- [ ] **Console.dev** — submit (high-intent dev-tool newsletter; editorial features are free).
- [ ] **TLDR newsletter** — submit for same-week inclusion.
- [ ] **Bonus**: post a short Loom/YouTube of the cross-agent moment; submit to a dev-tool aggregator (DevHunt).

---

### Task 6: Post-launch (keep the flywheel)

- [ ] Respond to every HN/Reddit/issue comment within the first 48h; convert bug reports into quick fixes + a visible changelog.
- [ ] Track the truer signals (npm downloads, plugin installs, contributor retention), not just stars.
- [ ] Ship the **enforcement fast-follow** (Plan FF) as a second beat ("Recall now *blocks* edits that break your conventions") — a fresh, datable re-spike.
- [ ] Add a recurring re-spike hook: a shareable `recall export` "convention pack", or a public gallery of community packs.
- [ ] Write a short build-log blog post ("how we use Claude Code/Codex as a free local LLM provider") — compounding search tail + a second HN shot.

---

## Self-Review

**Spec coverage (architecture §8 launch plan; product spec §11):**
- README as a landing page (problem-first, demo GIF, one-command install, cross-agent story, comparison) → Task 1. ✅
- VHS demo GIF auto-regenerated in CI → Task 2. ✅
- LICENSE + repo metadata → Task 3. ✅
- Pre-launch directory listings → Task 4. ✅
- Coordinated 48h launch across the channels research identified → Task 5. ✅
- Post-launch flywheel + enforcement as second beat → Task 6. ✅

**Placeholder scan:** README uses only real, verifiable claims; the demo GIF path is produced in Task 2; no fabricated metrics.

**Honesty note:** Tasks 4–6 are operational and require the maintainer (accounts, posting identity, judgment on timing). They are a checklist, deliberately not autonomous.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-recall-plan6-launch.md`. Run Tasks 1–3 once Plans 1–5 are green and a release exists; run Tasks 4–6 with the maintainer for the actual launch.

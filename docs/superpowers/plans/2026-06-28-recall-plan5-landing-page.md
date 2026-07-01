# Recall Plan 5 — Landing Page

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A fast, beautiful single-page landing site at `apps/web` — dark terminal-flavored developer aesthetic, the cross-agent "wow" shown as a styled terminal sequence above the fold, copy-paste install, deployed to GitHub Pages on every push to `main`.

**Architecture:** Build out the `apps/web` placeholder from Plan 0 into real sections (Hero, Terminal demo, How-it-works, Features, Install, Footer) as small React 19 components styled with Tailwind v4 (`@theme` dark palette). A couple of Vitest render tests guard the key copy. A `deploy-web.yml` GitHub Pages workflow ships it (project-page `base`).

**Tech Stack:** Vite + React 19 + React Compiler + TailwindCSS v4 (`@tailwindcss/vite`), Vitest + Testing Library, Bun, GitHub Pages Actions.

## Global Constraints

- **Aesthetic:** near-black background (`#0a0a0b`), one green accent (`#4ade80`-ish), mono for code/terminal + grotesk for prose, generous spacing, subtle borders (`white/10`). No heavy frameworks; hand-rolled components.
- **Self-contained demo:** the hero "wow" is a styled terminal component with real text — it must look great with **no external GIF**. A VHS GIF (Plan 6) can augment later.
- **Build output:** `apps/web/dist`. `base` is `/recall/` for the GitHub Pages project page, overridable to `/` for a custom domain via `PAGES_BASE`.
- **Install commands shown must be exact:** `npx -y @tlgimenes/recall`, `brew install tlgimenes/recall/recall`, the curl installer, and the plugin-store commands for Claude Code + Codex.
- **Run with Bun:** `bun --filter './apps/web' <script>`.
- **Commit style:** end every commit body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

### Task 1: Design tokens + global styles

**Files:**
- Modify: `apps/web/src/index.css` (Tailwind v4 `@theme` dark palette + base)
- Modify: `apps/web/index.html` (title, meta description, fonts)

**Interfaces:**
- Produces: the dark theme tokens (`--color-bg`, `--color-accent`, font families) used by all components.

- [ ] **Step 1: Replace `apps/web/src/index.css`**

```css
@import "tailwindcss";

@theme {
  --color-bg: #0a0a0b;
  --color-surface: #121214;
  --color-border: #26262b;
  --color-fg: #ededee;
  --color-muted: #9a9aa3;
  --color-accent: #4ade80;
  --color-accent-dim: #22683f;
  --font-sans: "Inter", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, "SF Mono", monospace;
}

html {
  scroll-behavior: smooth;
}

body {
  background-color: var(--color-bg);
  color: var(--color-fg);
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
}
```

- [ ] **Step 2: Update `apps/web/index.html`** `<head>` (title, description, fonts)

```html
    <title>Recall — teach your AI once, it writes code like you everywhere</title>
    <meta
      name="description"
      content="Recall is a personal coding-convention brain for AI agents. Correct your AI once — it remembers and applies it in every repo, branch, and agent (Claude Code, Codex)."
    />
    <meta property="og:title" content="Recall — the convention brain for coding agents" />
    <meta property="og:description" content="Teach your AI once. It writes code like you in every repo and every agent." />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link
      href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap"
      rel="stylesheet"
    />
```

- [ ] **Step 3: Verify build still works**

Run: `bun --filter './apps/web' build`
Expected: builds cleanly.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/index.css apps/web/index.html
git commit -m "feat(web): dark terminal design tokens + page metadata

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: `CopyButton` + `Terminal` demo components

**Files:**
- Create: `apps/web/src/components/CopyButton.tsx`
- Create: `apps/web/src/components/Terminal.tsx`
- Test: `apps/web/src/components/Terminal.test.tsx` (added in Task 5 once Vitest is set up)

**Interfaces:**
- Produces: `<CopyButton text={...} />` and `<Terminal />` (the self-contained cross-agent "wow").

- [ ] **Step 1: Create `apps/web/src/components/CopyButton.tsx`**

```tsx
import { useState } from "react";

export function CopyButton({ text, label }: { text: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      onClick={() => {
        navigator.clipboard?.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
      className="rounded-md border border-[var(--color-border)] px-2 py-1 text-xs text-[var(--color-muted)] transition hover:border-[var(--color-accent-dim)] hover:text-[var(--color-accent)]"
      aria-label={label ?? "Copy to clipboard"}
    >
      {copied ? "copied" : "copy"}
    </button>
  );
}
```

- [ ] **Step 2: Create `apps/web/src/components/Terminal.tsx`** (the cross-agent story, self-contained)

```tsx
type Line = { kind: "cmd" | "out" | "dim" | "accent"; text: string };

const SESSION_A: Line[] = [
  { kind: "dim", text: "# repo: acme/api · agent: Claude Code" },
  { kind: "cmd", text: "you: stop creating barrel files — import directly" },
  { kind: "accent", text: "recall ✓ learned: \"Import directly; no barrel files\" (global)" },
];

const SESSION_B: Line[] = [
  { kind: "dim", text: "# a week later · repo: acme/web · agent: Codex" },
  { kind: "cmd", text: "you: add a users service" },
  { kind: "out", text: "codex: created users.service.ts, imported directly from ./user" },
  { kind: "accent", text: "↳ already follows your convention — you never said a word" },
];

function Block({ title, lines }: { title: string; lines: Line[] }) {
  const color: Record<Line["kind"], string> = {
    cmd: "text-[var(--color-fg)]",
    out: "text-[var(--color-muted)]",
    dim: "text-[var(--color-muted)]/60",
    accent: "text-[var(--color-accent)]",
  };
  return (
    <div className="flex-1 overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
      <div className="flex items-center gap-1.5 border-b border-[var(--color-border)] px-4 py-2.5">
        <span className="h-3 w-3 rounded-full bg-red-500/70" />
        <span className="h-3 w-3 rounded-full bg-yellow-500/70" />
        <span className="h-3 w-3 rounded-full bg-green-500/70" />
        <span className="ml-2 text-xs text-[var(--color-muted)]">{title}</span>
      </div>
      <pre className="overflow-x-auto p-4 font-mono text-sm leading-relaxed">
        {lines.map((l, i) => (
          <div key={i} className={color[l.kind]}>
            {l.text}
          </div>
        ))}
      </pre>
    </div>
  );
}

export function Terminal() {
  return (
    <div className="flex flex-col gap-4 md:flex-row">
      <Block title="session 1" lines={SESSION_A} />
      <Block title="session 2" lines={SESSION_B} />
    </div>
  );
}
```

- [ ] **Step 3: Verify build**

Run: `bun --filter './apps/web' build`
Expected: builds (components unused until Task 3 — that's fine, Vite tree-shakes).

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/components
git commit -m "feat(web): CopyButton + self-contained cross-agent Terminal demo

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Page sections (Hero, HowItWorks, Features, Install, Footer)

**Files:**
- Create: `apps/web/src/components/Hero.tsx`
- Create: `apps/web/src/components/HowItWorks.tsx`
- Create: `apps/web/src/components/Features.tsx`
- Create: `apps/web/src/components/Install.tsx`
- Create: `apps/web/src/components/Footer.tsx`
- Modify: `apps/web/src/App.tsx` (compose them)

**Interfaces:**
- Produces: the full page. Consumes `CopyButton`, `Terminal`.

- [ ] **Step 1: Create `apps/web/src/components/Hero.tsx`**

```tsx
import { CopyButton } from "./CopyButton";
import { Terminal } from "./Terminal";

const GH = "https://github.com/tlgimenes/recall";
const INSTALL = "npx -y @tlgimenes/recall";

export function Hero() {
  return (
    <header className="mx-auto max-w-6xl px-6 pt-20 pb-12 md:pt-28">
      <div className="mx-auto max-w-3xl text-center">
        <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--color-border)] px-3 py-1 font-mono text-xs text-[var(--color-muted)]">
          <span className="text-[var(--color-accent)]">●</span> local-first · MCP · Rust
        </div>
        <h1 className="text-balance text-4xl font-bold tracking-tight md:text-6xl">
          Teach your AI once.
          <br />
          It writes code <span className="text-[var(--color-accent)]">like you</span> everywhere.
        </h1>
        <p className="mx-auto mt-6 max-w-2xl text-balance text-lg text-[var(--color-muted)]">
          Recall is a personal convention brain for coding agents. Correct your AI
          once — it remembers and applies it in every repo, every branch, and every
          agent. Claude Code, Codex, and beyond.
        </p>
        <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <div className="flex items-center gap-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] px-4 py-2.5 font-mono text-sm">
            <span className="text-[var(--color-muted)]">$</span>
            <span>{INSTALL}</span>
            <CopyButton text={INSTALL} />
          </div>
          <a
            href={GH}
            className="rounded-lg bg-[var(--color-accent)] px-4 py-2.5 text-sm font-semibold text-black transition hover:opacity-90"
          >
            Star on GitHub →
          </a>
        </div>
      </div>
      <div className="mt-14">
        <Terminal />
      </div>
    </header>
  );
}
```

- [ ] **Step 2: Create `apps/web/src/components/HowItWorks.tsx`**

```tsx
const STEPS = [
  { n: "1", t: "Correct it once", d: "Tell any agent how you like things — or just work, and Recall distills your conventions from the session." },
  { n: "2", t: "Recall remembers", d: "Curated, compact rules — scoped to you, a language, a repo, or a branch. Stale rules are superseded, never piled up." },
  { n: "3", t: "Applied everywhere", d: "Every new session in every repo and every agent starts already knowing your conventions. No copy-pasting CLAUDE.md." },
];

export function HowItWorks() {
  return (
    <section className="mx-auto max-w-6xl px-6 py-20">
      <h2 className="text-center text-2xl font-bold tracking-tight md:text-3xl">How it works</h2>
      <div className="mt-10 grid gap-6 md:grid-cols-3">
        {STEPS.map((s) => (
          <div key={s.n} className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-6">
            <div className="font-mono text-sm text-[var(--color-accent)]">{s.n}</div>
            <h3 className="mt-2 text-lg font-semibold">{s.t}</h3>
            <p className="mt-2 text-sm text-[var(--color-muted)]">{s.d}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
```

- [ ] **Step 3: Create `apps/web/src/components/Features.tsx`**

```tsx
const FEATURES = [
  { t: "Cross-agent", d: "One brain for Claude Code and Codex. Your conventions don't live trapped in one tool." },
  { t: "Cross-repo & cross-branch", d: "Scoped to you, not the project. Global style follows you; repo/branch rules stay put." },
  { t: "Curated, not logged", d: "Compact, imperative rules — not a dump of everything you ever did." },
  { t: "Local-first", d: "A single fast Rust binary + SQLite on your machine. Inspectable and editable." },
  { t: "Uses your own agent", d: "No extra model to run or pay for — Recall distills via the Claude Code / Codex you already have." },
  { t: "Enforced (soon)", d: "Opt-in PreToolUse gating blocks edits that violate a convention, not just reminds." },
];

export function Features() {
  return (
    <section className="mx-auto max-w-6xl px-6 py-20">
      <h2 className="text-center text-2xl font-bold tracking-tight md:text-3xl">
        The convention brain nobody else ships
      </h2>
      <div className="mt-10 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
        {FEATURES.map((f) => (
          <div key={f.t} className="rounded-xl border border-[var(--color-border)] p-6">
            <h3 className="text-base font-semibold text-[var(--color-accent)]">{f.t}</h3>
            <p className="mt-2 text-sm text-[var(--color-muted)]">{f.d}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
```

- [ ] **Step 4: Create `apps/web/src/components/Install.tsx`**

```tsx
import { CopyButton } from "./CopyButton";

const ROWS: { label: string; cmd: string }[] = [
  { label: "Claude Code", cmd: "/plugin marketplace add tlgimenes/recall" },
  { label: "…then", cmd: "/plugin install recall@recall" },
  { label: "Codex", cmd: "codex plugin marketplace add tlgimenes/recall" },
  { label: "npm / npx", cmd: "npx -y @tlgimenes/recall" },
  { label: "Homebrew", cmd: "brew install tlgimenes/recall/recall" },
  { label: "curl", cmd: "curl -fsSL https://github.com/tlgimenes/recall/releases/latest/download/recall-installer.sh | sh" },
];

export function Install() {
  return (
    <section id="install" className="mx-auto max-w-3xl px-6 py-20">
      <h2 className="text-center text-2xl font-bold tracking-tight md:text-3xl">Install</h2>
      <p className="mt-3 text-center text-sm text-[var(--color-muted)]">
        Install the plugin in your agent — the MCP server runs via npx, no separate setup.
      </p>
      <div className="mt-8 divide-y divide-[var(--color-border)] overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
        {ROWS.map((r) => (
          <div key={r.cmd} className="flex items-center gap-3 px-4 py-3">
            <span className="w-28 shrink-0 text-xs text-[var(--color-muted)]">{r.label}</span>
            <code className="flex-1 overflow-x-auto font-mono text-sm">{r.cmd}</code>
            <CopyButton text={r.cmd} />
          </div>
        ))}
      </div>
    </section>
  );
}
```

- [ ] **Step 5: Create `apps/web/src/components/Footer.tsx`**

```tsx
export function Footer() {
  return (
    <footer className="border-t border-[var(--color-border)]">
      <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-6 py-10 text-sm text-[var(--color-muted)] sm:flex-row">
        <span className="font-mono">recall</span>
        <nav className="flex gap-6">
          <a href="https://github.com/tlgimenes/recall" className="hover:text-[var(--color-fg)]">GitHub</a>
          <a href="https://github.com/tlgimenes/recall#readme" className="hover:text-[var(--color-fg)]">Docs</a>
          <a href="#install" className="hover:text-[var(--color-fg)]">Install</a>
        </nav>
        <span>MIT · built in Rust</span>
      </div>
    </footer>
  );
}
```

- [ ] **Step 6: Replace `apps/web/src/App.tsx`**

```tsx
import { Hero } from "./components/Hero";
import { HowItWorks } from "./components/HowItWorks";
import { Features } from "./components/Features";
import { Install } from "./components/Install";
import { Footer } from "./components/Footer";

export default function App() {
  return (
    <div className="min-h-screen">
      <Hero />
      <HowItWorks />
      <Features />
      <Install />
      <Footer />
    </div>
  );
}
```

- [ ] **Step 7: Build + eyeball in dev**

Run: `bun --filter './apps/web' build` then `bun --filter './apps/web' dev` and open the local URL.
Expected: full dark landing page renders; the two-session Terminal demo shows the cross-agent story; copy buttons work.

- [ ] **Step 8: Commit**

```bash
git add apps/web/src
git commit -m "feat(web): hero, how-it-works, features, install, footer sections

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Vitest + render tests

**Files:**
- Modify: `apps/web/package.json` (add `vitest`, `@testing-library/react`, `jsdom`, `@testing-library/jest-dom`, `test` script)
- Create: `apps/web/vitest.config.ts`
- Create: `apps/web/src/test-setup.ts`
- Create: `apps/web/src/App.test.tsx`

**Interfaces:**
- Produces: `bun --filter './apps/web' test` guarding the hero copy + an install command.

- [ ] **Step 1: Add dev deps + script to `apps/web/package.json`**

Add to `devDependencies`:
```json
    "vitest": "^2.0.0",
    "jsdom": "^25.0.0",
    "@testing-library/react": "^16.0.0",
    "@testing-library/jest-dom": "^6.0.0"
```
Add to `scripts`:
```json
    "test": "vitest run"
```

- [ ] **Step 2: Create `apps/web/vitest.config.ts`**

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
  },
});
```

- [ ] **Step 3: Create `apps/web/src/test-setup.ts`**

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 4: Create `apps/web/src/App.test.tsx`**

```tsx
import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "./App";

test("hero states the value proposition", () => {
  render(<App />);
  expect(screen.getByText(/Teach your AI once/i)).toBeInTheDocument();
});

test("shows the npx install command", () => {
  render(<App />);
  expect(screen.getAllByText(/npx -y @tlgimenes\/recall/).length).toBeGreaterThan(0);
});

test("tells the cross-agent story", () => {
  render(<App />);
  expect(screen.getByText(/already follows your convention/i)).toBeInTheDocument();
});
```

- [ ] **Step 5: Install + run tests**

Run: `bun install && bun --filter './apps/web' test`
Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add apps/web/package.json apps/web/vitest.config.ts apps/web/src/test-setup.ts apps/web/src/App.test.tsx bun.lock
git commit -m "test(web): Vitest render tests for hero/install/cross-agent story

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: GitHub Pages deploy workflow

**Files:**
- Create: `.github/workflows/deploy-web.yml`
- Modify: `.github/workflows/ci.yml` (add `bun --filter './apps/web' test` to the web job)

**Interfaces:**
- Produces: auto-deploy of `apps/web` to GitHub Pages on push to `main`.

- [ ] **Step 1: Create `.github/workflows/deploy-web.yml`**

```yaml
name: Deploy web

on:
  push:
    branches: [main]
    paths:
      - "apps/web/**"
      - "bun.lock"
      - ".github/workflows/deploy-web.yml"
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
        with:
          bun-version: latest
      - run: bun install --frozen-lockfile
      - name: Build (project-page base)
        run: bun --filter './apps/web' build
        env:
          PAGES_BASE: /recall/
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with:
          path: apps/web/dist

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 2: Add the web test step to `.github/workflows/ci.yml`** (after "Build web")

```yaml
      - name: Test web
        run: bun --filter './apps/web' test
```

- [ ] **Step 3: Validate YAML**

Run: `python3 -c "import yaml; [yaml.safe_load(open(p)) for p in ['.github/workflows/deploy-web.yml','.github/workflows/ci.yml']]; print('workflows OK')"`
Expected: `workflows OK`.

- [ ] **Step 4: One-time GitHub setup** (manual, record in PR): repo **Settings → Pages → Source = "GitHub Actions"**. Site will be at `https://tlgimenes.github.io/recall/`. (For a custom domain later: set it in Settings → Pages and build with `PAGES_BASE=/`.)

- [ ] **Step 5: Commit + push, confirm deploy**

```bash
git add .github/workflows/deploy-web.yml .github/workflows/ci.yml
git commit -m "ci(web): deploy apps/web to GitHub Pages

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
git push
```
Expected: the Deploy-web workflow runs and publishes; the page loads at the Pages URL with assets resolving under `/recall/`.

---

## Self-Review

**Spec coverage (architecture §7 landing page):**
- Vite + React 19 + React Compiler + Tailwind v4 → built on Plan 0's scaffold. ✅
- Dark terminal aesthetic, self-contained "wow" demo → Tasks 1–3. ✅
- Hero one-liner + copy-paste install + cross-agent story + features + install matrix → Task 3. ✅
- GitHub Pages deploy, `base` env-driven, path-filtered → Task 5. ✅
- Tests guarding key copy → Task 4. ✅
- VHS demo GIF → augmentation in Plan 6 (the Terminal component stands alone meanwhile). ✅ (noted, not silent)
- Vite+ vs plain Vite: Plan uses plain Vite + Vitest (Oxc-compatible); a Vite+ swap is transparent and optional. ✅

**Placeholder scan:** No TBD/TODO. All component code is complete and renders without external assets.

**Type consistency:** Component prop shapes (`CopyButton text`, `Terminal` lines) are internally consistent; install commands match Plan 3's plugin names and Plan 4's package/tap names exactly (`@tlgimenes/recall`, `tlgimenes/recall/recall`, `recall@recall`).

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-recall-plan5-landing-page.md`. Execute after Plan 0 (independent of Plans 1–4).

# Recall Plan 4 — Release Pipeline (binaries, Homebrew, npm)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One `git tag` cuts a release: cross-platform `recall` binaries on a GitHub Release, a `curl | sh` installer, a Homebrew formula, and the `@tlgimenes/recall` npm package the plugins invoke via `npx`.

**Architecture:** **cargo-dist** owns the build matrix → GitHub Release + shell installer + Homebrew tap. The npm package is **hand-wired Biome-style** (a launcher package `@tlgimenes/recall` with per-platform `optionalDependencies` + a JS bin shim), generated from the release binaries by a workflow job — giving exact control of the package name and `bin` so `npx -y @tlgimenes/recall mcp` works. **crates.io is deferred** (taken names; primary install is npx/brew/curl).

**Tech Stack:** cargo-dist (`dist` v0.32), GitHub Actions, Bun (to build/publish the npm packages), Node-compatible bin shim.

## Global Constraints

- **npm package name:** exactly `@tlgimenes/recall`; bin name `recall`. Per-platform packages: `@tlgimenes/recall-<os>-<arch>` (+ `-musl` for static Linux).
- **GATED ON NPM ACCOUNT RECOVERY:** do not run the npm publish job (Task 4) or set `NPM_TOKEN` until the maintainer confirms the npm account is recovered. Everything else (binaries, Homebrew, shell installer) can ship first.
- **crates.io:** not used in v1. (Revisit to publish the reusable `agent-cli` crate later, under a free name — `agent-cli` is taken.)
- **Targets:** `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`.
- **cargo-dist caveat:** axodotdev wound down; v0.32 still works. If it stalls, the `astral-sh/cargo-dist` fork (now under OpenAI) is the fallback. Pin `cargo-dist-version` so CI is reproducible.
- **Secrets (GitHub → Settings → Secrets):** `HOMEBREW_TAP_TOKEN` (PAT, `repo` scope) now; `NPM_TOKEN` (granular, publish to `@tlgimenes/*`) only after account recovery. `GITHUB_TOKEN` (default) suffices for the Release itself.
- **Commit style:** end every commit body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

### Task 1: cargo-dist — binary matrix + GitHub Release + shell installer

**Files:**
- Create: `dist-workspace.toml`
- Create (generated): `.github/workflows/release.yml`
- Modify: `crates/recall-cli/Cargo.toml` (release profile metadata if `dist init` adds it)

**Interfaces:**
- Produces: a tag-triggered workflow that builds `recall` for all targets and publishes a GitHub Release + shell installer.

- [ ] **Step 1: Install dist** (the executor machine)

Run: `cargo install cargo-dist --version 0.32.0` (or `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/axodotdev/cargo-dist/releases/download/v0.32.0/cargo-dist-installer.sh | sh`)
Expected: `dist --version` → `dist 0.32.0`.

- [ ] **Step 2: Create `dist-workspace.toml`**

```toml
[workspace]
members = ["cargo:*"]

[dist]
cargo-dist-version = "0.32.0"
ci = ["github"]
# npm is hand-wired in Task 3/4, so it's intentionally NOT in this installers list.
installers = ["shell", "homebrew"]
targets = [
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
]
tap = "tlgimenes/homebrew-recall"
publish-jobs = ["homebrew"]
install-path = "CARGO_HOME"
# Only ship the `recall` binary (internal crates are libs with no bins).
```

- [ ] **Step 3: Generate the release workflow**

Run: `dist generate`
Expected: creates `.github/workflows/release.yml` and may adjust `Cargo.toml` profiles. Review the diff.

- [ ] **Step 4: Verify the plan locally (no publish)**

Run: `dist plan`
Expected: JSON listing the 5 target artifacts + shell + homebrew installers, app name `recall`.

- [ ] **Step 5: Validate the generated workflow YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('release.yml OK')"`
Expected: `release.yml OK`.

- [ ] **Step 6: Commit**

```bash
git add dist-workspace.toml .github/workflows/release.yml Cargo.toml Cargo.lock
git commit -m "ci(release): cargo-dist binary matrix + GitHub Release + shell + homebrew

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Homebrew tap

**Files:** (external repo + a secret; no files in this repo)

**Interfaces:**
- Produces: a `tlgimenes/homebrew-recall` repo cargo-dist can push the formula to.

- [ ] **Step 1: Create the tap repo**

Run: `gh repo create tlgimenes/homebrew-recall --public -d "Homebrew tap for Recall" --add-readme`
Expected: repo created.

- [ ] **Step 2: Add the tap token secret**

Create a PAT with `repo` scope, then:
Run: `gh secret set HOMEBREW_TAP_TOKEN --repo tlgimenes/recall` (paste the PAT)
Expected: secret stored. (cargo-dist's homebrew publish-job commits the formula to the tap using this.)

- [ ] **Step 3: Document the install** — confirmed working after Task 5's release:

```bash
brew install tlgimenes/recall/recall
```

(No commit — external repo + secret only. Record completion in the PR description.)

---

### Task 3: Hand-wired `@tlgimenes/recall` npm launcher

**Files:**
- Create: `packages/recall/package.json` (the launcher)
- Create: `packages/recall/bin/recall` (JS shim)
- Create: `packages/recall/README.md`
- Create: `packages/recall/scripts/generate-packages.mjs` (builds per-platform packages from release binaries)
- Modify: root `package.json` (the launcher is already in the `packages/*` workspace glob)

**Interfaces:**
- Produces: the launcher package + a script that, given downloaded release binaries, emits `@tlgimenes/recall-<os>-<arch>` packages and stamps versions. Consumed by Task 4's workflow.

- [ ] **Step 1: Create `packages/recall/package.json`**

```json
{
  "name": "@tlgimenes/recall",
  "version": "0.0.0",
  "description": "Teach your AI once — it writes code like you in every repo and every agent. Personal coding-convention brain (MCP server + CLI).",
  "bin": { "recall": "bin/recall" },
  "files": ["bin"],
  "keywords": ["mcp", "claude", "codex", "ai", "memory", "conventions"],
  "homepage": "https://github.com/tlgimenes/recall",
  "repository": { "type": "git", "url": "https://github.com/tlgimenes/recall.git" },
  "license": "MIT",
  "optionalDependencies": {
    "@tlgimenes/recall-darwin-arm64": "0.0.0",
    "@tlgimenes/recall-darwin-x64": "0.0.0",
    "@tlgimenes/recall-linux-x64": "0.0.0",
    "@tlgimenes/recall-linux-arm64": "0.0.0",
    "@tlgimenes/recall-win32-x64": "0.0.0"
  }
}
```

(Versions are rewritten to the release version by `generate-packages.mjs`.)

- [ ] **Step 2: Create `packages/recall/bin/recall` (the shim)**

```javascript
#!/usr/bin/env node
const { spawnSync } = require("node:child_process");

const PKGS = {
  "darwin-arm64": "@tlgimenes/recall-darwin-arm64/bin/recall",
  "darwin-x64": "@tlgimenes/recall-darwin-x64/bin/recall",
  "linux-x64": "@tlgimenes/recall-linux-x64/bin/recall",
  "linux-arm64": "@tlgimenes/recall-linux-arm64/bin/recall",
  "win32-x64": "@tlgimenes/recall-win32-x64/bin/recall.exe",
};

const key = `${process.platform}-${process.arch}`;
const sub = process.env.RECALL_BINARY ?? PKGS[key];
if (!sub) {
  console.error(`recall: unsupported platform ${key}`);
  process.exit(1);
}

let bin;
try {
  bin = process.env.RECALL_BINARY ?? require.resolve(sub);
} catch {
  console.error(
    `recall: could not find the native binary for ${key}. ` +
      `Reinstall, or download from https://github.com/tlgimenes/recall/releases`,
  );
  process.exit(1);
}

const r = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (r.error) {
  console.error(r.error.message);
  process.exit(1);
}
process.exit(r.status ?? 1);
```

- [ ] **Step 3: Create `packages/recall/README.md`**

```markdown
# @tlgimenes/recall

Teach your AI once — it writes code like you in every repo and every agent.

```bash
npx -y @tlgimenes/recall mcp     # run the MCP server
npm i -g @tlgimenes/recall       # install the `recall` CLI
```

See https://github.com/tlgimenes/recall.
```

- [ ] **Step 4: Create `packages/recall/scripts/generate-packages.mjs`**

```javascript
// Build per-platform npm packages from downloaded release binaries.
// Usage: node generate-packages.mjs <version> <binaries-dir> <out-dir>
// <binaries-dir> contains: recall-<target>(.exe). Mapping below.
import { mkdirSync, writeFileSync, copyFileSync, readFileSync } from "node:fs";
import { join } from "node:path";

const [version, binDir, outDir] = process.argv.slice(2);
if (!version || !binDir || !outDir) {
  console.error("usage: generate-packages.mjs <version> <bin-dir> <out-dir>");
  process.exit(1);
}

// npm key -> { rustTarget, os, cpu, exe }
const TARGETS = {
  "darwin-arm64": { t: "aarch64-apple-darwin", os: "darwin", cpu: "arm64", exe: "recall" },
  "darwin-x64": { t: "x86_64-apple-darwin", os: "darwin", cpu: "x64", exe: "recall" },
  "linux-x64": { t: "x86_64-unknown-linux-gnu", os: "linux", cpu: "x64", exe: "recall" },
  "linux-arm64": { t: "aarch64-unknown-linux-gnu", os: "linux", cpu: "arm64", exe: "recall" },
  "win32-x64": { t: "x86_64-pc-windows-msvc", os: "win32", cpu: "x64", exe: "recall.exe" },
};

for (const [key, m] of Object.entries(TARGETS)) {
  const pkgDir = join(outDir, `recall-${key}`);
  mkdirSync(join(pkgDir, "bin"), { recursive: true });
  copyFileSync(join(binDir, `recall-${m.t}${m.exe.endsWith(".exe") ? ".exe" : ""}`),
               join(pkgDir, "bin", m.exe));
  writeFileSync(join(pkgDir, "package.json"), JSON.stringify({
    name: `@tlgimenes/recall-${key}`,
    version,
    os: [m.os],
    cpu: [m.cpu],
    files: ["bin"],
    license: "MIT",
  }, null, 2));
}

// Stamp the launcher version + optionalDependencies versions.
const launcherPath = join(outDir, "recall", "package.json");
const launcher = JSON.parse(readFileSync(launcherPath, "utf8"));
launcher.version = version;
for (const dep of Object.keys(launcher.optionalDependencies)) {
  launcher.optionalDependencies[dep] = version;
}
writeFileSync(launcherPath, JSON.stringify(launcher, null, 2));
console.log(`stamped ${version} across launcher + ${Object.keys(TARGETS).length} platform packages`);
```

> Note: the exact release-asset file names (`recall-<target>` vs an archive) depend on cargo-dist's artifact naming; Task 4 downloads + unpacks them into `<bin-dir>` with these names. Verify against `dist plan` output at execution time and adjust the copy step if cargo-dist ships tarballs (then untar first).

- [ ] **Step 5: Validate JS + shim syntax**

Run: `node --check packages/recall/bin/recall && node --check packages/recall/scripts/generate-packages.mjs && echo "shim+script OK"`
Expected: `shim+script OK`.

- [ ] **Step 6: Commit**

```bash
git add packages/recall
git commit -m "feat(npm): hand-wired @tlgimenes/recall launcher (optionalDependencies + shim)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: npm publish workflow (GATED on account recovery)

**Files:**
- Create: `.github/workflows/release-npm.yml`

**Interfaces:**
- Produces: a workflow that, after a release publishes binaries, builds the per-platform npm packages and publishes them + the launcher.

> **DO NOT enable/run until the maintainer confirms the npm account is recovered and `NPM_TOKEN` is set.** Until then, the binary/Homebrew/shell channels (Tasks 1–2) are the release; the plugins can be dogfooded against the local binary (Plan 3 Task 5).

- [ ] **Step 1: Create `.github/workflows/release-npm.yml`**

```yaml
name: Publish npm

on:
  release:
    types: [published]
  workflow_dispatch:
    inputs:
      tag:
        description: "Release tag to publish (e.g. v0.1.0)"
        required: true

permissions:
  contents: read

jobs:
  npm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
        with:
          bun-version: latest

      - name: Resolve tag + version
        id: v
        run: |
          TAG="${{ github.event.release.tag_name || inputs.tag }}"
          echo "tag=$TAG" >> "$GITHUB_OUTPUT"
          echo "version=${TAG#v}" >> "$GITHUB_OUTPUT"

      - name: Download release binaries
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          mkdir -p dist-bin staged
          cp -R packages/recall staged/recall
          # Download cargo-dist archives, unpack, normalize to recall-<target>(.exe) in dist-bin/
          gh release download "${{ steps.v.outputs.tag }}" -R tlgimenes/recall -p "*.tar.xz" -p "*.zip" -D archives
          for f in archives/*; do
            case "$f" in
              *.tar.xz) tar -xJf "$f" -C archives ;;
              *.zip)    unzip -o "$f" -d archives ;;
            esac
          done
          # Each unpacked dir is named like recall-<target>/ containing the binary.
          for d in archives/recall-*/; do
            tgt="$(basename "$d" | sed 's/^recall-//')"
            if [ -f "$d/recall.exe" ]; then cp "$d/recall.exe" "dist-bin/recall-$tgt.exe";
            else cp "$d/recall" "dist-bin/recall-$tgt"; fi
          done
          ls -la dist-bin

      - name: Generate platform packages
        run: node packages/recall/scripts/generate-packages.mjs "${{ steps.v.outputs.version }}" dist-bin staged

      - name: Publish to npm
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
        run: |
          echo "//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}" > ~/.npmrc
          for d in staged/recall-*/; do (cd "$d" && npm publish --access public); done
          (cd staged/recall && npm publish --access public)
```

> The archive/unpack step assumes cargo-dist's default tarball layout (`recall-<target>/recall`). Confirm names from a real release (or `dist plan`) and adjust the `sed`/paths if needed — this is the one spot to verify hands-on.

- [ ] **Step 2: Validate the workflow YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-npm.yml')); print('release-npm.yml OK')"`
Expected: `release-npm.yml OK`.

- [ ] **Step 3: Commit (workflow only; no publish yet)**

```bash
git add .github/workflows/release-npm.yml
git commit -m "ci(release): npm publish workflow (gated on NPM_TOKEN)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Cut the first release + verify install channels

**Files:** (none — process)

- [ ] **Step 1: Set the version** — ensure `crates/recall-cli/Cargo.toml` `version` is `0.1.0` (and any other crate versions intended to ship).

- [ ] **Step 2: Tag and push**

```bash
git tag v0.1.0
git push origin v0.1.0
```
Expected: `release.yml` runs the matrix, creates the GitHub Release with binaries + shell installer, and pushes the Homebrew formula to the tap.

- [ ] **Step 3: Verify the non-npm channels**

```bash
curl -fsSL https://github.com/tlgimenes/recall/releases/download/v0.1.0/recall-installer.sh | sh
recall --version
brew install tlgimenes/recall/recall && recall --version
```
Expected: both install a working `recall`.

- [ ] **Step 4: (AFTER npm recovery) publish npm + verify**

Set `NPM_TOKEN`, then run the `release-npm.yml` workflow (`gh workflow run release-npm.yml -f tag=v0.1.0`). Verify:
```bash
npx -y @tlgimenes/recall --version
echo '{}' | npx -y @tlgimenes/recall mcp   # MCP server starts (Ctrl-C to exit)
```
Expected: the package runs the binary; the plugins' `npx -y @tlgimenes/recall mcp` now resolves for end users.

- [ ] **Step 5: Record completion** in the PR/release notes (which channels are live; npm pending or done).

---

## Self-Review

**Spec coverage (architecture §5 distribution):**
- Cross-platform binary matrix + GitHub Release + shell installer + Homebrew → Tasks 1, 2, 5. ✅
- `@tlgimenes/recall` npm package with exact name/bin for `npx` → Tasks 3, 4 (Biome-style, hand-wired for name control). ✅
- crates.io deferred (documented rationale: taken names; npx/brew/curl cover install) → constraints. ✅
- npm gated on account recovery → Task 4 banner + Task 5 Step 4. ✅
- cargo-dist maintenance flag + pinned version → constraints. ✅
- Secrets enumerated (`HOMEBREW_TAP_TOKEN` now, `NPM_TOKEN` later) → constraints, Task 2. ✅

**Placeholder scan:** No TBD/TODO. Two explicit "verify against real cargo-dist artifact names" notes (Task 3 Step 4, Task 4) are honest external-tool caveats, not placeholders — the code is complete for the documented default layout.

**Type consistency:** npm package name `@tlgimenes/recall` and `bin: recall` match Plan 3's `npx -y @tlgimenes/recall` references exactly; per-platform package names match between `package.json optionalDependencies`, the shim's `PKGS` map, and `generate-packages.mjs` `TARGETS`.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-recall-plan4-release-pipeline.md`. Execute after Plans 0–3; **hold Task 4 + Task 5 Step 4 until npm account recovery**.

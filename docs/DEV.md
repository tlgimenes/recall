# Developing Recall by dogfooding it

Recall is built by using Recall. Once the binary builds, register the local
debug build as an MCP server on your own Claude Code / Codex session and call
its tools live.

## Build

```bash
cargo build
```

## Register on Claude Code (project scope)

The repo ships a dev `.mcp.json` pointing at `./target/debug/recall mcp`.
From the repo root:

```bash
claude mcp add recall -- ./target/debug/recall mcp
# or rely on the project-scoped .mcp.json and approve it when prompted
```

Then in a session: call the `recall_conventions` and `recall_list` tools.

## Teach + verify loop

```bash
# teach a convention (uses ~/.recall/recall.db by default)
./target/debug/recall learn "Import directly; no barrel files" --scope global

# confirm it's stored
./target/debug/recall list

# the MCP tool should now return it
#   recall_conventions  -> includes the rule
#   recall_list         -> includes the rule
```

Use a throwaway DB while experimenting:

```bash
RECALL_DB=/tmp/recall-dev.db ./target/debug/recall learn "..." 
RECALL_DB=/tmp/recall-dev.db ./target/debug/recall list
```

## Dogfooding the full plugin (before npm publish)

The shipped plugin runs `npx -y @tlgimenes/recall` (published in Plan 4). Until
then, dogfood against the local debug binary:

```bash
cargo build

# Option A: register just the MCP server at the local binary (fast loop)
claude mcp add recall -- ./target/debug/recall mcp

# Option B: install the whole plugin from the local marketplace, then
# temporarily point its MCP at the debug binary by copying the dev variant:
cp plugins/claude-code/.mcp.dev.json plugins/claude-code/.mcp.json   # local only; don't commit
claude plugin marketplace add .
claude plugin install recall@recall
```

For the hooks to use the local binary during dev, add to `~/.claude/settings.json`
(SessionStart/Stop) pointing `command` at `./target/debug/recall hook ...`, or
ensure `recall` is on PATH (`cargo install --path crates/recall-cli`).

Smoke test the loop:

```bash
./target/debug/recall learn "Import directly; no barrel files" --scope global
# new Claude Code session in this repo -> the SessionStart hook injects the rule
# tell Claude "always prefer early returns" -> it should call recall_learn
./target/debug/recall list   # confirm both rules are stored
```

## Enforcement mode

Recall checks edits against your conventions. Set the mode via `RECALL_ENFORCE`:

- `warn` (default) — adds a heads-up but allows the edit
- `block` — denies edits that violate a convention
- `off` — disables the check

```bash
export RECALL_ENFORCE=block   # in your shell / agent env
```

The check fails open (allows) on any provider error, so it never wedges a session.

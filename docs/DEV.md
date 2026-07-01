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

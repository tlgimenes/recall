# Recall

Recall is a personal coding-convention brain for AI coding agents.

You correct an agent once — "stop creating barrel files, import directly from
the module" — and Recall remembers it. The next time you, or any agent, touch
any repo, that convention is already there, injected automatically before the
agent's first action.

It is not a chat app, a repo-search tool, or a general memory service. It does
one thing: capture the coding conventions you actually care about, scope them
correctly, and hand them back to whichever agent you're using, wherever you're
working.

## How it works

- **Learn.** Conventions come from two sources: explicit manual teaching
  (`recall learn "<rule>" --scope <scope>`, or a `/recall-learn` slash command
  in-agent) and, optionally, automatic session distillation — when a supported
  LLM provider (your existing Claude Code / Codex login, or your own API key)
  is available, Recall can extract durable conventions from a session
  transcript. Manual teaching always works, with no provider required.
- **Scope.** Every convention is scoped — global (applies everywhere), by
  language, by repo, or by branch — with more specific scopes taking
  precedence. New conventions can supersede stale ones instead of piling up
  forever.
- **Inject.** A `SessionStart` hook and an MCP tool
  (`recall_conventions`/`recall_list`) surface only the relevant, active
  conventions for the current repo/branch/language, so agents see them before
  they start working — not a giant static file that grows stale.
- **Enforce (opt-in).** A `PreToolUse` hook can check edits against your
  conventions and warn or block violations (`RECALL_ENFORCE=warn|block|off`).
  It fails open on any error, so it never wedges a session.

Recall is local-first: a single Rust binary, backed by SQLite at
`~/.recall/recall.db`, with no cloud sync and no data leaving your machine
except to whichever LLM provider you've configured for distillation (optional,
and off by default without one).

It's designed to work the same way across agents — Claude Code and Codex
today, more to follow — so a convention you teach in one agent applies in the
others too.

## Status

Recall is under active development. The first tagged release,
[v0.1.0](https://github.com/tlgimenes/recall/releases/tag/v0.1.0), ships
prebuilt binaries for macOS, Linux, and Windows via a shell installer:

```sh
curl -fsSL https://github.com/tlgimenes/recall/releases/latest/download/recall-cli-installer.sh | sh
claude mcp add recall --scope user -- recall mcp
```

The npm package, Homebrew tap, and Claude Code / Codex plugin listings
aren't published yet. See [`docs/DEV.md`](docs/DEV.md) if you'd rather build
from source.

## License

MIT — see [`LICENSE`](LICENSE).

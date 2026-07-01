---
name: recall
description: Use to follow and record the developer's personal coding conventions. ALWAYS call recall_conventions before writing or editing code. Call recall_learn whenever the developer states a durable preference or corrects you ("always X", "never Y", "we use Z here").
---

# Recall — the developer's convention brain

Recall remembers how THIS developer likes code written, across every repo and
branch. Use it so your code matches their conventions without being told twice.

## When to call which tool

- **Before writing or editing code**, call `recall_conventions` to load the
  rules relevant to the current repo/branch/languages. Follow them.
- **When the developer states a durable preference or corrects you** — e.g.
  "always use early returns", "never add barrel files", "we use snake_case for
  files here" — call `recall_learn` with a compact imperative `rule` and the
  right `scope` (`global` for personal style, `repo`/`branch` for project rules,
  `language:<lang>` for language-specific ones).
- To show the developer everything Recall knows, call `recall_list`.

## Rules of thumb

- Keep each rule short and imperative (< 140 chars).
- Prefer `global` scope for personal style that should follow the developer
  everywhere; use `repo`/`branch` only for project-specific rules.
- Don't record one-off task details — only durable conventions.

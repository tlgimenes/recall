# Recall Plan FF — Enforcement (PreToolUse gating)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn memory into outcomes — an opt-in `PreToolUse` hook that checks a proposed edit against the developer's active conventions and either warns or **blocks** the edit when it violates one. This is the "convert recall into enforcement" feature that no competitor ships.

**Architecture:** A pure `recall-capture::enforce` module builds a check prompt/schema, runs the proposed file content + relevant conventions through the `AgentProvider`, and parses violations. The CLI adds `recall hook pre-tool-use`, which reads the PreToolUse stdin, extracts the proposed change, gets violations, and emits the portable `permissionDecision` payload according to the enforcement mode (`off` | `warn` | `block`, default `warn`). Both plugins register the PreToolUse hook.

**Tech Stack:** extends `recall-capture` + `recall-cli` (Plans 2–3); uses the hook `permissionDecision` contract (Claude Code + Codex identical).

## Global Constraints

- **Default is `warn`, not `block`** — blocking is opt-in via `RECALL_ENFORCE=block`. Never silently block on first install (bad first impression).
- **Latency budget:** the check is one bounded provider call; only run it for edit tools (`Edit`, `Write`, `MultiEdit`, `apply_patch`). Skip everything else (allow immediately). Hard-timeout; on any error, **fail open** (allow) — never wedge the user's session.
- **Decision payload (portable):** `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"..."}}` to block; `{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"..."}}` to warn-and-allow.
- **Commit style:** end every commit body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

### Task 1: `recall-capture::enforce` — violation check

**Files:**
- Create: `crates/recall-capture/src/enforce.rs`
- Modify: `crates/recall-capture/src/lib.rs` (add `mod enforce; pub use enforce::*;`)
- Test: inline in `crates/recall-capture/src/enforce.rs`

**Interfaces:**
- Produces: `struct Violation { rule: String, explanation: String }`; `check_schema() -> Value`; `check_prompt(content, conventions: &[Convention]) -> String`; `parse_violations(&Value) -> Result<Vec<Violation>>`; `async fn check(content: &str, conventions: &[Convention], provider: &dyn AgentProvider) -> Result<Vec<Violation>>`.

- [ ] **Step 1: Write the failing tests in `crates/recall-capture/src/enforce.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use agent_cli::MockProvider;
    use recall_core::*;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    fn conv(rule: &str) -> Convention {
        let now = Utc::now();
        Convention {
            id: Uuid::new_v4(), rule: rule.into(), rationale: None, scope: Scope::Global,
            tags: vec![], provenance: Provenance { source: Source::ManualTeach, repo: None,
            branch: None, agent: None, excerpt: None, learned_at: now },
            status: Status::Active, superseded_by: None, confidence: 0.9, created_at: now, updated_at: now,
        }
    }

    #[test]
    fn parse_reads_violations() {
        let v = json!({"violations":[{"rule":"No barrel files","explanation":"adds index.ts re-export"}]});
        let vs = parse_violations(&v).unwrap();
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].rule, "No barrel files");
    }

    #[test]
    fn prompt_includes_rules_and_content() {
        let p = check_prompt("export * from './x'", &[conv("No barrel files")]);
        assert!(p.contains("No barrel files"));
        assert!(p.contains("export * from"));
    }

    #[tokio::test]
    async fn check_runs_provider() {
        let provider = MockProvider::new(json!({"violations":[{"rule":"No barrel files","explanation":"re-export"}]}));
        let vs = check("export * from './x'", &[conv("No barrel files")], &provider).await.unwrap();
        assert_eq!(vs.len(), 1);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p recall-capture enforce`
Expected: FAIL — module not found.

- [ ] **Step 3: Add module wiring to `crates/recall-capture/src/lib.rs`**

```rust
mod enforce;
pub use enforce::*;
```

- [ ] **Step 4: Write `crates/recall-capture/src/enforce.rs`** (above the test block)

```rust
use agent_cli::AgentProvider;
use anyhow::{anyhow, Result};
use recall_core::Convention;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub rule: String,
    pub explanation: String,
}

pub fn check_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "violations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "rule": { "type": "string" },
                        "explanation": { "type": "string" }
                    },
                    "required": ["rule", "explanation"]
                }
            }
        },
        "required": ["violations"]
    })
}

pub fn check_prompt(content: &str, conventions: &[Convention]) -> String {
    let rules = conventions
        .iter()
        .map(|c| format!("- {}", c.rule))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Check whether the proposed code change violates any of the developer's \
         conventions. Only report a clear, concrete violation; if unsure, do not \
         report it. Return JSON.\n\n=== CONVENTIONS ===\n{rules}\n\n=== PROPOSED CHANGE ===\n{content}"
    )
}

pub fn parse_violations(v: &Value) -> Result<Vec<Violation>> {
    let arr = v.get("violations").and_then(|a| a.as_array())
        .ok_or_else(|| anyhow!("missing 'violations'"))?;
    Ok(arr.iter().filter_map(|item| {
        Some(Violation {
            rule: item.get("rule")?.as_str()?.to_string(),
            explanation: item.get("explanation")?.as_str()?.to_string(),
        })
    }).collect())
}

pub async fn check(
    content: &str,
    conventions: &[Convention],
    provider: &dyn AgentProvider,
) -> Result<Vec<Violation>> {
    if conventions.is_empty() {
        return Ok(vec![]);
    }
    let raw = provider.complete_json(&check_prompt(content, conventions), &check_schema()).await?;
    parse_violations(&raw)
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p recall-capture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/recall-capture
git commit -m "feat(enforce): provider-backed convention violation check

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: CLI — `recall hook pre-tool-use` + enforcement mode

**Files:**
- Modify: `crates/recall-cli/src/lib.rs` (add `EnforceMode`, `extract_proposed`, `pre_tool_use_decision`, `cmd_hook_pre_tool_use`)
- Modify: `crates/recall-cli/src/main.rs` (route `recall hook pre-tool-use`)
- Test: extend the `#[cfg(test)]` block in `crates/recall-cli/src/lib.rs`

**Interfaces:**
- Produces: `EnforceMode` (`Off|Warn|Block`, from `RECALL_ENFORCE`); `extract_proposed(tool_name, tool_input) -> Option<(Option<String> path, String content)>`; `pre_tool_use_decision(violations: &[Violation], mode: EnforceMode) -> Option<String>` (the JSON to print, or None); `cmd_hook_pre_tool_use(db, stdin_json, mode, provider) -> Result<Option<String>>`.

- [ ] **Step 1: Write the failing tests in the `#[cfg(test)]` block of `crates/recall-cli/src/lib.rs`**

```rust
    #[test]
    fn extract_proposed_handles_write_and_edit() {
        let (p, c) = extract_proposed("Write", &serde_json::json!({"file_path":"a.ts","content":"x"})).unwrap();
        assert_eq!(p.as_deref(), Some("a.ts"));
        assert_eq!(c, "x");
        let (_, c2) = extract_proposed("Edit", &serde_json::json!({"file_path":"a.ts","new_string":"y"})).unwrap();
        assert_eq!(c2, "y");
        assert!(extract_proposed("Bash", &serde_json::json!({"command":"ls"})).is_none());
    }

    #[test]
    fn decision_block_denies_when_violations() {
        use recall_capture::Violation;
        let v = vec![Violation { rule: "No barrels".into(), explanation: "re-export".into() }];
        let out = pre_tool_use_decision(&v, EnforceMode::Block).unwrap();
        assert!(out.contains("\"permissionDecision\":\"deny\""));
        assert!(out.contains("No barrels"));
    }

    #[test]
    fn decision_warn_allows_with_context() {
        use recall_capture::Violation;
        let v = vec![Violation { rule: "No barrels".into(), explanation: "re-export".into() }];
        let out = pre_tool_use_decision(&v, EnforceMode::Warn).unwrap();
        assert!(out.contains("additionalContext"));
        assert!(!out.contains("deny"));
    }

    #[test]
    fn decision_none_when_no_violations_or_off() {
        assert!(pre_tool_use_decision(&[], EnforceMode::Block).is_none());
        use recall_capture::Violation;
        let v = vec![Violation { rule: "x".into(), explanation: "y".into() }];
        assert!(pre_tool_use_decision(&v, EnforceMode::Off).is_none());
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p recall-cli pre_tool_use`
Expected: FAIL — items not found.

- [ ] **Step 3: Add the implementations to `crates/recall-cli/src/lib.rs`** (above the test block)

```rust
use recall_capture::{check, Violation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforceMode { Off, Warn, Block }

impl EnforceMode {
    pub fn from_env() -> Self {
        match std::env::var("RECALL_ENFORCE").as_deref() {
            Ok("block") => EnforceMode::Block,
            Ok("off") => EnforceMode::Off,
            _ => EnforceMode::Warn, // default
        }
    }
}

/// Pull the proposed file path + content from an edit tool's input. None for non-edit tools.
pub fn extract_proposed(tool_name: &str, tool_input: &Value) -> Option<(Option<String>, String)> {
    let is_edit = matches!(tool_name, "Write" | "Edit" | "MultiEdit" | "apply_patch");
    if !is_edit {
        return None;
    }
    let path = tool_input.get("file_path").and_then(|p| p.as_str()).map(String::from);
    let content = tool_input
        .get("content")
        .or_else(|| tool_input.get("new_string"))
        .or_else(|| tool_input.get("file_text"))
        .or_else(|| tool_input.get("input")) // apply_patch
        .and_then(|c| c.as_str())
        .map(String::from)?;
    Some((path, content))
}

/// Build the hook output JSON for the given violations + mode (None = stay silent / allow).
pub fn pre_tool_use_decision(violations: &[Violation], mode: EnforceMode) -> Option<String> {
    if violations.is_empty() || mode == EnforceMode::Off {
        return None;
    }
    let summary = violations
        .iter()
        .map(|v| format!("• {} — {}", v.rule, v.explanation))
        .collect::<Vec<_>>()
        .join("\n");
    let out = match mode {
        EnforceMode::Block => json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": format!("This edit violates your Recall conventions:\n{summary}")
            }
        }),
        EnforceMode::Warn => json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "additionalContext": format!("Heads up — this edit may violate your conventions:\n{summary}")
            }
        }),
        EnforceMode::Off => return None,
    };
    Some(out.to_string())
}

pub async fn cmd_hook_pre_tool_use(
    db: &Path,
    stdin_json: &str,
    mode: EnforceMode,
    provider: &dyn AgentProvider,
) -> Result<Option<String>> {
    if mode == EnforceMode::Off {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(stdin_json).unwrap_or(json!({}));
    let tool_name = v.get("tool_name").and_then(|t| t.as_str()).unwrap_or("");
    let tool_input = v.get("tool_input").cloned().unwrap_or(json!({}));
    let (path, content) = match extract_proposed(tool_name, &tool_input) {
        Some(x) => x,
        None => return Ok(None), // not an edit; allow
    };
    let cwd = v.get("cwd").and_then(|c| c.as_str()).map(std::path::PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let store = Store::open(db)?;
    let mut ctx = recall_inject::detect_context(&cwd);
    // Narrow languages by the edited file extension when present.
    if let Some(p) = &path {
        if let Some(lang) = lang_for_path(p) {
            ctx.languages = vec![lang];
        }
    }
    let convs = recall_inject::select(&store.active()?, &ctx, 4000);
    let violations = check(&content, &convs, provider).await.unwrap_or_default(); // fail open
    Ok(pre_tool_use_decision(&violations, mode))
}

fn lang_for_path(p: &str) -> Option<String> {
    let ext = std::path::Path::new(p).extension()?.to_str()?;
    let l = match ext {
        "rs" => "rust", "ts" | "tsx" => "typescript", "js" | "jsx" => "javascript",
        "py" => "python", "go" => "go", _ => return None,
    };
    Some(l.to_string())
}
```

- [ ] **Step 4: Route the event in `crates/recall-cli/src/main.rs`** — add to the `Hook` match:

```rust
                "pre-tool-use" => {
                    let mode = recall_cli::EnforceMode::from_env();
                    if mode != recall_cli::EnforceMode::Off {
                        if let Some(provider) = agent_cli::detect() {
                            if let Some(out) =
                                recall_cli::cmd_hook_pre_tool_use(&db, &input, mode, provider.as_ref()).await?
                            {
                                println!("{out}");
                            }
                        }
                    }
                }
```

- [ ] **Step 5: Run tests + build**

Run: `cargo test -p recall-cli && cargo build`
Expected: PASS; binary builds.

- [ ] **Step 6: Commit**

```bash
git add crates/recall-cli
git commit -m "feat(cli): pre-tool-use enforcement hook (off|warn|block, fail-open)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Register the PreToolUse hook in both plugins + document

**Files:**
- Modify: `plugins/claude-code/hooks/hooks.json`
- Modify: `plugins/codex/hooks/hooks.json`
- Modify: `skills/recall/SKILL.md` (note enforcement)
- Modify: `docs/DEV.md` (how to enable block mode)

**Interfaces:**
- Produces: PreToolUse wired in both hosts (matchers for edit tools); skill mentions it.

- [ ] **Step 1: Add a `PreToolUse` entry to `plugins/claude-code/hooks/hooks.json`** (alongside SessionStart/Stop)

```json
    "PreToolUse": [
      {
        "matcher": "Write|Edit|MultiEdit",
        "hooks": [
          { "type": "command", "command": "npx -y @tlgimenes/recall hook pre-tool-use", "timeout": 30 }
        ]
      }
    ]
```

- [ ] **Step 2: Add a `PreToolUse` entry to `plugins/codex/hooks/hooks.json`** (Codex edit tool is `apply_patch`)

```json
    "PreToolUse": [
      {
        "matcher": "apply_patch|Write|Edit",
        "hooks": [
          { "type": "command", "command": "npx -y @tlgimenes/recall hook pre-tool-use", "timeout": 30 }
        ]
      }
    ]
```

- [ ] **Step 3: Append to `skills/recall/SKILL.md`** (so the agent understands blocks)

```markdown

## Enforcement

If an edit is blocked with a "violates your Recall conventions" reason, fix the
code to satisfy the cited convention and retry — don't work around it. Conventions
are the developer's explicit rules.
```

Then run `just sync-plugins` to propagate the skill into both plugins.

- [ ] **Step 4: Document enabling block mode in `docs/DEV.md`**

````markdown
## Enforcement mode

Recall checks edits against your conventions. Set the mode via `RECALL_ENFORCE`:

- `warn` (default) — adds a heads-up but allows the edit
- `block` — denies edits that violate a convention
- `off` — disables the check

```bash
export RECALL_ENFORCE=block   # in your shell / agent env
```
The check fails open (allows) on any provider error, so it never wedges a session.
````

- [ ] **Step 5: Validate + sync + test**

Run:
```bash
python3 -c "import json; [json.load(open(p)) for p in ['plugins/claude-code/hooks/hooks.json','plugins/codex/hooks/hooks.json']]; print('hooks OK')"
just sync-plugins
cargo test --workspace
```
Expected: `hooks OK`; skills synced; tests pass.

- [ ] **Step 6: Commit**

```bash
git add plugins skills docs/DEV.md
git commit -m "feat(plugin): register PreToolUse enforcement hook in both plugins

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage (product spec §10 fast-follow; architecture enforcement gap #3):**
- Provider-backed violation check → Task 1. ✅
- PreToolUse hook with `off|warn|block`, default warn, fail-open, edit-tools-only → Task 2. ✅
- Registered in both plugins (Codex `apply_patch`) → Task 3. ✅
- Skill teaches the agent to fix-and-retry on block → Task 3. ✅

**Placeholder scan:** No TBD/TODO. Fail-open (`unwrap_or_default`) is deliberate and documented.

**Type consistency:** `Violation` is the `recall-capture` type reused by the CLI; `EnforceMode`, `extract_proposed`, `pre_tool_use_decision`, `cmd_hook_pre_tool_use` signatures match between lib and main; hook payload matches the verified `permissionDecision` contract.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-recall-planFF-enforcement.md`. Execute after Plans 2 and 3 (it depends on the provider + the plugins). Ship as the second launch beat.

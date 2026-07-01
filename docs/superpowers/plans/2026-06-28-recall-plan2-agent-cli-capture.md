# Recall Plan 2 — `agent-cli` bindings + LLM capture/curate

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Learn conventions automatically — a standalone `agent-cli` crate that drives the user's installed `claude`/`codex` (or an API key) as an LLM backend, and an `recall-capture` pipeline that distills session transcripts into curated, deduped, supersession-aware conventions, surfaced through `recall capture` / `recall review`.

**Architecture:** `agent-cli` exposes an `AgentProvider` trait with three backends (Claude subprocess, Codex subprocess, Anthropic API key), each split into **pure arg-builders + output-parsers** (unit-tested) and a thin async spawn/send. `recall-capture` builds a JSON-schema extraction prompt, runs it through a provider, parses `Candidate`s, and curates them into the store (lexical dedup + provider-judged supersession). The CLI adds `capture` (hook entrypoint) and `review` (promote/reject Pending).

**Tech Stack:** `tokio` (process + async), `async-trait`, `serde_json`, `which`, `reqwest` (rustls) for the API-key backend; `recall-core`/`recall-store` from Plans 0–1.

## Global Constraints

- **Only spawn the genuine `claude`/`codex` binaries.** Never extract OAuth tokens; never use an Agent SDK with subscription OAuth. (Anthropic forbids both.)
- **Bounded calls:** single turn, no tools, hard timeout, validated JSON output, retry once on malformed. Scrub `ANTHROPIC_API_KEY` from the child env for subscription-backed Claude runs unless the API-key backend is explicitly selected.
- **Privacy:** transcripts are read locally and sent only to the user's chosen provider. Capture is opt-outable and never transmits anywhere else.
- **`recall-llm` from the architecture spec is realized as:** the reusable `agent-cli` crate (provider) + the Recall-specific prompt/schema living in `recall-capture`. No separate `recall-llm` crate.
- **Pure/impure split:** arg-building and output-parsing are pure functions with unit tests; real-CLI/network calls live behind `#[ignore]` integration tests.
- **Commit style:** end every commit body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

### Task 1: `agent-cli` crate — trait, types, MockProvider

**Files:**
- Create: `crates/agent-cli/Cargo.toml`
- Create: `crates/agent-cli/src/lib.rs`
- Create: `crates/agent-cli/src/mock.rs`
- Modify: root `Cargo.toml` (workspace deps: add `async-trait`, `which`, `tokio`, `reqwest`)
- Test: inline in `crates/agent-cli/src/mock.rs`

**Interfaces:**
- Produces: `agent_cli::{AgentProvider, ProviderHealth, MockProvider}`.
  - `trait AgentProvider: Send + Sync { async fn complete_json(&self, prompt: &str, schema: &serde_json::Value) -> anyhow::Result<serde_json::Value>; async fn health(&self) -> ProviderHealth; fn name(&self) -> &str; }`
  - `struct ProviderHealth { available: bool, detail: String }`
  - `MockProvider::new(canned: serde_json::Value)`.

- [ ] **Step 1: Add shared deps to root `Cargo.toml` `[workspace.dependencies]`**

```toml
async-trait = "0.1"
which = "7"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "io-util", "time"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

- [ ] **Step 2: Create `crates/agent-cli/Cargo.toml`**

```toml
[package]
name = "agent-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Drive Claude Code / Codex (or an API key) as an LLM backend from Rust."

[dependencies]
anyhow = { workspace = true }
async-trait = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
which = { workspace = true }
reqwest = { workspace = true }

[dev-dependencies]
tokio = { workspace = true }
```

- [ ] **Step 3: Write the failing test in `crates/agent-cli/src/mock.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentProvider;
    use serde_json::json;

    #[tokio::test]
    async fn mock_returns_canned_json() {
        let p = MockProvider::new(json!({"ok": true}));
        let out = p.complete_json("anything", &json!({})).await.unwrap();
        assert_eq!(out, json!({"ok": true}));
        assert!(p.health().await.available);
        assert_eq!(p.name(), "mock");
    }
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p agent-cli`
Expected: FAIL — `cannot find type MockProvider`.

- [ ] **Step 5: Write `crates/agent-cli/src/lib.rs`**

```rust
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub mod mock;

/// A backend that performs one bounded, JSON-schema-constrained completion.
#[async_trait]
pub trait AgentProvider: Send + Sync {
    async fn complete_json(&self, prompt: &str, schema: &Value) -> Result<Value>;
    async fn health(&self) -> ProviderHealth;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub available: bool,
    pub detail: String,
}

pub use mock::MockProvider;
```

- [ ] **Step 6: Write `crates/agent-cli/src/mock.rs`** (above the test block)

```rust
use crate::{AgentProvider, ProviderHealth};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Deterministic provider for tests: returns the same canned JSON every call.
pub struct MockProvider {
    canned: Value,
}

impl MockProvider {
    pub fn new(canned: Value) -> Self {
        Self { canned }
    }
}

#[async_trait]
impl AgentProvider for MockProvider {
    async fn complete_json(&self, _prompt: &str, _schema: &Value) -> Result<Value> {
        Ok(self.canned.clone())
    }
    async fn health(&self) -> ProviderHealth {
        ProviderHealth { available: true, detail: "mock".into() }
    }
    fn name(&self) -> &str {
        "mock"
    }
}
```

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test -p agent-cli`
Expected: PASS (1 test).

- [ ] **Step 8: Add the crate to the workspace** — no edit needed (`members = ["crates/*"]` already globs it). Verify: `cargo build -p agent-cli`.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock crates/agent-cli
git commit -m "feat(agent-cli): AgentProvider trait + MockProvider

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: `agent-cli` — Claude backend (pure builders + spawn)

**Files:**
- Create: `crates/agent-cli/src/claude.rs`
- Modify: `crates/agent-cli/src/lib.rs` (add `pub mod claude; pub use claude::ClaudeCli;`)
- Test: inline in `crates/agent-cli/src/claude.rs`

**Interfaces:**
- Produces: `agent_cli::ClaudeCli` (impl `AgentProvider`); pure `build_claude_args(prompt, schema, model: Option<&str>) -> Vec<String>` and `parse_claude_json(stdout: &str) -> Result<Value>`.

- [ ] **Step 1: Add module wiring to `crates/agent-cli/src/lib.rs`**

```rust
pub mod claude;
pub use claude::ClaudeCli;
```

- [ ] **Step 2: Write the failing tests in `crates/agent-cli/src/claude.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn args_request_json_single_turn_no_tools() {
        let args = build_claude_args("hi", &json!({"type": "object"}), Some("claude-sonnet-4-6"));
        assert!(args.contains(&"-p".to_string()));
        assert!(args.windows(2).any(|w| w == ["--output-format", "json"]));
        assert!(args.windows(2).any(|w| w == ["--max-turns", "1"]));
        assert!(args.windows(2).any(|w| w == ["--allowedTools", ""]));
        assert!(args.windows(2).any(|w| w == ["--model", "claude-sonnet-4-6"]));
        assert!(args.iter().any(|a| a == "--json-schema"));
    }

    #[test]
    fn parse_prefers_structured_output() {
        let stdout = r#"{"type":"result","result":"ignored","structured_output":{"rules":[]}}"#;
        assert_eq!(parse_claude_json(stdout).unwrap(), json!({"rules": []}));
    }

    #[test]
    fn parse_falls_back_to_result_string_as_json() {
        let stdout = r#"{"type":"result","result":"{\"rules\":[1]}"}"#;
        assert_eq!(parse_claude_json(stdout).unwrap(), json!({"rules": [1]}));
    }

    #[test]
    fn parse_errors_on_garbage() {
        assert!(parse_claude_json("not json").is_err());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p agent-cli claude`
Expected: FAIL — `cannot find function build_claude_args`.

- [ ] **Step 4: Write the implementation at the top of `crates/agent-cli/src/claude.rs`**

```rust
use crate::{AgentProvider, ProviderHealth};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;

pub struct ClaudeCli {
    pub model: Option<String>,
    pub timeout: Duration,
}

impl Default for ClaudeCli {
    fn default() -> Self {
        Self { model: None, timeout: Duration::from_secs(60) }
    }
}

/// Build the `claude` CLI args for a bounded, schema-constrained completion.
pub fn build_claude_args(prompt: &str, schema: &Value, model: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        prompt.to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        "--json-schema".to_string(),
        schema.to_string(),
        "--max-turns".to_string(),
        "1".to_string(),
        "--allowedTools".to_string(),
        "".to_string(),
    ];
    if let Some(m) = model {
        args.push("--model".to_string());
        args.push(m.to_string());
    }
    args
}

/// Parse the `claude -p --output-format json` envelope into the structured value.
pub fn parse_claude_json(stdout: &str) -> Result<Value> {
    let env: Value = serde_json::from_str(stdout.trim())
        .with_context(|| "claude output was not valid JSON")?;
    if let Some(s) = env.get("structured_output") {
        if !s.is_null() {
            return Ok(s.clone());
        }
    }
    // Fall back to `.result`, which may itself be a JSON string.
    match env.get("result") {
        Some(Value::String(s)) => serde_json::from_str(s)
            .with_context(|| "claude .result was not JSON"),
        Some(v) => Ok(v.clone()),
        None => Err(anyhow!("claude output had neither structured_output nor result")),
    }
}

#[async_trait]
impl AgentProvider for ClaudeCli {
    async fn complete_json(&self, prompt: &str, schema: &Value) -> Result<Value> {
        let args = build_claude_args(prompt, schema, self.model.as_deref());
        let fut = Command::new("claude")
            .args(&args)
            .env_remove("ANTHROPIC_API_KEY") // keep subscription auth; avoid silent per-token billing
            .output();
        let out = tokio::time::timeout(self.timeout, fut)
            .await
            .map_err(|_| anyhow!("claude timed out"))?
            .with_context(|| "failed to spawn `claude` (is Claude Code installed?)")?;
        if !out.status.success() {
            return Err(anyhow!(
                "claude exited with {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        parse_claude_json(&String::from_utf8_lossy(&out.stdout))
    }

    async fn health(&self) -> ProviderHealth {
        match which::which("claude") {
            Ok(p) => ProviderHealth { available: true, detail: format!("claude at {}", p.display()) },
            Err(_) => ProviderHealth { available: false, detail: "claude not found on PATH".into() },
        }
    }

    fn name(&self) -> &str {
        "claude"
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    #[ignore = "requires an installed, authed claude CLI"]
    async fn real_claude_echoes_schema() {
        let p = ClaudeCli::default();
        let schema = json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"]});
        let v = p.complete_json("Reply with answer = ok.", &schema).await.unwrap();
        assert!(v.get("answer").is_some());
    }
}
```

- [ ] **Step 5: Run the unit tests to verify they pass**

Run: `cargo test -p agent-cli claude`
Expected: PASS (4 unit tests; the integration test is `#[ignore]`d).

- [ ] **Step 6: (Optional) validate against the real CLI if available**

Run: `cargo test -p agent-cli -- --ignored real_claude` (only if `claude` is installed/authed).
Expected: PASS, or a clear auth/availability error to fix.

- [ ] **Step 7: Commit**

```bash
git add crates/agent-cli
git commit -m "feat(agent-cli): Claude subprocess backend (claude -p --json-schema)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: `agent-cli` — Codex backend

**Files:**
- Create: `crates/agent-cli/src/codex.rs`
- Modify: `crates/agent-cli/src/lib.rs` (add `pub mod codex; pub use codex::CodexCli;`)
- Test: inline in `crates/agent-cli/src/codex.rs`

**Interfaces:**
- Produces: `agent_cli::CodexCli`; pure `build_codex_args(schema_path: &std::path::Path, model: Option<&str>) -> Vec<String>` and `parse_codex_output(stdout: &str) -> Result<Value>` (extracts the final agent message from the `--json` NDJSON stream and parses it as JSON).

- [ ] **Step 1: Add module wiring to `crates/agent-cli/src/lib.rs`**

```rust
pub mod codex;
pub use codex::CodexCli;
```

- [ ] **Step 2: Write the failing tests in `crates/agent-cli/src/codex.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn args_are_readonly_schema_json() {
        let args = build_codex_args(Path::new("/tmp/s.json"), Some("gpt-5.5"));
        assert!(args.contains(&"exec".to_string()));
        assert!(args.contains(&"--json".to_string()));
        assert!(args.windows(2).any(|w| w == ["--sandbox", "read-only"]));
        assert!(args.contains(&"--skip-git-repo-check".to_string()));
        assert!(args.windows(2).any(|w| w == ["--output-schema", "/tmp/s.json"]));
        assert!(args.windows(2).any(|w| w == ["--model", "gpt-5.5"]));
    }

    #[test]
    fn parse_extracts_last_completed_message_as_json() {
        // codex --json emits NDJSON events; the final agent message carries our JSON.
        let stdout = "\
{\"type\":\"thread.started\"}
{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"{\\\"rules\\\":[]}\"}}
{\"type\":\"turn.completed\"}";
        assert_eq!(parse_codex_output(stdout).unwrap(), json!({"rules": []}));
    }

    #[test]
    fn parse_errors_when_no_message() {
        assert!(parse_codex_output("{\"type\":\"turn.failed\"}").is_err());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p agent-cli codex`
Expected: FAIL — `cannot find function build_codex_args`.

- [ ] **Step 4: Write the implementation at the top of `crates/agent-cli/src/codex.rs`**

```rust
use crate::{AgentProvider, ProviderHealth};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub struct CodexCli {
    pub model: Option<String>,
    pub timeout: Duration,
}

impl Default for CodexCli {
    fn default() -> Self {
        Self { model: None, timeout: Duration::from_secs(60) }
    }
}

pub fn build_codex_args(schema_path: &Path, model: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "exec".to_string(),
        "--json".to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
        "--skip-git-repo-check".to_string(),
        "--output-schema".to_string(),
        schema_path.to_string_lossy().into_owned(),
    ];
    if let Some(m) = model {
        args.push("--model".to_string());
        args.push(m.to_string());
    }
    args
}

/// Scan the NDJSON event stream for the final agent message and parse it as JSON.
pub fn parse_codex_output(stdout: &str) -> Result<Value> {
    let mut last_text: Option<String> = None;
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ev: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if ev.get("type").and_then(|t| t.as_str()) == Some("item.completed") {
            if let Some(text) = ev.pointer("/item/text").and_then(|t| t.as_str()) {
                last_text = Some(text.to_string());
            }
        }
    }
    let text = last_text.ok_or_else(|| anyhow!("codex produced no agent message"))?;
    serde_json::from_str(&text).with_context(|| "codex message was not JSON")
}

#[async_trait]
impl AgentProvider for CodexCli {
    async fn complete_json(&self, prompt: &str, schema: &Value) -> Result<Value> {
        // codex --output-schema takes a file path; write the schema to a temp file.
        let mut tmp = tempfile::NamedTempFile::new().context("temp schema file")?;
        tmp.write_all(schema.to_string().as_bytes())?;
        let path = tmp.path().to_path_buf();

        let args = build_codex_args(&path, self.model.as_deref());
        let mut child = Command::new("codex")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| "failed to spawn `codex` (is Codex installed?)")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await?;
            stdin.shutdown().await?;
        }
        let out = tokio::time::timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| anyhow!("codex timed out"))??;
        if !out.status.success() {
            return Err(anyhow!(
                "codex exited with {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        parse_codex_output(&String::from_utf8_lossy(&out.stdout))
    }

    async fn health(&self) -> ProviderHealth {
        match which::which("codex") {
            Ok(p) => ProviderHealth { available: true, detail: format!("codex at {}", p.display()) },
            Err(_) => ProviderHealth { available: false, detail: "codex not found on PATH".into() },
        }
    }

    fn name(&self) -> &str {
        "codex"
    }
}
```

Also add `tempfile` to `crates/agent-cli/Cargo.toml` `[dependencies]`:

```toml
tempfile = { workspace = true }
```

- [ ] **Step 5: Run the unit tests to verify they pass**

Run: `cargo test -p agent-cli codex`
Expected: PASS (3 tests).

> **Real-CLI caveat:** the exact `--json` event shape for the final message (`item.completed` / `item.text` vs `agent_message`) must be confirmed against the installed `codex` version. If a real run fails parsing, adjust `parse_codex_output`'s pointer (`/item/text`) to match — the unit test documents the assumed shape and is the place to update.

- [ ] **Step 6: Commit**

```bash
git add crates/agent-cli
git commit -m "feat(agent-cli): Codex subprocess backend (codex exec --json --output-schema)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: `agent-cli` — Anthropic API-key backend + `detect()`

**Files:**
- Create: `crates/agent-cli/src/apikey.rs`
- Create: `crates/agent-cli/src/detect.rs`
- Modify: `crates/agent-cli/src/lib.rs`
- Test: inline in both files

**Interfaces:**
- Produces: `agent_cli::AnthropicApiKey` (impl `AgentProvider`); pure `build_anthropic_body(prompt, schema, model) -> Value` (forces a tool call so output is schema-shaped JSON). `agent_cli::detect() -> Option<Box<dyn AgentProvider>>` (prefers `claude`, then `codex`, then `ANTHROPIC_API_KEY`).

- [ ] **Step 1: Add module wiring to `crates/agent-cli/src/lib.rs`**

```rust
pub mod apikey;
pub mod detect;
pub use apikey::AnthropicApiKey;
pub use detect::detect;
```

- [ ] **Step 2: Write the failing test in `crates/agent-cli/src/apikey.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn body_forces_structured_tool_call() {
        let schema = json!({"type":"object","properties":{"x":{"type":"number"}}});
        let body = build_anthropic_body("hi", &schema, "claude-sonnet-4-6");
        assert_eq!(body["model"], "claude-sonnet-4-6");
        assert_eq!(body["tools"][0]["name"], "emit");
        assert_eq!(body["tools"][0]["input_schema"], schema);
        assert_eq!(body["tool_choice"]["type"], "tool");
        assert_eq!(body["tool_choice"]["name"], "emit");
        assert_eq!(body["messages"][0]["role"], "user");
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p agent-cli apikey`
Expected: FAIL — `cannot find function build_anthropic_body`.

- [ ] **Step 4: Write `crates/agent-cli/src/apikey.rs`** (above the test block)

```rust
use crate::{AgentProvider, ProviderHealth};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;

pub struct AnthropicApiKey {
    pub api_key: String,
    pub model: String,
    pub timeout: Duration,
}

impl AnthropicApiKey {
    pub fn from_env() -> Option<Self> {
        std::env::var("ANTHROPIC_API_KEY").ok().map(|k| Self {
            api_key: k,
            model: "claude-sonnet-4-6".to_string(),
            timeout: Duration::from_secs(60),
        })
    }
}

/// Build a Messages API body that forces a single tool call whose input schema is
/// our schema — so the tool input IS the structured JSON we want.
pub fn build_anthropic_body(prompt: &str, schema: &Value, model: &str) -> Value {
    json!({
        "model": model,
        "max_tokens": 1024,
        "tools": [{ "name": "emit", "description": "Emit the result.", "input_schema": schema }],
        "tool_choice": { "type": "tool", "name": "emit" },
        "messages": [{ "role": "user", "content": prompt }]
    })
}

/// Extract the forced tool call's input from a Messages API response.
pub fn parse_anthropic_response(resp: &Value) -> Result<Value> {
    let content = resp.get("content").and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("anthropic response missing content"))?;
    for block in content {
        if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
            if let Some(input) = block.get("input") {
                return Ok(input.clone());
            }
        }
    }
    Err(anyhow!("anthropic response had no tool_use block"))
}

#[async_trait]
impl AgentProvider for AnthropicApiKey {
    async fn complete_json(&self, prompt: &str, schema: &Value) -> Result<Value> {
        let body = build_anthropic_body(prompt, schema, &self.model);
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .timeout(self.timeout)
            .json(&body)
            .send()
            .await
            .context("anthropic request failed")?
            .error_for_status()
            .context("anthropic returned an error status")?
            .json::<Value>()
            .await
            .context("anthropic response was not JSON")?;
        parse_anthropic_response(&resp)
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth { available: !self.api_key.is_empty(), detail: "anthropic api key".into() }
    }

    fn name(&self) -> &str {
        "anthropic-api"
    }
}
```

- [ ] **Step 5: Write the failing test in `crates/agent-cli/src/detect.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_none_or_a_named_provider() {
        // Environment-dependent: just assert it doesn't panic and, if Some,
        // names a known backend.
        if let Some(p) = detect() {
            assert!(["claude", "codex", "anthropic-api"].contains(&p.name()));
        }
    }
}
```

- [ ] **Step 6: Run the test to verify it fails**

Run: `cargo test -p agent-cli detect`
Expected: FAIL — `cannot find function detect`.

- [ ] **Step 7: Write `crates/agent-cli/src/detect.rs`** (above the test block)

```rust
use crate::{AgentProvider, AnthropicApiKey, ClaudeCli, CodexCli};

/// Pick a provider: prefer an installed `claude`, then `codex`, then an
/// `ANTHROPIC_API_KEY`. Returns None if nothing is available.
pub fn detect() -> Option<Box<dyn AgentProvider>> {
    if which::which("claude").is_ok() {
        return Some(Box::new(ClaudeCli::default()));
    }
    if which::which("codex").is_ok() {
        return Some(Box::new(CodexCli::default()));
    }
    AnthropicApiKey::from_env().map(|p| Box::new(p) as Box<dyn AgentProvider>)
}
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p agent-cli`
Expected: PASS (all agent-cli tests).

- [ ] **Step 9: Commit**

```bash
git add crates/agent-cli
git commit -m "feat(agent-cli): Anthropic API-key backend + provider detect()

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: `recall-capture` — extraction (transcript → candidates)

**Files:**
- Create: `crates/recall-capture/Cargo.toml`
- Create: `crates/recall-capture/src/lib.rs`
- Create: `crates/recall-capture/src/extract.rs`
- Test: inline in `crates/recall-capture/src/extract.rs`

**Interfaces:**
- Produces: `recall_capture::{Candidate, ScopeHint, extraction_schema, extraction_prompt, parse_candidates, extract}`.
  - `struct Candidate { rule: String, scope_hint: ScopeHint, tags: Vec<String>, rationale: Option<String>, excerpt: Option<String> }`
  - `enum ScopeHint { Global, Language(String), Repo, Branch }`
  - `async fn extract(transcript: &str, provider: &dyn AgentProvider) -> Result<Vec<Candidate>>`
- Consumes: `agent_cli::AgentProvider`.

- [ ] **Step 1: Create `crates/recall-capture/Cargo.toml`**

```toml
[package]
name = "recall-capture"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
recall-core = { path = "../recall-core" }
recall-store = { path = "../recall-store" }
agent-cli = { path = "../agent-cli" }
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }

[dev-dependencies]
tokio = { workspace = true }
```

- [ ] **Step 2: Write the failing tests in `crates/recall-capture/src/extract.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use agent_cli::MockProvider;
    use serde_json::json;

    #[test]
    fn parse_candidates_reads_rules_array() {
        let v = json!({"conventions":[
            {"rule":"Import directly; no barrel files","scope":"global","tags":["imports"],"excerpt":"no barrels"}
        ]});
        let cands = parse_candidates(&v).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].rule, "Import directly; no barrel files");
        assert!(matches!(cands[0].scope_hint, ScopeHint::Global));
        assert_eq!(cands[0].tags, vec!["imports".to_string()]);
    }

    #[test]
    fn parse_candidates_maps_language_scope() {
        let v = json!({"conventions":[{"rule":"Use ? over unwrap","scope":"language:rust","tags":[]}]});
        let cands = parse_candidates(&v).unwrap();
        assert!(matches!(&cands[0].scope_hint, ScopeHint::Language(l) if l == "rust"));
    }

    #[tokio::test]
    async fn extract_runs_provider_and_parses() {
        let provider = MockProvider::new(json!({"conventions":[
            {"rule":"Prefer early returns","scope":"global","tags":[]}
        ]}));
        let cands = extract("user: prefer early returns please", &provider).await.unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].rule, "Prefer early returns");
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p recall-capture`
Expected: FAIL — `cannot find ... ScopeHint`.

- [ ] **Step 4: Write `crates/recall-capture/src/lib.rs`**

```rust
mod extract;
pub use extract::*;
```

- [ ] **Step 5: Write the implementation at the top of `crates/recall-capture/src/extract.rs`**

```rust
use agent_cli::AgentProvider;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeHint {
    Global,
    Language(String),
    Repo,
    Branch,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub rule: String,
    pub scope_hint: ScopeHint,
    pub tags: Vec<String>,
    pub rationale: Option<String>,
    pub excerpt: Option<String>,
}

/// JSON schema the provider must satisfy.
pub fn extraction_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "conventions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "rule": { "type": "string" },
                        "scope": { "type": "string", "description": "global | repo | branch | language:<lang>" },
                        "tags": { "type": "array", "items": { "type": "string" } },
                        "rationale": { "type": "string" },
                        "excerpt": { "type": "string" }
                    },
                    "required": ["rule", "scope"]
                }
            }
        },
        "required": ["conventions"]
    })
}

pub fn extraction_prompt(transcript: &str) -> String {
    format!(
        "You extract DURABLE, PERSONAL coding conventions a developer expressed or \
         corrected in this agent session. Ignore one-off task details. For each, give \
         an imperative rule under 140 chars, a scope (global | repo | branch | \
         language:<lang>), optional tags, and the short excerpt it came from. Return \
         JSON matching the schema. If none, return an empty array.\n\n\
         === SESSION TRANSCRIPT ===\n{transcript}"
    )
}

fn scope_hint_from_str(s: &str) -> ScopeHint {
    if let Some(lang) = s.strip_prefix("language:") {
        return ScopeHint::Language(lang.to_string());
    }
    match s {
        "repo" => ScopeHint::Repo,
        "branch" => ScopeHint::Branch,
        _ => ScopeHint::Global,
    }
}

pub fn parse_candidates(v: &Value) -> Result<Vec<Candidate>> {
    let arr = v.get("conventions").and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("missing 'conventions' array"))?;
    let mut out = Vec::new();
    for item in arr {
        let rule = item.get("rule").and_then(|r| r.as_str())
            .ok_or_else(|| anyhow!("convention missing 'rule'"))?
            .to_string();
        let scope_hint = scope_hint_from_str(item.get("scope").and_then(|s| s.as_str()).unwrap_or("global"));
        let tags = item.get("tags").and_then(|t| t.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let rationale = item.get("rationale").and_then(|r| r.as_str()).map(String::from);
        let excerpt = item.get("excerpt").and_then(|e| e.as_str()).map(String::from);
        out.push(Candidate { rule, scope_hint, tags, rationale, excerpt });
    }
    Ok(out)
}

/// Run the extraction prompt through the provider and parse the result.
pub async fn extract(transcript: &str, provider: &dyn AgentProvider) -> Result<Vec<Candidate>> {
    let schema = extraction_schema();
    let raw = provider.complete_json(&extraction_prompt(transcript), &schema).await?;
    parse_candidates(&raw)
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p recall-capture`
Expected: PASS (3 tests).

- [ ] **Step 7: Commit**

```bash
git add crates/recall-capture
git commit -m "feat(capture): session transcript -> convention candidates via provider

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: `recall-capture` — curation (candidates → store, with supersession)

**Files:**
- Create: `crates/recall-capture/src/curate.rs`
- Modify: `crates/recall-capture/src/lib.rs` (add `mod curate; pub use curate::*;`)
- Test: inline in `crates/recall-capture/src/curate.rs`

**Interfaces:**
- Produces: `candidate_to_convention(c: &Candidate, ctx: &RepoContext, now: DateTime<Utc>) -> Convention` (status `Pending`); `async fn curate(cands: &[Candidate], store: &Store, ctx: &RepoContext, provider: &dyn AgentProvider) -> Result<CurationReport>`; `struct CurationReport { added: Vec<Uuid>, corroborated: Vec<Uuid>, conflicts: Vec<(Uuid, Uuid)> }`.
- Consumes: `recall_core` (`Convention`, `RepoContext`, `dedup_decision`, `normalize_rule`), `recall_store::Store`, `agent_cli::AgentProvider`.

- [ ] **Step 1: Write the failing tests in `crates/recall-capture/src/curate.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Candidate, ScopeHint};
    use agent_cli::MockProvider;
    use recall_core::*;
    use recall_store::Store;
    use chrono::Utc;
    use serde_json::json;

    fn cand(rule: &str) -> Candidate {
        Candidate { rule: rule.into(), scope_hint: ScopeHint::Global, tags: vec![], rationale: None, excerpt: None }
    }
    fn ctx() -> RepoContext { RepoContext::default() }

    #[test]
    fn candidate_becomes_pending_convention() {
        let c = candidate_to_convention(&cand("Use early returns"), &ctx(), Utc::now());
        assert_eq!(c.status, Status::Pending);
        assert_eq!(c.scope, Scope::Global);
        assert_eq!(c.provenance.source, Source::SessionDistill);
    }

    #[tokio::test]
    async fn curate_adds_new_pending() {
        let store = Store::open_in_memory().unwrap();
        let provider = MockProvider::new(json!({"contradicts": false}));
        let report = curate(&[cand("Use early returns")], &store, &ctx(), &provider).await.unwrap();
        assert_eq!(report.added.len(), 1);
        assert_eq!(store.all().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn curate_corroborates_duplicate_active() {
        let store = Store::open_in_memory().unwrap();
        // seed an ACTIVE identical convention
        let mut seed = candidate_to_convention(&cand("Use early returns"), &ctx(), Utc::now());
        seed.status = Status::Active;
        store.add(&seed).unwrap();
        let provider = MockProvider::new(json!({"contradicts": false}));
        let report = curate(&[cand("use EARLY returns")], &store, &ctx(), &provider).await.unwrap();
        assert_eq!(report.corroborated, vec![seed.id]);
        assert_eq!(report.added.len(), 0);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p recall-capture curate`
Expected: FAIL — `cannot find function candidate_to_convention`.

- [ ] **Step 3: Add module wiring to `crates/recall-capture/src/lib.rs`**

```rust
mod curate;
mod extract;
pub use curate::*;
pub use extract::*;
```

- [ ] **Step 4: Write the implementation at the top of `crates/recall-capture/src/curate.rs`**

```rust
use crate::{Candidate, ScopeHint};
use agent_cli::AgentProvider;
use anyhow::Result;
use chrono::{DateTime, Utc};
use recall_core::{dedup_decision, Convention, DedupDecision, Provenance, RepoContext, Scope, Source, Status};
use recall_store::Store;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct CurationReport {
    pub added: Vec<Uuid>,
    pub corroborated: Vec<Uuid>,
    pub conflicts: Vec<(Uuid, Uuid)>, // (new_pending, existing_active_it_may_supersede)
}

fn resolve_scope(hint: &ScopeHint, ctx: &RepoContext) -> Scope {
    match hint {
        ScopeHint::Global => Scope::Global,
        ScopeHint::Language(l) => Scope::Language(l.clone()),
        ScopeHint::Repo => match &ctx.remote_id {
            Some(r) => Scope::Repo { remote_id: r.clone() },
            None => Scope::Global,
        },
        ScopeHint::Branch => match (&ctx.remote_id, &ctx.branch) {
            (Some(r), Some(b)) => Scope::Branch { remote_id: r.clone(), branch: b.clone() },
            _ => Scope::Global,
        },
    }
}

pub fn candidate_to_convention(c: &Candidate, ctx: &RepoContext, now: DateTime<Utc>) -> Convention {
    Convention {
        id: Uuid::new_v4(),
        rule: c.rule.clone(),
        rationale: c.rationale.clone(),
        scope: resolve_scope(&c.scope_hint, ctx),
        tags: c.tags.clone(),
        provenance: Provenance {
            source: Source::SessionDistill,
            repo: ctx.remote_id.clone(),
            branch: ctx.branch.clone(),
            agent: None,
            excerpt: c.excerpt.clone(),
            learned_at: now,
        },
        status: Status::Pending,
        superseded_by: None,
        confidence: 0.5,
        created_at: now,
        updated_at: now,
    }
}

/// Ask the provider whether `new_rule` contradicts `existing_rule` (same scope).
async fn contradicts(provider: &dyn AgentProvider, new_rule: &str, existing_rule: &str) -> bool {
    let schema = json!({"type":"object","properties":{"contradicts":{"type":"boolean"}},"required":["contradicts"]});
    let prompt = format!(
        "Do these two coding conventions directly contradict each other (one says to do \
         X, the other says NOT to do X, for the same situation)? Reply JSON.\n\
         A: {new_rule}\nB: {existing_rule}"
    );
    provider.complete_json(&prompt, &schema).await
        .ok()
        .and_then(|v| v.get("contradicts").and_then(|c| c.as_bool()))
        .unwrap_or(false)
}

pub async fn curate(
    cands: &[Candidate],
    store: &Store,
    ctx: &RepoContext,
    provider: &dyn AgentProvider,
) -> Result<CurationReport> {
    let mut report = CurationReport::default();
    for c in cands {
        let conv = candidate_to_convention(c, ctx, Utc::now());
        let active = store.active()?;
        match dedup_decision(&conv.rule, &conv.scope, &active) {
            DedupDecision::Corroborates(id) => {
                if let Some(mut e) = store.get(id)? {
                    e.confidence = (e.confidence + 0.1).min(1.0);
                    store.add(&e)?;
                }
                report.corroborated.push(id);
            }
            DedupDecision::New => {
                // Look for a same-scope active rule this one might supersede.
                for e in active.iter().filter(|e| e.scope == conv.scope) {
                    if contradicts(provider, &conv.rule, &e.rule).await {
                        report.conflicts.push((conv.id, e.id));
                        break;
                    }
                }
                store.add(&conv)?; // Pending; supersession applied on review-accept
                report.added.push(conv.id);
            }
        }
    }
    Ok(report)
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p recall-capture`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/recall-capture
git commit -m "feat(capture): curate candidates into Pending conventions, detect conflicts

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: CLI — `capture`, `review`, and provider-aware `status`

**Files:**
- Modify: `crates/recall-cli/Cargo.toml` (add `recall-capture`, `agent-cli` deps)
- Modify: `crates/recall-cli/src/lib.rs` (add `cmd_capture`, `cmd_review_list`, `cmd_review_accept`, `cmd_review_reject`; update `cmd_status`)
- Modify: `crates/recall-cli/src/main.rs` (add `Capture`, `Review` subcommands)
- Test: extend the `#[cfg(test)]` block in `crates/recall-cli/src/lib.rs`

**Interfaces:**
- Produces: `cmd_capture(db, transcript_path, ctx, provider) -> Result<String>`; `cmd_review_list(db) -> Result<String>`; `cmd_review_accept(db, id_prefix) -> Result<String>` (Pending→Active; if it had a conflict, retire the conflicting active one as Superseded); `cmd_review_reject(db, id_prefix) -> Result<String>`.

- [ ] **Step 1: Add deps to `crates/recall-cli/Cargo.toml`**

```toml
recall-capture = { path = "../recall-capture" }
agent-cli = { path = "../agent-cli" }
```

- [ ] **Step 2: Write the failing tests in the `#[cfg(test)]` block of `crates/recall-cli/src/lib.rs`**

```rust
    #[tokio::test]
    async fn capture_then_review_accept_promotes() {
        use agent_cli::MockProvider;
        use serde_json::json;
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let transcript = tmp.path().join("t.txt");
        std::fs::write(&transcript, "user: always use early returns").unwrap();

        let extractor = MockProvider::new(json!({"conventions":[
            {"rule":"Use early returns","scope":"global","tags":[]}
        ]}));
        let ctx = recall_core::RepoContext::default();
        let msg = cmd_capture(&db, &transcript, &ctx, &extractor).await.unwrap();
        assert!(msg.to_lowercase().contains("1"));

        let pending = cmd_review_list(&db).unwrap();
        assert!(pending.contains("Use early returns"));
        let id = &pending[pending.find('[').unwrap() + 1..pending.find(']').unwrap()];

        let accepted = cmd_review_accept(&db, id).unwrap();
        assert!(accepted.to_lowercase().contains("accept"));
        assert!(cmd_list(&db).unwrap().contains("Use early returns")); // now Active
    }

    #[tokio::test]
    async fn review_reject_retires_pending() {
        use agent_cli::MockProvider;
        use serde_json::json;
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let transcript = tmp.path().join("t.txt");
        std::fs::write(&transcript, "x").unwrap();
        let extractor = MockProvider::new(json!({"conventions":[{"rule":"Nope","scope":"global","tags":[]}]}));
        cmd_capture(&db, &transcript, &recall_core::RepoContext::default(), &extractor).await.unwrap();
        let pending = cmd_review_list(&db).unwrap();
        let id = &pending[pending.find('[').unwrap() + 1..pending.find(']').unwrap()];
        cmd_review_reject(&db, id).unwrap();
        assert!(cmd_review_list(&db).unwrap().to_lowercase().contains("no pending"));
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p recall-cli`
Expected: FAIL — `cannot find function cmd_capture`.

- [ ] **Step 4: Add the implementations to `crates/recall-cli/src/lib.rs`** (above the test block)

```rust
use agent_cli::AgentProvider;
use recall_capture::{curate, extract};
use recall_core::{RepoContext, Status};

pub async fn cmd_capture(
    db: &Path,
    transcript_path: &Path,
    ctx: &RepoContext,
    provider: &dyn AgentProvider,
) -> Result<String> {
    let transcript = std::fs::read_to_string(transcript_path)
        .with_context(|| format!("reading transcript {}", transcript_path.display()))?;
    let store = Store::open(db)?;
    let candidates = extract(&transcript, provider).await?;
    let report = curate(&candidates, &store, ctx, provider).await?;
    Ok(format!(
        "Captured: {} new (pending review), {} corroborated, {} potential conflicts.",
        report.added.len(),
        report.corroborated.len(),
        report.conflicts.len()
    ))
}

pub fn cmd_review_list(db: &Path) -> Result<String> {
    let store = Store::open(db)?;
    let pending: Vec<_> = store.all()?.into_iter().filter(|c| c.status == Status::Pending).collect();
    if pending.is_empty() {
        return Ok("No pending conventions to review.".to_string());
    }
    let mut s = String::from("Pending review (recall review --accept <id> | --reject <id>):\n");
    for c in &pending {
        s.push_str(&format!("[{}] {} ({})\n", short(&c.id), c.rule, scope_label(&c.scope)));
    }
    Ok(s.trim_end().to_string())
}

pub fn cmd_review_accept(db: &Path, id_prefix: &str) -> Result<String> {
    let store = Store::open(db)?;
    let mut c = find_by_prefix(&store, id_prefix)?;
    // Retire any same-scope active rule with a different normalized text (supersession).
    let n = recall_core::normalize_rule(&c.rule);
    for e in store.active()? {
        if e.scope == c.scope && recall_core::normalize_rule(&e.rule) != n {
            let mut sup = e.clone();
            sup.status = Status::Superseded;
            sup.superseded_by = Some(c.id);
            store.add(&sup)?;
        }
    }
    c.status = Status::Active;
    c.confidence = c.confidence.max(0.8);
    store.add(&c)?;
    Ok(format!("Accepted [{}]: {}", short(&c.id), c.rule))
}

pub fn cmd_review_reject(db: &Path, id_prefix: &str) -> Result<String> {
    let store = Store::open(db)?;
    let mut c = find_by_prefix(&store, id_prefix)?;
    c.status = Status::Retired;
    store.add(&c)?;
    Ok(format!("Rejected [{}]: {}", short(&c.id), c.rule))
}
```

> Also update `cmd_status` to report provider detection. Replace the existing
> `cmd_status` body's provider line with a live detect:
> ```rust
> let provider = match agent_cli::detect() {
>     Some(p) => p.name().to_string(),
>     None => "none (install Claude Code or Codex, or set ANTHROPIC_API_KEY)".to_string(),
> };
> ```
> and include `provider` in the formatted output.

- [ ] **Step 5: Wire new subcommands in `crates/recall-cli/src/main.rs`**

Add to the `Cmd` enum:

```rust
    /// Capture conventions from a session transcript (used by the Stop hook)
    Capture {
        /// Path to the session transcript file
        transcript: String,
    },
    /// Review pending conventions
    Review {
        /// Accept a pending convention by id (or prefix)
        #[arg(long)]
        accept: Option<String>,
        /// Reject a pending convention by id (or prefix)
        #[arg(long)]
        reject: Option<String>,
    },
```

Add to the `match cli.cmd` block:

```rust
        Cmd::Capture { transcript } => {
            let provider = agent_cli::detect()
                .ok_or_else(|| anyhow::anyhow!("no LLM provider; install Claude Code or Codex"))?;
            let ctx = recall_inject::detect_context(&std::env::current_dir()?);
            println!(
                "{}",
                recall_cli::cmd_capture(&db, std::path::Path::new(&transcript), &ctx, provider.as_ref()).await?
            );
        }
        Cmd::Review { accept, reject } => {
            if let Some(id) = accept {
                println!("{}", recall_cli::cmd_review_accept(&db, &id)?);
            } else if let Some(id) = reject {
                println!("{}", recall_cli::cmd_review_reject(&db, &id)?);
            } else {
                println!("{}", recall_cli::cmd_review_list(&db)?);
            }
        }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p recall-cli`
Expected: PASS (all CLI tests, including the two new async ones).

- [ ] **Step 7: Build the binary end-to-end**

Run: `cargo build && ./target/debug/recall review`
Expected: "No pending conventions to review." (on an empty/default DB).

- [ ] **Step 8: Commit**

```bash
git add crates/recall-cli
git commit -m "feat(cli): capture + review commands; provider-aware status

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage (architecture §3 agent-cli; product spec §6.1 capture, §7 provider):**
- `agent-cli` standalone crate with `AgentProvider` + Claude/Codex/ApiKey + `detect()` → Tasks 1–4. ✅
- ToS guardrails (genuine binaries, env-scrub `ANTHROPIC_API_KEY`, bounded single-turn, timeouts) → Tasks 2–4. ✅
- Session distillation (transcript → candidates) → Task 5. ✅
- Curation: lexical dedup + provider-judged conflict → Pending → Task 6. ✅
- Supersession applied on accept → Task 7 `cmd_review_accept`. ✅
- `recall capture` (Stop-hook entrypoint) + `recall review` → Task 7. ✅ (The hook itself is wired in Plan 3.)
- Provider disclosure in `status` → Task 7. ✅
- Auto-promote-by-confidence threshold → **intentionally deferred**: Plan 2 promotes via explicit `review` only; a confidence auto-promote is a small follow-up once real distillation data exists (flagged, not silent).

**Placeholder scan:** No TBD/TODO. Real-CLI output shapes are flagged with `#[ignore]` integration tests + notes (Tasks 2, 3), which is the honest TDD treatment for external-binary behavior.

**Type consistency:** `AgentProvider`/`complete_json` signature is identical across all backends and consumers; `Candidate`/`ScopeHint`/`CurationReport` and the `cmd_*` signatures match between capture and CLI; `dedup_decision`/`normalize_rule`/`Status`/`Scope` reused from `recall-core` exactly as defined in Plan 1.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-recall-plan2-agent-cli-capture.md`. Execute via subagent-driven (recommended) or inline, **after** Plans 0 and 1.

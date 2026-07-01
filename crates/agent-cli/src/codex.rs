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
        Self {
            model: None,
            timeout: Duration::from_secs(60),
        }
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
            .kill_on_drop(true)
            .spawn()
            .with_context(|| "failed to spawn `codex` (is Codex installed?)")?;
        if let Some(mut stdin) = child.stdin.take() {
            let prompt = prompt.to_string();
            tokio::spawn(async move {
                let _ = stdin.write_all(prompt.as_bytes()).await;
                let _ = stdin.shutdown().await;
            });
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
            Ok(p) => ProviderHealth {
                available: true,
                detail: format!("codex at {}", p.display()),
            },
            Err(_) => ProviderHealth {
                available: false,
                detail: "codex not found on PATH".into(),
            },
        }
    }

    fn name(&self) -> &str {
        "codex"
    }
}

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
        assert!(args
            .windows(2)
            .any(|w| w == ["--output-schema", "/tmp/s.json"]));
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

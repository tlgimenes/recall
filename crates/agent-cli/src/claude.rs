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
        Self {
            model: None,
            timeout: Duration::from_secs(60),
        }
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
    let env: Value =
        serde_json::from_str(stdout.trim()).with_context(|| "claude output was not valid JSON")?;
    if let Some(s) = env.get("structured_output") {
        if !s.is_null() {
            return Ok(s.clone());
        }
    }
    // Fall back to `.result`, which may itself be a JSON string.
    match env.get("result") {
        Some(Value::String(s)) => {
            serde_json::from_str(s).with_context(|| "claude .result was not JSON")
        }
        Some(v) => Ok(v.clone()),
        None => Err(anyhow!(
            "claude output had neither structured_output nor result"
        )),
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
            Ok(p) => ProviderHealth {
                available: true,
                detail: format!("claude at {}", p.display()),
            },
            Err(_) => ProviderHealth {
                available: false,
                detail: "claude not found on PATH".into(),
            },
        }
    }

    fn name(&self) -> &str {
        "claude"
    }
}

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
        assert!(args
            .windows(2)
            .any(|w| w == ["--model", "claude-sonnet-4-6"]));
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

#[cfg(test)]
mod integration {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    #[ignore = "requires an installed, authed claude CLI"]
    async fn real_claude_echoes_schema() {
        let p = ClaudeCli::default();
        let schema = json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"]});
        let v = p
            .complete_json("Reply with answer = ok.", &schema)
            .await
            .unwrap();
        assert!(v.get("answer").is_some());
    }
}

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
    let content = resp
        .get("content")
        .and_then(|c| c.as_array())
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
        ProviderHealth {
            available: !self.api_key.is_empty(),
            detail: "anthropic api key".into(),
        }
    }

    fn name(&self) -> &str {
        "anthropic-api"
    }
}

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

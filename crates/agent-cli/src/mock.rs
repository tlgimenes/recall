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
        ProviderHealth {
            available: true,
            detail: "mock".into(),
        }
    }
    fn name(&self) -> &str {
        "mock"
    }
}

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

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

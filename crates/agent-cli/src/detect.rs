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

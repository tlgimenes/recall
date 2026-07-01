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
    let arr = v
        .get("violations")
        .and_then(|a| a.as_array())
        .ok_or_else(|| anyhow!("missing 'violations'"))?;
    Ok(arr
        .iter()
        .filter_map(|item| {
            Some(Violation {
                rule: item.get("rule")?.as_str()?.to_string(),
                explanation: item.get("explanation")?.as_str()?.to_string(),
            })
        })
        .collect())
}

pub async fn check(
    content: &str,
    conventions: &[Convention],
    provider: &dyn AgentProvider,
) -> Result<Vec<Violation>> {
    if conventions.is_empty() {
        return Ok(vec![]);
    }
    let raw = provider
        .complete_json(&check_prompt(content, conventions), &check_schema())
        .await?;
    parse_violations(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_cli::MockProvider;
    use chrono::Utc;
    use recall_core::*;
    use serde_json::json;
    use uuid::Uuid;

    fn conv(rule: &str) -> Convention {
        let now = Utc::now();
        Convention {
            id: Uuid::new_v4(),
            rule: rule.into(),
            rationale: None,
            scope: Scope::Global,
            tags: vec![],
            provenance: Provenance {
                source: Source::ManualTeach,
                repo: None,
                branch: None,
                agent: None,
                excerpt: None,
                learned_at: now,
            },
            status: Status::Active,
            superseded_by: None,
            confidence: 0.9,
            created_at: now,
            updated_at: now,
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
        let provider = MockProvider::new(
            json!({"violations":[{"rule":"No barrel files","explanation":"re-export"}]}),
        );
        let vs = check("export * from './x'", &[conv("No barrel files")], &provider)
            .await
            .unwrap();
        assert_eq!(vs.len(), 1);
    }
}

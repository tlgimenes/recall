use crate::{Convention, Scope, Status};
use uuid::Uuid;

/// Canonical form for comparing two rule strings.
pub fn normalize_rule(s: &str) -> String {
    s.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Result of checking a new rule against existing active conventions.
#[derive(Debug, PartialEq, Eq)]
pub enum DedupDecision {
    New,
    Corroborates(Uuid),
}

/// Plan-1 dedup: an active convention with the same scope and the same
/// normalized rule text corroborates; otherwise it's new. (Semantic
/// contradiction → supersession arrives in Plan 2 with the LLM provider.)
pub fn dedup_decision(new_rule: &str, new_scope: &Scope, existing: &[Convention]) -> DedupDecision {
    let n = normalize_rule(new_rule);
    for c in existing {
        if c.status == Status::Active && &c.scope == new_scope && normalize_rule(&c.rule) == n {
            return DedupDecision::Corroborates(c.id);
        }
    }
    DedupDecision::New
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn conv(rule: &str, scope: Scope) -> Convention {
        let now = Utc::now();
        Convention {
            id: Uuid::new_v4(),
            rule: rule.into(),
            rationale: None,
            scope,
            tags: vec![],
            provenance: Provenance {
                source: Source::ManualTeach, repo: None, branch: None,
                agent: None, excerpt: None, learned_at: now,
            },
            status: Status::Active,
            superseded_by: None,
            confidence: 0.8,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn normalize_collapses_whitespace_and_case() {
        assert_eq!(normalize_rule("  Use   Early  Returns "), "use early returns");
    }

    #[test]
    fn dedup_detects_same_rule_same_scope() {
        let existing = vec![conv("Use early returns", Scope::Global)];
        match dedup_decision("use   EARLY returns", &Scope::Global, &existing) {
            DedupDecision::Corroborates(id) => assert_eq!(id, existing[0].id),
            _ => panic!("expected Corroborates"),
        }
    }

    #[test]
    fn dedup_treats_different_scope_as_new() {
        let existing = vec![conv("Use early returns", Scope::Global)];
        assert!(matches!(
            dedup_decision("use early returns", &Scope::Language("rust".into()), &existing),
            DedupDecision::New
        ));
    }

    #[test]
    fn dedup_ignores_non_active() {
        let mut e = conv("Use early returns", Scope::Global);
        e.status = Status::Retired;
        assert!(matches!(
            dedup_decision("use early returns", &Scope::Global, &[e]),
            DedupDecision::New
        ));
    }
}

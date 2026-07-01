mod context;
pub use context::*;

use recall_core::{Convention, RepoContext, Scope, Status};

/// Select the active conventions relevant to `ctx`, most-specific scope first,
/// then highest confidence, then most recent — capped to a character budget.
pub fn select(convs: &[Convention], ctx: &RepoContext, budget_chars: usize) -> Vec<Convention> {
    let mut matched: Vec<Convention> = convs
        .iter()
        .filter(|c| c.status == Status::Active && c.scope.matches(ctx))
        .cloned()
        .collect();

    matched.sort_by(|a, b| {
        b.scope
            .specificity()
            .cmp(&a.scope.specificity())
            .then(
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(b.updated_at.cmp(&a.updated_at))
    });

    let mut out = Vec::new();
    let mut used = 0usize;
    for c in matched {
        let cost = c.rule.len() + 4;
        if used + cost > budget_chars && !out.is_empty() {
            break;
        }
        used += cost;
        out.push(c);
    }
    out
}

/// Render conventions as a compact Markdown block for injection into an agent.
pub fn render(convs: &[Convention]) -> String {
    if convs.is_empty() {
        return String::new();
    }
    let mut s = String::from("# Your coding conventions (via Recall)\n\n");
    for c in convs {
        s.push_str(&format!(
            "- {} _({})_\n",
            c.rule.trim(),
            scope_label(&c.scope)
        ));
    }
    s
}

/// Human-readable label for a scope.
pub fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Global => "global".to_string(),
        Scope::Language(l) => format!("language: {l}"),
        Scope::Repo { remote_id } => format!("repo: {remote_id}"),
        Scope::Branch { remote_id, branch } => format!("branch: {remote_id}@{branch}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use recall_core::*;
    use uuid::Uuid;

    fn conv(rule: &str, scope: Scope, conf: f32) -> Convention {
        let now = Utc::now();
        Convention {
            id: Uuid::new_v4(),
            rule: rule.into(),
            rationale: None,
            scope,
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
            confidence: conf,
            created_at: now,
            updated_at: now,
        }
    }

    fn ctx() -> RepoContext {
        RepoContext {
            remote_id: Some("github.com/me/app".into()),
            branch: Some("main".into()),
            languages: vec!["rust".into()],
        }
    }

    #[test]
    fn select_filters_by_scope_and_orders_most_specific_first() {
        let convs = vec![
            conv("global rule", Scope::Global, 0.9),
            conv("rust rule", Scope::Language("rust".into()), 0.5),
            conv(
                "repo rule",
                Scope::Repo {
                    remote_id: "github.com/me/app".into(),
                },
                0.5,
            ),
            conv(
                "other repo",
                Scope::Repo {
                    remote_id: "github.com/me/other".into(),
                },
                0.9,
            ),
        ];
        let out = select(&convs, &ctx(), 10_000);
        let rules: Vec<&str> = out.iter().map(|c| c.rule.as_str()).collect();
        assert_eq!(rules, vec!["repo rule", "rust rule", "global rule"]);
    }

    #[test]
    fn select_respects_budget() {
        let convs = vec![
            conv(
                "aaaaaaaaaa",
                Scope::Repo {
                    remote_id: "github.com/me/app".into(),
                },
                0.9,
            ),
            conv("bbbbbbbbbb", Scope::Global, 0.9),
        ];
        let out = select(&convs, &ctx(), 12);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule, "aaaaaaaaaa");
    }

    #[test]
    fn render_lists_rules_with_scope_labels() {
        let convs = vec![conv("Use early returns", Scope::Global, 0.9)];
        let r = render(&convs);
        assert!(r.contains("Use early returns"));
        assert!(r.contains("global"));
    }

    #[test]
    fn render_empty_is_empty_string() {
        assert_eq!(render(&[]), "");
    }
}

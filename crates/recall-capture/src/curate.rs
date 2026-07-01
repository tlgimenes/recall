use crate::{Candidate, ScopeHint};
use agent_cli::AgentProvider;
use anyhow::Result;
use chrono::{DateTime, Utc};
use recall_core::{
    dedup_decision, Convention, DedupDecision, Provenance, RepoContext, Scope, Source, Status,
};
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
            Some(r) => Scope::Repo {
                remote_id: r.clone(),
            },
            None => Scope::Global,
        },
        ScopeHint::Branch => match (&ctx.remote_id, &ctx.branch) {
            (Some(r), Some(b)) => Scope::Branch {
                remote_id: r.clone(),
                branch: b.clone(),
            },
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
pub async fn contradicts(
    provider: &dyn AgentProvider,
    new_rule: &str,
    existing_rule: &str,
) -> bool {
    let schema = json!({"type":"object","properties":{"contradicts":{"type":"boolean"}},"required":["contradicts"]});
    let prompt = format!(
        "Do these two coding conventions directly contradict each other (one says to do \
         X, the other says NOT to do X, for the same situation)? Reply JSON.\n\
         A: {new_rule}\nB: {existing_rule}"
    );
    provider
        .complete_json(&prompt, &schema)
        .await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Candidate, ScopeHint};
    use agent_cli::MockProvider;
    use chrono::Utc;
    use recall_store::Store;
    use serde_json::json;

    fn cand(rule: &str) -> Candidate {
        Candidate {
            rule: rule.into(),
            scope_hint: ScopeHint::Global,
            tags: vec![],
            rationale: None,
            excerpt: None,
        }
    }
    fn ctx() -> RepoContext {
        RepoContext::default()
    }

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
        let report = curate(&[cand("Use early returns")], &store, &ctx(), &provider)
            .await
            .unwrap();
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
        let report = curate(&[cand("use EARLY returns")], &store, &ctx(), &provider)
            .await
            .unwrap();
        assert_eq!(report.corroborated, vec![seed.id]);
        assert_eq!(report.added.len(), 0);
    }
}

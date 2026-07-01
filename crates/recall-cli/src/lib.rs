use agent_cli::AgentProvider;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use recall_capture::{contradicts, curate, extract};
use recall_core::{Convention, Provenance, Source, Status};
use recall_inject::scope_label;
use recall_store::Store;
use std::path::Path;
use uuid::Uuid;

pub use recall_inject::parse_scope;

fn short(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}

pub fn cmd_learn(db: &Path, rule: &str, scope: &str, tags: Vec<String>) -> Result<String> {
    let store = Store::open(db)?;
    let scope = parse_scope(scope, &std::env::current_dir()?)?;
    let now = Utc::now();
    let c = Convention {
        id: Uuid::new_v4(),
        rule: rule.to_string(),
        rationale: None,
        scope,
        tags,
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
        confidence: 0.8,
        created_at: now,
        updated_at: now,
    };
    let id = store.add_curated(&c)?;
    Ok(format!("Learned [{}]: {}", short(&id), rule))
}

pub fn cmd_list(db: &Path) -> Result<String> {
    let store = Store::open(db)?;
    let convs = store.active()?;
    if convs.is_empty() {
        return Ok("No conventions yet. Teach one: recall learn \"...\"".to_string());
    }
    let mut s = String::new();
    for c in &convs {
        s.push_str(&format!(
            "[{}] {} ({})\n",
            short(&c.id),
            c.rule,
            scope_label(&c.scope)
        ));
    }
    Ok(s.trim_end().to_string())
}

fn find_by_prefix(store: &Store, prefix: &str) -> Result<Convention> {
    let matches: Vec<Convention> = store
        .all()?
        .into_iter()
        .filter(|c| c.id.to_string().starts_with(prefix))
        .collect();
    match matches.len() {
        0 => Err(anyhow!("no convention matches id '{prefix}'")),
        1 => Ok(matches.into_iter().next().unwrap()),
        n => Err(anyhow!(
            "'{prefix}' is ambiguous ({n} matches); use more characters"
        )),
    }
}

fn source_label(s: &Source) -> &'static str {
    match s {
        Source::ManualTeach => "manual teaching",
        Source::SessionDistill => "session distillation",
        Source::ImportedRules => "imported rules",
    }
}

pub fn cmd_why(db: &Path, id_prefix: &str) -> Result<String> {
    let store = Store::open(db)?;
    let c = find_by_prefix(&store, id_prefix)?;
    let mut s = String::new();
    s.push_str(&format!("[{}] {}\n", short(&c.id), c.rule));
    s.push_str(&format!("  scope:      {}\n", scope_label(&c.scope)));
    s.push_str(&format!(
        "  learned by: {}\n",
        source_label(&c.provenance.source)
    ));
    s.push_str(&format!(
        "  learned at: {}\n",
        c.provenance.learned_at.to_rfc3339()
    ));
    s.push_str(&format!("  status:     {:?}\n", c.status));
    s.push_str(&format!("  confidence: {:.2}", c.confidence));
    if let Some(by) = c.superseded_by {
        s.push_str(&format!("\n  superseded by: {}", short(&by)));
    }
    Ok(s)
}

pub fn cmd_forget(db: &Path, id_prefix: &str) -> Result<String> {
    let store = Store::open(db)?;
    let c = find_by_prefix(&store, id_prefix)?;
    store.retire(c.id)?;
    Ok(format!("Retired [{}]: {}", short(&c.id), c.rule))
}

pub fn cmd_status(db: &Path) -> Result<String> {
    let store = Store::open(db)?;
    let active = store.active()?.len();
    let total = store.all()?.len();
    let provider = match agent_cli::detect() {
        Some(p) => p.name().to_string(),
        None => "none (install Claude Code or Codex, or set ANTHROPIC_API_KEY)".to_string(),
    };
    Ok(format!(
        "Recall\n  db:       {}\n  active:   {active}\n  total:    {total}\n  provider: {provider}",
        db.display()
    ))
}

pub async fn cmd_capture(
    db: &Path,
    transcript_path: &Path,
    ctx: &recall_core::RepoContext,
    provider: &dyn AgentProvider,
) -> Result<String> {
    let transcript = std::fs::read_to_string(transcript_path)
        .with_context(|| format!("reading transcript {}", transcript_path.display()))?;
    let store = Store::open(db)?;
    let candidates = extract(&transcript, provider).await?;
    let report = curate(&candidates, &store, ctx, provider).await?;
    Ok(format!(
        "Captured: {} new (pending review), {} corroborated, {} potential conflicts.",
        report.added.len(),
        report.corroborated.len(),
        report.conflicts.len()
    ))
}

pub fn cmd_review_list(db: &Path) -> Result<String> {
    let store = Store::open(db)?;
    let pending: Vec<_> = store
        .all()?
        .into_iter()
        .filter(|c| c.status == Status::Pending)
        .collect();
    if pending.is_empty() {
        return Ok("No pending conventions to review.".to_string());
    }
    let mut s = String::from("Pending review (recall review --accept <id> | --reject <id>):\n");
    for c in &pending {
        s.push_str(&format!(
            "[{}] {} ({})\n",
            short(&c.id),
            c.rule,
            scope_label(&c.scope)
        ));
    }
    Ok(s.trim_end().to_string())
}

pub async fn cmd_review_accept(
    db: &Path,
    id_prefix: &str,
    provider: Option<&dyn AgentProvider>,
) -> Result<String> {
    let store = Store::open(db)?;
    let mut c = find_by_prefix(&store, id_prefix)?;
    // Only retire a same-scope active rule if an LLM judgment says it genuinely
    // contradicts the one being accepted -- not merely "has different text".
    // Without a provider, retire nothing else (safe no-op); manual review/accept
    // must still work even when no LLM provider is available.
    if let Some(p) = provider {
        let n = recall_core::normalize_rule(&c.rule);
        for e in store.active()? {
            if e.scope == c.scope
                && recall_core::normalize_rule(&e.rule) != n
                && contradicts(p, &c.rule, &e.rule).await
            {
                let mut sup = e.clone();
                sup.status = Status::Superseded;
                sup.superseded_by = Some(c.id);
                store.add(&sup)?;
            }
        }
    }
    c.status = Status::Active;
    c.confidence = c.confidence.max(0.8);
    store.add(&c)?;
    Ok(format!("Accepted [{}]: {}", short(&c.id), c.rule))
}

pub fn cmd_review_reject(db: &Path, id_prefix: &str) -> Result<String> {
    let store = Store::open(db)?;
    let mut c = find_by_prefix(&store, id_prefix)?;
    c.status = Status::Retired;
    store.add(&c)?;
    Ok(format!("Rejected [{}]: {}", short(&c.id), c.rule))
}

use serde_json::{json, Value};

/// SessionStart hook: emit the injection JSON (or "" if nothing relevant).
pub fn hook_session_start(db: &Path, stdin_json: &str) -> Result<String> {
    let v: Value = serde_json::from_str(stdin_json).unwrap_or(json!({}));
    let cwd = match v.get("cwd").and_then(|c| c.as_str()) {
        Some(c) => std::path::PathBuf::from(c),
        None => std::env::current_dir()?,
    };
    let store = Store::open(db)?;
    let convs = store.active()?;
    let ctx = recall_inject::detect_context(&cwd);
    let selected = recall_inject::select(&convs, &ctx, 4000);
    let rendered = recall_inject::render(&selected);
    if rendered.is_empty() {
        return Ok(String::new());
    }
    Ok(serde_json::to_string(&json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": rendered
        }
    }))?)
}

/// Stop hook: extract a usable transcript path (None for Codex's null / missing).
pub fn hook_stop_transcript(stdin_json: &str) -> Option<String> {
    serde_json::from_str::<Value>(stdin_json)
        .ok()
        .and_then(|v| {
            v.get("transcript_path")
                .and_then(|t| t.as_str())
                .map(String::from)
        })
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use recall_core::Scope;

    #[test]
    fn parse_scope_global_and_language() {
        assert_eq!(
            parse_scope("global", Path::new(".")).unwrap(),
            Scope::Global
        );
        assert_eq!(
            parse_scope("language:rust", Path::new(".")).unwrap(),
            Scope::Language("rust".into())
        );
    }

    #[test]
    fn parse_scope_rejects_unknown() {
        assert!(parse_scope("nonsense", Path::new(".")).is_err());
    }

    #[test]
    fn learn_then_list_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let msg = cmd_learn(&db, "Use early returns", "global", vec!["style".into()]).unwrap();
        assert!(msg.contains("Use early returns"));
        let listed = cmd_list(&db).unwrap();
        assert!(listed.contains("Use early returns"));
        assert!(listed.contains("global"));
    }

    #[test]
    fn why_and_forget_by_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let msg = cmd_learn(&db, "Use early returns", "global", vec![]).unwrap();
        // extract the 8-char id from "Learned [xxxxxxxx]: ..."
        let id = &msg[msg.find('[').unwrap() + 1..msg.find(']').unwrap()];

        let why = cmd_why(&db, id).unwrap();
        assert!(why.contains("Use early returns"));
        assert!(why.to_lowercase().contains("manual"));

        let forget = cmd_forget(&db, id).unwrap();
        assert!(forget.to_lowercase().contains("retired"));
        assert!(cmd_list(&db)
            .unwrap()
            .to_lowercase()
            .contains("no conventions"));
    }

    #[test]
    fn status_reports_counts() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        cmd_learn(&db, "Use early returns", "global", vec![]).unwrap();
        let status = cmd_status(&db).unwrap();
        assert!(status.contains("1"));
        assert!(status.to_lowercase().contains("active"));
    }

    #[tokio::test]
    async fn capture_then_review_accept_promotes() {
        use agent_cli::MockProvider;
        use serde_json::json;
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let transcript = tmp.path().join("t.txt");
        std::fs::write(&transcript, "user: always use early returns").unwrap();

        let extractor = MockProvider::new(json!({"conventions":[
            {"rule":"Use early returns","scope":"global","tags":[]}
        ]}));
        let ctx = recall_core::RepoContext::default();
        let msg = cmd_capture(&db, &transcript, &ctx, &extractor)
            .await
            .unwrap();
        assert!(msg.to_lowercase().contains("1"));

        let pending = cmd_review_list(&db).unwrap();
        assert!(pending.contains("Use early returns"));
        let id = &pending[pending.find('[').unwrap() + 1..pending.find(']').unwrap()];

        let accepted = cmd_review_accept(&db, id, None).await.unwrap();
        assert!(accepted.to_lowercase().contains("accept"));
        assert!(cmd_list(&db).unwrap().contains("Use early returns")); // now Active
    }

    #[tokio::test]
    async fn review_accept_without_provider_does_not_retire_unrelated_active_conventions() {
        use agent_cli::MockProvider;
        use serde_json::json;
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");

        // Seed two unrelated, pre-existing Active conventions in the same (global) scope.
        cmd_learn(&db, "Use tabs not spaces", "global", vec![]).unwrap();
        cmd_learn(&db, "Write tests for all new code", "global", vec![]).unwrap();

        // Capture an unrelated Pending convention.
        let transcript = tmp.path().join("t.txt");
        std::fs::write(&transcript, "user: prefer early returns").unwrap();
        let extractor = MockProvider::new(json!({"conventions":[
            {"rule":"Prefer early returns","scope":"global","tags":[]}
        ]}));
        cmd_capture(
            &db,
            &transcript,
            &recall_core::RepoContext::default(),
            &extractor,
        )
        .await
        .unwrap();

        let pending = cmd_review_list(&db).unwrap();
        assert!(pending.contains("Prefer early returns"));
        let id = &pending[pending.find('[').unwrap() + 1..pending.find(']').unwrap()];

        // Accept without a provider: this must NOT retire the unrelated Active conventions.
        let accepted = cmd_review_accept(&db, id, None).await.unwrap();
        assert!(accepted.to_lowercase().contains("accept"));

        let listed = cmd_list(&db).unwrap();
        assert!(listed.contains("Use tabs not spaces"));
        assert!(listed.contains("Write tests for all new code"));
        assert!(listed.contains("Prefer early returns"));
    }

    #[tokio::test]
    async fn review_reject_retires_pending() {
        use agent_cli::MockProvider;
        use serde_json::json;
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let transcript = tmp.path().join("t.txt");
        std::fs::write(&transcript, "x").unwrap();
        let extractor =
            MockProvider::new(json!({"conventions":[{"rule":"Nope","scope":"global","tags":[]}]}));
        cmd_capture(
            &db,
            &transcript,
            &recall_core::RepoContext::default(),
            &extractor,
        )
        .await
        .unwrap();
        let pending = cmd_review_list(&db).unwrap();
        let id = &pending[pending.find('[').unwrap() + 1..pending.find(']').unwrap()];
        cmd_review_reject(&db, id).unwrap();
        assert!(cmd_review_list(&db)
            .unwrap()
            .to_lowercase()
            .contains("no pending"));
    }

    #[test]
    fn hook_session_start_injects_active_conventions() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        cmd_learn(&db, "Use early returns", "global", vec![]).unwrap();
        let stdin = format!(
            r#"{{"cwd":"{}","hook_event_name":"SessionStart"}}"#,
            tmp.path().display()
        );
        let out = hook_session_start(&db, &stdin).unwrap();
        assert!(out.contains("hookSpecificOutput"));
        assert!(out.contains("SessionStart"));
        assert!(out.contains("Use early returns"));
    }

    #[test]
    fn hook_session_start_empty_when_no_conventions() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let out = hook_session_start(&db, r#"{"cwd":"/tmp"}"#).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn hook_stop_transcript_extracts_path_and_handles_null() {
        assert_eq!(
            hook_stop_transcript(r#"{"transcript_path":"/tmp/t.jsonl"}"#).as_deref(),
            Some("/tmp/t.jsonl")
        );
        assert_eq!(hook_stop_transcript(r#"{"transcript_path":null}"#), None);
        assert_eq!(hook_stop_transcript(r#"{}"#), None);
    }
}

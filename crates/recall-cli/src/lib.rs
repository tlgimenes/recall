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

use recall_capture::{check, Violation};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforceMode {
    Off,
    Warn,
    Block,
}

impl EnforceMode {
    pub fn from_env() -> Self {
        match std::env::var("RECALL_ENFORCE").as_deref() {
            Ok("block") => EnforceMode::Block,
            Ok("off") => EnforceMode::Off,
            _ => EnforceMode::Warn, // default
        }
    }
}

/// Pull the proposed file path + content from an edit tool's input. None for non-edit tools.
pub fn extract_proposed(tool_name: &str, tool_input: &Value) -> Option<(Option<String>, String)> {
    let is_edit = matches!(tool_name, "Write" | "Edit" | "MultiEdit" | "apply_patch");
    if !is_edit {
        return None;
    }
    let path = tool_input
        .get("file_path")
        .and_then(|p| p.as_str())
        .map(String::from);
    // MultiEdit nests its edits as tool_input.edits[], each with its own
    // new_string -- there is no top-level new_string for MultiEdit calls.
    // Concatenate every edit's proposed new_string so the full set of
    // changes in the call gets checked.
    if tool_name == "MultiEdit" {
        if let Some(edits) = tool_input.get("edits").and_then(|e| e.as_array()) {
            let joined = edits
                .iter()
                .filter_map(|e| e.get("new_string").and_then(|s| s.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            if !joined.is_empty() {
                return Some((path, joined));
            }
        }
        return None; // malformed/missing edits; nothing to check
    }
    let content = tool_input
        .get("content")
        .or_else(|| tool_input.get("new_string"))
        .or_else(|| tool_input.get("file_text"))
        .or_else(|| tool_input.get("input")) // apply_patch
        .and_then(|c| c.as_str())
        .map(String::from)?;
    Some((path, content))
}

/// Build the hook output JSON for the given violations + mode (None = stay silent / allow).
pub fn pre_tool_use_decision(violations: &[Violation], mode: EnforceMode) -> Option<String> {
    if violations.is_empty() || mode == EnforceMode::Off {
        return None;
    }
    let summary = violations
        .iter()
        .map(|v| format!("• {} — {}", v.rule, v.explanation))
        .collect::<Vec<_>>()
        .join("\n");
    let out = match mode {
        EnforceMode::Block => json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": format!("This edit violates your Recall conventions:\n{summary}")
            }
        }),
        EnforceMode::Warn => json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "additionalContext": format!("Heads up — this edit may violate your conventions:\n{summary}")
            }
        }),
        EnforceMode::Off => return None,
    };
    Some(out.to_string())
}

/// PreToolUse hook: check a proposed edit against active conventions and
/// report a decision (deny/warn) for the host to act on.
///
/// This function never returns `Err`: any internal failure (stdin parsing,
/// missing cwd, DB open/read errors, provider errors, ...) degrades to
/// `Ok(None)` -- i.e. "print nothing, allow the edit" -- so it can never wedge
/// a developer's live session. `Ok(Some(json))` is returned only when there
/// is an actual decision (deny or warn) to report.
pub async fn cmd_hook_pre_tool_use(
    db: &Path,
    stdin_json: &str,
    mode: EnforceMode,
    provider: &dyn AgentProvider,
) -> Result<Option<String>> {
    Ok(cmd_hook_pre_tool_use_inner(db, stdin_json, mode, provider)
        .await
        .unwrap_or(None))
}

async fn cmd_hook_pre_tool_use_inner(
    db: &Path,
    stdin_json: &str,
    mode: EnforceMode,
    provider: &dyn AgentProvider,
) -> Result<Option<String>> {
    if mode == EnforceMode::Off {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(stdin_json).unwrap_or(json!({}));
    let tool_name = v.get("tool_name").and_then(|t| t.as_str()).unwrap_or("");
    let tool_input = v.get("tool_input").cloned().unwrap_or(json!({}));
    let (path, content) = match extract_proposed(tool_name, &tool_input) {
        Some(x) => x,
        None => return Ok(None), // not an edit; allow
    };
    let cwd = v
        .get("cwd")
        .and_then(|c| c.as_str())
        .map(std::path::PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let store = Store::open(db)?;
    let mut ctx = recall_inject::detect_context(&cwd);
    // Narrow languages by the edited file extension when present.
    if let Some(p) = &path {
        if let Some(lang) = lang_for_path(p) {
            ctx.languages = vec![lang];
        }
    }
    let convs = recall_inject::select(&store.active()?, &ctx, 4000);
    let violations = check(&content, &convs, provider).await.unwrap_or_default(); // fail open
    Ok(pre_tool_use_decision(&violations, mode))
}

fn lang_for_path(p: &str) -> Option<String> {
    let ext = std::path::Path::new(p).extension()?.to_str()?;
    let l = match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        _ => return None,
    };
    Some(l.to_string())
}

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

    #[test]
    fn extract_proposed_handles_write_and_edit() {
        let (p, c) = extract_proposed(
            "Write",
            &serde_json::json!({"file_path":"a.ts","content":"x"}),
        )
        .unwrap();
        assert_eq!(p.as_deref(), Some("a.ts"));
        assert_eq!(c, "x");
        let (_, c2) = extract_proposed(
            "Edit",
            &serde_json::json!({"file_path":"a.ts","new_string":"y"}),
        )
        .unwrap();
        assert_eq!(c2, "y");
        assert!(extract_proposed("Bash", &serde_json::json!({"command":"ls"})).is_none());
    }

    #[test]
    fn extract_proposed_handles_multi_edit_nested_edits() {
        // Claude Code's real MultiEdit schema nests edits as tool_input.edits[],
        // each with its own old_string/new_string -- there is no top-level
        // new_string for MultiEdit calls.
        let (p, c) = extract_proposed(
            "MultiEdit",
            &serde_json::json!({
                "file_path": "a.ts",
                "edits": [
                    {"old_string": "foo", "new_string": "bar the first edit"},
                    {"old_string": "baz", "new_string": "qux the second edit"}
                ]
            }),
        )
        .unwrap();
        assert_eq!(p.as_deref(), Some("a.ts"));
        assert!(c.contains("bar the first edit"));
        assert!(c.contains("qux the second edit"));
    }

    #[test]
    fn extract_proposed_multi_edit_missing_edits_returns_none() {
        assert!(extract_proposed("MultiEdit", &serde_json::json!({"file_path":"a.ts"})).is_none());
        assert!(extract_proposed(
            "MultiEdit",
            &serde_json::json!({"file_path":"a.ts","edits":[]})
        )
        .is_none());
    }

    #[test]
    fn decision_block_denies_when_violations() {
        let v = vec![Violation {
            rule: "No barrels".into(),
            explanation: "re-export".into(),
        }];
        let out = pre_tool_use_decision(&v, EnforceMode::Block).unwrap();
        assert!(out.contains("\"permissionDecision\":\"deny\""));
        assert!(out.contains("No barrels"));
    }

    #[test]
    fn decision_warn_allows_with_context() {
        let v = vec![Violation {
            rule: "No barrels".into(),
            explanation: "re-export".into(),
        }];
        let out = pre_tool_use_decision(&v, EnforceMode::Warn).unwrap();
        assert!(out.contains("additionalContext"));
        assert!(!out.contains("deny"));
    }

    #[test]
    fn decision_none_when_no_violations_or_off() {
        assert!(pre_tool_use_decision(&[], EnforceMode::Block).is_none());
        let v = vec![Violation {
            rule: "x".into(),
            explanation: "y".into(),
        }];
        assert!(pre_tool_use_decision(&v, EnforceMode::Off).is_none());
    }

    #[tokio::test]
    async fn hook_pre_tool_use_fails_open_when_store_open_fails() {
        use agent_cli::MockProvider;

        // Force Store::open to fail: `db`'s parent path component is a
        // regular file, not a directory, so `create_dir_all` cannot create
        // it and `Connection::open` then fails because the directory does
        // not exist. This simulates DB corruption / disk full / a bogus
        // path -- none of which should ever wedge the hook.
        let blocking_file = tempfile::NamedTempFile::new().unwrap();
        let db = blocking_file.path().join("sub").join("recall.db");
        assert!(
            Store::open(&db).is_err(),
            "test setup invalid: Store::open unexpectedly succeeded for {}",
            db.display()
        );

        let provider = MockProvider::new(json!({"violations": []}));
        let stdin = serde_json::json!({
            "tool_name": "Write",
            "tool_input": {"file_path": "a.rs", "content": "fn main() {}"},
            "cwd": "/tmp"
        })
        .to_string();

        let result = cmd_hook_pre_tool_use(&db, &stdin, EnforceMode::Block, &provider).await;
        assert!(
            result.is_ok(),
            "cmd_hook_pre_tool_use must never propagate Err, got {result:?}"
        );
        assert_eq!(result.unwrap(), None);
    }
}

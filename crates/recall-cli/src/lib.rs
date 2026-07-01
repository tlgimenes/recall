use anyhow::{anyhow, Result};
use chrono::Utc;
use recall_core::{Convention, Provenance, Scope, Source, Status};
use recall_inject::{detect_context, scope_label};
use recall_store::Store;
use std::path::Path;
use uuid::Uuid;

/// Parse a `--scope` string into a Scope. `repo`/`branch` resolve from cwd git.
pub fn parse_scope(s: &str) -> Result<Scope> {
    if s == "global" {
        return Ok(Scope::Global);
    }
    if let Some(lang) = s.strip_prefix("language:") {
        if lang.is_empty() {
            return Err(anyhow!("language scope needs a name, e.g. language:rust"));
        }
        return Ok(Scope::Language(lang.to_string()));
    }
    if s == "repo" || s == "branch" {
        let ctx = detect_context(&std::env::current_dir()?);
        let remote = ctx.remote_id.ok_or_else(|| {
            anyhow!("not in a git repo with an 'origin' remote; can't use --scope {s}")
        })?;
        if s == "repo" {
            return Ok(Scope::Repo { remote_id: remote });
        }
        let branch = ctx
            .branch
            .ok_or_else(|| anyhow!("can't detect the current branch"))?;
        return Ok(Scope::Branch {
            remote_id: remote,
            branch,
        });
    }
    Err(anyhow!(
        "unknown scope '{s}': use global | repo | branch | language:<lang>"
    ))
}

fn short(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}

pub fn cmd_learn(db: &Path, rule: &str, scope: &str, tags: Vec<String>) -> Result<String> {
    let store = Store::open(db)?;
    let scope = parse_scope(scope)?;
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
    Ok(format!(
        "Recall\n  db:       {}\n  active:   {active}\n  total:    {total}\n  provider: not configured (LLM capture arrives in Plan 2)",
        db.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use recall_core::Scope;

    #[test]
    fn parse_scope_global_and_language() {
        assert_eq!(parse_scope("global").unwrap(), Scope::Global);
        assert_eq!(
            parse_scope("language:rust").unwrap(),
            Scope::Language("rust".into())
        );
    }

    #[test]
    fn parse_scope_rejects_unknown() {
        assert!(parse_scope("nonsense").is_err());
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
}

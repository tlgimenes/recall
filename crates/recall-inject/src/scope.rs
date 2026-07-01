use crate::detect_context;
use anyhow::{anyhow, Result};
use recall_core::Scope;
use std::path::Path;

/// Parse a `--scope` string into a `Scope`. `repo`/`branch` resolve from the
/// git context at `cwd`.
pub fn parse_scope(s: &str, cwd: &Path) -> Result<Scope> {
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
        let ctx = detect_context(cwd);
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

#[cfg(test)]
mod tests {
    use super::parse_scope;
    use recall_core::Scope;
    use std::path::Path;

    #[test]
    fn parses_global_and_language() {
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
    fn rejects_unknown() {
        assert!(parse_scope("bogus", Path::new(".")).is_err());
    }
}

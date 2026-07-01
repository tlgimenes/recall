use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The scope a convention applies to. Ordered least → most specific.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Scope {
    Global,
    Language(String),
    Repo { remote_id: String },
    Branch { remote_id: String, branch: String },
}

impl Scope {
    /// 0 = least specific (Global) … 3 = most specific (Branch).
    pub fn specificity(&self) -> u8 {
        match self {
            Scope::Global => 0,
            Scope::Language(_) => 1,
            Scope::Repo { .. } => 2,
            Scope::Branch { .. } => 3,
        }
    }

    /// Whether this scope applies in the given repo context.
    pub fn matches(&self, ctx: &RepoContext) -> bool {
        match self {
            Scope::Global => true,
            Scope::Language(l) => ctx.languages.iter().any(|x| x.eq_ignore_ascii_case(l)),
            Scope::Repo { remote_id } => ctx.remote_id.as_deref() == Some(remote_id.as_str()),
            Scope::Branch { remote_id, branch } => {
                ctx.remote_id.as_deref() == Some(remote_id.as_str())
                    && ctx.branch.as_deref() == Some(branch.as_str())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Pending,
    Active,
    Superseded,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    SessionDistill,
    ManualTeach,
    ImportedRules,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: Source,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub agent: Option<String>,
    pub excerpt: Option<String>,
    pub learned_at: DateTime<Utc>,
}

/// A single curated, compact convention — the unit of memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Convention {
    pub id: Uuid,
    pub rule: String,
    pub rationale: Option<String>,
    pub scope: Scope,
    pub tags: Vec<String>,
    pub provenance: Provenance,
    pub status: Status,
    pub superseded_by: Option<Uuid>,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The resolved context Recall injects conventions for.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RepoContext {
    pub remote_id: Option<String>,
    pub branch: Option<String>,
    pub languages: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> RepoContext {
        RepoContext {
            remote_id: Some("github.com/me/app".into()),
            branch: Some("main".into()),
            languages: vec!["rust".into()],
        }
    }

    #[test]
    fn specificity_orders_global_to_branch() {
        assert!(Scope::Global.specificity() < Scope::Language("rust".into()).specificity());
        assert!(
            Scope::Language("rust".into()).specificity()
                < Scope::Repo {
                    remote_id: "x".into()
                }
                .specificity()
        );
        assert!(
            Scope::Repo {
                remote_id: "x".into()
            }
            .specificity()
                < Scope::Branch {
                    remote_id: "x".into(),
                    branch: "y".into()
                }
                .specificity()
        );
    }

    #[test]
    fn global_matches_any_context() {
        assert!(Scope::Global.matches(&ctx()));
    }

    #[test]
    fn language_matches_case_insensitively() {
        assert!(Scope::Language("Rust".into()).matches(&ctx()));
        assert!(!Scope::Language("go".into()).matches(&ctx()));
    }

    #[test]
    fn repo_and_branch_match_exactly() {
        assert!(Scope::Repo {
            remote_id: "github.com/me/app".into()
        }
        .matches(&ctx()));
        assert!(!Scope::Repo {
            remote_id: "github.com/me/other".into()
        }
        .matches(&ctx()));
        assert!(Scope::Branch {
            remote_id: "github.com/me/app".into(),
            branch: "main".into()
        }
        .matches(&ctx()));
        assert!(!Scope::Branch {
            remote_id: "github.com/me/app".into(),
            branch: "dev".into()
        }
        .matches(&ctx()));
    }
}

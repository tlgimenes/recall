use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use recall_inject::{detect_context, render, select};
use recall_store::Store;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::transport::stdio;
use rmcp::{schemars, tool, tool_handler, tool_router};
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt};

const BUDGET_CHARS: usize = 4000;

/// Plain, testable handler for the `recall_list` tool.
pub fn handle_list(db_path: &Path) -> Result<String> {
    let store = Store::open(db_path)?;
    let convs = store.active()?;
    let rendered = render(&convs);
    Ok(non_empty(rendered))
}

/// Plain, testable handler for the `recall_conventions` tool.
pub fn handle_conventions(db_path: &Path, cwd: Option<&str>) -> Result<String> {
    let store = Store::open(db_path)?;
    let convs = store.active()?;
    let dir = match cwd {
        Some(c) => PathBuf::from(c),
        None => std::env::current_dir()?,
    };
    let ctx = detect_context(&dir);
    let selected = select(&convs, &ctx, BUDGET_CHARS);
    Ok(non_empty(render(&selected)))
}

/// Plain, testable handler for the `recall_learn` tool.
pub fn handle_learn(
    db_path: &Path,
    rule: &str,
    scope: &str,
    tags: Vec<String>,
    cwd: Option<&str>,
) -> Result<String> {
    use chrono::Utc;
    use recall_core::{Convention, Provenance, Source, Status};
    use uuid::Uuid;

    let store = Store::open(db_path)?;
    let dir = match cwd {
        Some(c) => PathBuf::from(c),
        None => std::env::current_dir()?,
    };
    let scope = recall_inject::parse_scope(scope, &dir)?;
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
    store.add_curated(&c)?;
    Ok(format!("Learned: {rule}"))
}

fn non_empty(s: String) -> String {
    if s.is_empty() {
        "No conventions recorded yet. Teach one with: recall learn \"...\"".to_string()
    } else {
        s
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConventionsParams {
    /// Working directory of the current repo. Defaults to the server's cwd.
    pub cwd: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LearnParams {
    /// The convention, imperative and compact, e.g. "Import directly; no barrel files".
    pub rule: String,
    /// global | repo | branch | language:<lang>. Defaults to global.
    pub scope: Option<String>,
    /// Optional tags.
    pub tags: Option<Vec<String>>,
    /// Working directory (for repo/branch scope). Defaults to the server cwd.
    pub cwd: Option<String>,
}

#[derive(Clone)]
pub struct Recall {
    db_path: Arc<PathBuf>,
    // Read by the #[tool_handler] macro's generated dispatch code; the
    // dead-code lint can't see that usage through the macro expansion.
    #[allow(dead_code)]
    tool_router: ToolRouter<Recall>,
}

impl Recall {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path: Arc::new(db_path),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl Recall {
    #[tool(
        description = "Get the developer's coding conventions relevant to the current repo, branch, and languages. Call this before writing code."
    )]
    fn recall_conventions(
        &self,
        Parameters(p): Parameters<ConventionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let text = handle_conventions(&self.db_path, p.cwd.as_deref())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "List all of the developer's active coding conventions across every scope."
    )]
    fn recall_list(&self) -> Result<CallToolResult, McpError> {
        let text = handle_list(&self.db_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Record a durable coding convention the developer wants followed. Call this when they state a preference or correct you (e.g. 'always X', 'never Y', 'we use Z here')."
    )]
    fn recall_learn(
        &self,
        Parameters(p): Parameters<LearnParams>,
    ) -> Result<CallToolResult, McpError> {
        let text = handle_learn(
            &self.db_path,
            &p.rule,
            p.scope.as_deref().unwrap_or("global"),
            p.tags.unwrap_or_default(),
            p.cwd.as_deref(),
        )
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler]
impl ServerHandler for Recall {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "Recall is the developer's personal coding-convention brain. \
                 Call recall_conventions before writing or editing code so you \
                 follow how this developer likes code written."
                    .to_string(),
            )
    }
}

/// Run the Recall MCP server over stdio until the client disconnects.
pub async fn run_stdio(db_path: PathBuf) -> Result<()> {
    let service = Recall::new(db_path).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{handle_conventions, handle_list};
    use chrono::Utc;
    use recall_core::*;
    use recall_store::Store;
    use uuid::Uuid;

    fn seed(db: &std::path::Path) {
        let store = Store::open(db).unwrap();
        let now = Utc::now();
        let c = Convention {
            id: Uuid::new_v4(),
            rule: "Use early returns".into(),
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
        };
        store.add(&c).unwrap();
    }

    #[test]
    fn handle_list_returns_seeded_rule() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        seed(&db);
        let out = handle_list(&db).unwrap();
        assert!(out.contains("Use early returns"));
    }

    #[test]
    fn handle_conventions_includes_global_rule() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        seed(&db);
        // A non-git cwd: remote_id is None, but Global conventions still match.
        let cwd = tempfile::tempdir().unwrap();
        let out = handle_conventions(&db, Some(cwd.path().to_str().unwrap())).unwrap();
        assert!(out.contains("Use early returns"));
    }

    #[test]
    fn handle_list_empty_db_is_friendly() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let out = handle_list(&db).unwrap();
        assert!(out.to_lowercase().contains("no conventions"));
    }

    #[test]
    fn handle_learn_then_list_shows_rule() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("recall.db");
        let msg = super::handle_learn(&db, "Use early returns", "global", vec![], None).unwrap();
        assert!(msg.contains("Use early returns"));
        assert!(super::handle_list(&db)
            .unwrap()
            .contains("Use early returns"));
    }
}

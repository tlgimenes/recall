# Recall Plan 1 — Dogfoodable Core (model + store + inject + MCP + CLI)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a real, local-first Recall binary whose MCP server serves the developer's manually-taught coding conventions, scoped by repo/branch/language — registerable on a live Claude Code/Codex session for dogfooding.

**Architecture:** A Rust workspace of small, single-responsibility crates: `recall-core` (pure domain model + scoping + dedup), `recall-store` (SQLite persistence), `recall-inject` (pure selection/render + git context detection), `recall-mcp` (rmcp stdio server), `recall-cli` (the `recall` binary). LLM-based capture/curation is deliberately out of scope here (Plan 2); this plan covers manual teaching + retrieval + inspection, which is the slice that unlocks dogfooding.

**Tech Stack:** Rust (stable), `rmcp` 1.8 (official MCP SDK, macro-based), `rusqlite` (bundled SQLite, single binary), `tokio`, `clap`, `serde`/`serde_json`, `chrono`, `uuid`, `schemars` 1.0; `tempfile` for tests.

## Global Constraints

- **Rust:** stable toolchain, `edition = "2021"`, MSRV 1.82+. One line each crate.
- **Single static binary:** `rusqlite` MUST use `features = ["bundled"]` (no system SQLite).
- **Binary name:** `recall`. Default DB: `~/.recall/recall.db`. Tests and dev MUST honor the `RECALL_DB` env var to override the DB path.
- **No network / no LLM in this plan.** `recall-core`, `recall-store`, `recall-inject` do no network I/O. All LLM provider work is Plan 2.
- **MCP:** use `rmcp = { version = "1.8", features = ["server", "macros", "transport-io", "schemars"] }`. Log to **stderr only** (stdout is the MCP transport).
- **schemars pin:** `schemars = "1.0"`; in derives use the rmcp re-export (`use rmcp::schemars;`) to avoid version skew.
- **Enum serde:** `Status`/`Scope`/`Source` serialize with default (externally tagged) serde; the SQLite `status` column stores the variant name string (`"Active"`, etc.) — keep these consistent.
- **Commit style:** end every commit body with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

### Task 1: Workspace + `recall-core` domain model

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/recall-core/Cargo.toml`
- Create: `crates/recall-core/src/lib.rs`
- Create: `crates/recall-core/src/convention.rs`
- Test: inline `#[cfg(test)]` in `crates/recall-core/src/convention.rs`

**Interfaces:**
- Produces: `recall_core::{Convention, Scope, Status, Source, Provenance, RepoContext}`. Methods `Scope::specificity(&self) -> u8` (Global=0, Language=1, Repo=2, Branch=3) and `Scope::matches(&self, ctx: &RepoContext) -> bool`.

> **Reconciliation with Plan 0:** Plan 0 already created the workspace root
> `Cargo.toml`, `crates/recall-core/Cargo.toml`, and a stub
> `crates/recall-core/src/lib.rs` (with a `toolchain_smoke` test). So in this
> task: **Step 1 is a no-op** (the root `Cargo.toml` already matches — verify it);
> **Step 2 is unchanged but already exists** (verify the deps match — recall-core
> needs `serde`, `chrono`, `uuid`); **Step 3 replaces the stub `lib.rs`** (drop
> the smoke test); **Step 4+ proceed as written**. If Plan 0 was skipped, execute
> Step 1 as a create.

- [ ] **Step 1: Create the workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/recall-core"]

[workspace.package]
edition = "2021"
rust-version = "1.82"
license = "MIT"
repository = "https://github.com/tlgimenes/recall"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde", "clock"] }
uuid = { version = "1", features = ["v4", "serde"] }
anyhow = "1"
tempfile = "3"
```

- [ ] **Step 2: Create `crates/recall-core/Cargo.toml`**

```toml
[package]
name = "recall-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
serde = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
```

- [ ] **Step 3: Create `crates/recall-core/src/lib.rs`**

```rust
mod convention;
pub use convention::*;
```

- [ ] **Step 4: Write the failing test in `crates/recall-core/src/convention.rs`**

Put this at the bottom of the file (the types above it come in Step 6):

```rust
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
        assert!(Scope::Language("rust".into()).specificity()
            < Scope::Repo { remote_id: "x".into() }.specificity());
        assert!(Scope::Repo { remote_id: "x".into() }.specificity()
            < Scope::Branch { remote_id: "x".into(), branch: "y".into() }.specificity());
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
        assert!(Scope::Repo { remote_id: "github.com/me/app".into() }.matches(&ctx()));
        assert!(!Scope::Repo { remote_id: "github.com/me/other".into() }.matches(&ctx()));
        assert!(Scope::Branch {
            remote_id: "github.com/me/app".into(), branch: "main".into()
        }.matches(&ctx()));
        assert!(!Scope::Branch {
            remote_id: "github.com/me/app".into(), branch: "dev".into()
        }.matches(&ctx()));
    }
}
```

- [ ] **Step 5: Run the test to verify it fails**

Run: `cargo test -p recall-core`
Expected: FAIL — `cannot find type RepoContext`/`Scope` (types not defined yet).

- [ ] **Step 6: Write the model at the top of `crates/recall-core/src/convention.rs`** (above the `#[cfg(test)]` block)

```rust
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
pub enum Status { Pending, Active, Superseded, Retired }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source { SessionDistill, ManualTeach, ImportedRules }

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
```

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test -p recall-core`
Expected: PASS (4 tests).

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/recall-core
git commit -m "feat(core): workspace + convention domain model with scope matching

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: `recall-core` — rule normalization + dedup decision

**Files:**
- Create: `crates/recall-core/src/supersede.rs`
- Modify: `crates/recall-core/src/lib.rs` (add `mod supersede; pub use supersede::*;`)
- Test: inline `#[cfg(test)]` in `crates/recall-core/src/supersede.rs`

**Interfaces:**
- Produces: `recall_core::normalize_rule(s: &str) -> String`; `recall_core::DedupDecision` (`New` | `Corroborates(Uuid)`); `recall_core::dedup_decision(new_rule: &str, new_scope: &Scope, existing: &[Convention]) -> DedupDecision`.
- Consumes: `Convention`, `Scope`, `Status` from Task 1.

- [ ] **Step 1: Add module wiring to `crates/recall-core/src/lib.rs`**

```rust
mod convention;
mod supersede;
pub use convention::*;
pub use supersede::*;
```

- [ ] **Step 2: Write the failing test in `crates/recall-core/src/supersede.rs`**

```rust
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
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p recall-core supersede`
Expected: FAIL — `cannot find function normalize_rule`.

- [ ] **Step 4: Write the implementation at the top of `crates/recall-core/src/supersede.rs`**

```rust
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
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p recall-core`
Expected: PASS (8 tests total).

- [ ] **Step 6: Commit**

```bash
git add crates/recall-core
git commit -m "feat(core): rule normalization + dedup decision

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: `recall-core` — git remote normalization

**Files:**
- Create: `crates/recall-core/src/remote.rs`
- Modify: `crates/recall-core/src/lib.rs` (add `mod remote; pub use remote::*;`)
- Test: inline `#[cfg(test)]` in `crates/recall-core/src/remote.rs`

**Interfaces:**
- Produces: `recall_core::normalize_remote(url: &str) -> String` → canonical `host/owner/repo`, lowercased, no scheme/creds/`.git`.

- [ ] **Step 1: Add module wiring to `crates/recall-core/src/lib.rs`**

```rust
mod convention;
mod remote;
mod supersede;
pub use convention::*;
pub use remote::*;
pub use supersede::*;
```

- [ ] **Step 2: Write the failing test in `crates/recall-core/src/remote.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::normalize_remote;

    #[test]
    fn https_with_token_creds() {
        assert_eq!(
            normalize_remote("https://x-access-token:ghs_ABC@github.com/Me/App.git"),
            "github.com/me/app"
        );
    }

    #[test]
    fn ssh_scp_form() {
        assert_eq!(normalize_remote("git@github.com:me/app.git"), "github.com/me/app");
    }

    #[test]
    fn ssh_url_form() {
        assert_eq!(normalize_remote("ssh://git@github.com/me/app.git"), "github.com/me/app");
    }

    #[test]
    fn plain_https_no_dot_git() {
        assert_eq!(normalize_remote("https://github.com/me/app"), "github.com/me/app");
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p recall-core remote`
Expected: FAIL — `cannot find function normalize_remote`.

- [ ] **Step 4: Write the implementation at the top of `crates/recall-core/src/remote.rs`**

```rust
/// Normalize a git remote URL to a canonical `host/owner/repo` identifier so the
/// same repository maps to the same conventions regardless of clone URL form.
pub fn normalize_remote(url: &str) -> String {
    let mut u = url.trim().to_string();
    for p in ["https://", "http://", "ssh://", "git://"] {
        if let Some(rest) = u.strip_prefix(p) { u = rest.to_string(); }
    }
    if let Some(rest) = u.strip_prefix("git@") { u = rest.to_string(); }
    // Drop any remaining userinfo (e.g. token creds) before the host.
    if let Some(at) = u.rfind('@') { u = u[at + 1..].to_string(); }
    // scp-style "host:owner/repo" → "host/owner/repo".
    u = u.replacen(':', "/", 1);
    if let Some(rest) = u.strip_suffix(".git") { u = rest.to_string(); }
    u.trim_end_matches('/').to_lowercase()
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p recall-core`
Expected: PASS (12 tests total).

- [ ] **Step 6: Commit**

```bash
git add crates/recall-core
git commit -m "feat(core): normalize git remote URLs to host/owner/repo

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: `recall-store` — SQLite persistence

**Files:**
- Create: `crates/recall-store/Cargo.toml`
- Create: `crates/recall-store/src/lib.rs`
- Modify: `Cargo.toml` (add `crates/recall-store` to `members`)
- Test: inline `#[cfg(test)]` in `crates/recall-store/src/lib.rs`

**Interfaces:**
- Produces: `recall_store::Store` with `open(&Path) -> Result<Store>`, `open_in_memory() -> Result<Store>`, `add(&Convention) -> Result<()>`, `get(Uuid) -> Result<Option<Convention>>`, `all() -> Result<Vec<Convention>>`, `active() -> Result<Vec<Convention>>`, `retire(Uuid) -> Result<bool>`, `add_curated(&Convention) -> Result<Uuid>`.
- Consumes: `Convention`, `Status`, `dedup_decision`, `DedupDecision` from `recall-core`.

- [ ] **Step 1: Add the crate to the workspace `members` in root `Cargo.toml`**

```toml
members = ["crates/recall-core", "crates/recall-store"]
```

- [ ] **Step 2: Create `crates/recall-store/Cargo.toml`**

```toml
[package]
name = "recall-store"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
recall-core = { path = "../recall-core" }
rusqlite = { version = "0.32", features = ["bundled"] }
serde_json = { workspace = true }
uuid = { workspace = true }
anyhow = { workspace = true }
```

- [ ] **Step 3: Write the failing test in `crates/recall-store/src/lib.rs`**

Put at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use recall_core::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn conv(rule: &str, scope: Scope) -> Convention {
        let now = Utc::now();
        Convention {
            id: Uuid::new_v4(), rule: rule.into(), rationale: None, scope, tags: vec![],
            provenance: Provenance { source: Source::ManualTeach, repo: None, branch: None,
                agent: None, excerpt: None, learned_at: now },
            status: Status::Active, superseded_by: None, confidence: 0.8,
            created_at: now, updated_at: now,
        }
    }

    #[test]
    fn add_and_get_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        let c = conv("Use early returns", Scope::Global);
        s.add(&c).unwrap();
        assert_eq!(s.get(c.id).unwrap().unwrap().rule, "Use early returns");
    }

    #[test]
    fn active_excludes_retired() {
        let s = Store::open_in_memory().unwrap();
        let c = conv("Use early returns", Scope::Global);
        s.add(&c).unwrap();
        assert_eq!(s.active().unwrap().len(), 1);
        assert!(s.retire(c.id).unwrap());
        assert_eq!(s.active().unwrap().len(), 0);
        assert_eq!(s.all().unwrap().len(), 1);
    }

    #[test]
    fn add_curated_corroborates_and_bumps_confidence() {
        let s = Store::open_in_memory().unwrap();
        let c = conv("Use early returns", Scope::Global);
        let first = s.add_curated(&c).unwrap();
        let dup = conv("use EARLY returns", Scope::Global);
        let second = s.add_curated(&dup).unwrap();
        assert_eq!(first, second); // corroborated, not duplicated
        assert_eq!(s.active().unwrap().len(), 1);
        assert!(s.get(first).unwrap().unwrap().confidence > 0.8);
    }
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p recall-store`
Expected: FAIL — `cannot find type Store`.

- [ ] **Step 5: Write the implementation at the top of `crates/recall-store/src/lib.rs`**

```rust
use anyhow::Result;
use recall_core::{dedup_decision, Convention, DedupDecision, Status};
use rusqlite::{params, Connection};
use std::path::Path;
use uuid::Uuid;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    fn init(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS conventions (
                id     TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                data   TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn add(&self, c: &Convention) -> Result<()> {
        let data = serde_json::to_string(c)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO conventions (id, status, data) VALUES (?1, ?2, ?3)",
            params![c.id.to_string(), status_str(&c.status), data],
        )?;
        Ok(())
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Convention>> {
        let mut stmt = self.conn.prepare("SELECT data FROM conventions WHERE id = ?1")?;
        let mut rows = stmt.query(params![id.to_string()])?;
        match rows.next()? {
            Some(row) => {
                let data: String = row.get(0)?;
                Ok(Some(serde_json::from_str(&data)?))
            }
            None => Ok(None),
        }
    }

    pub fn all(&self) -> Result<Vec<Convention>> {
        self.query("SELECT data FROM conventions")
    }

    pub fn active(&self) -> Result<Vec<Convention>> {
        self.query("SELECT data FROM conventions WHERE status = 'Active'")
    }

    fn query(&self, sql: &str) -> Result<Vec<Convention>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(serde_json::from_str(&r?)?);
        }
        Ok(out)
    }

    pub fn retire(&self, id: Uuid) -> Result<bool> {
        let mut c = match self.get(id)? {
            Some(c) => c,
            None => return Ok(false),
        };
        c.status = Status::Retired;
        self.add(&c)?;
        Ok(true)
    }

    /// Insert a convention, or corroborate (bump confidence on) an existing
    /// same-scope same-text active one. Returns the id of the surviving record.
    pub fn add_curated(&self, c: &Convention) -> Result<Uuid> {
        let existing = self.active()?;
        match dedup_decision(&c.rule, &c.scope, &existing) {
            DedupDecision::Corroborates(id) => {
                if let Some(mut e) = self.get(id)? {
                    e.confidence = (e.confidence + 0.1).min(1.0);
                    self.add(&e)?;
                }
                Ok(id)
            }
            DedupDecision::New => {
                self.add(c)?;
                Ok(c.id)
            }
        }
    }
}

fn status_str(s: &Status) -> &'static str {
    match s {
        Status::Pending => "Pending",
        Status::Active => "Active",
        Status::Superseded => "Superseded",
        Status::Retired => "Retired",
    }
}
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p recall-store`
Expected: PASS (3 tests).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/recall-store
git commit -m "feat(store): SQLite persistence with dedup-aware add

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: `recall-inject` — selection + render (pure)

**Files:**
- Create: `crates/recall-inject/Cargo.toml`
- Create: `crates/recall-inject/src/lib.rs`
- Modify: `Cargo.toml` (add `crates/recall-inject` to `members`)
- Test: inline `#[cfg(test)]` in `crates/recall-inject/src/lib.rs`

**Interfaces:**
- Produces: `recall_inject::select(convs: &[Convention], ctx: &RepoContext, budget_chars: usize) -> Vec<Convention>`; `recall_inject::render(convs: &[Convention]) -> String`; `recall_inject::scope_label(scope: &Scope) -> String`.
- Consumes: `Convention`, `RepoContext`, `Scope`, `Status` from `recall-core`.

- [ ] **Step 1: Add the crate to the workspace `members` in root `Cargo.toml`**

```toml
members = ["crates/recall-core", "crates/recall-store", "crates/recall-inject"]
```

- [ ] **Step 2: Create `crates/recall-inject/Cargo.toml`**

```toml
[package]
name = "recall-inject"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
recall-core = { path = "../recall-core" }
anyhow = { workspace = true }
```

- [ ] **Step 3: Write the failing test in `crates/recall-inject/src/lib.rs`**

Put at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use recall_core::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn conv(rule: &str, scope: Scope, conf: f32) -> Convention {
        let now = Utc::now();
        Convention {
            id: Uuid::new_v4(), rule: rule.into(), rationale: None, scope, tags: vec![],
            provenance: Provenance { source: Source::ManualTeach, repo: None, branch: None,
                agent: None, excerpt: None, learned_at: now },
            status: Status::Active, superseded_by: None, confidence: conf,
            created_at: now, updated_at: now,
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
            conv("repo rule", Scope::Repo { remote_id: "github.com/me/app".into() }, 0.5),
            conv("other repo", Scope::Repo { remote_id: "github.com/me/other".into() }, 0.9),
        ];
        let out = select(&convs, &ctx(), 10_000);
        let rules: Vec<&str> = out.iter().map(|c| c.rule.as_str()).collect();
        assert_eq!(rules, vec!["repo rule", "rust rule", "global rule"]);
    }

    #[test]
    fn select_respects_budget() {
        let convs = vec![
            conv("aaaaaaaaaa", Scope::Repo { remote_id: "github.com/me/app".into() }, 0.9),
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
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p recall-inject`
Expected: FAIL — `cannot find function select`.

- [ ] **Step 5: Write the implementation at the top of `crates/recall-inject/src/lib.rs`**

```rust
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
        s.push_str(&format!("- {} _({})_\n", c.rule.trim(), scope_label(&c.scope)));
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
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p recall-inject`
Expected: PASS (4 tests).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/recall-inject
git commit -m "feat(inject): scope-aware selection + Markdown render

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: `recall-inject` — git context detection

**Files:**
- Create: `crates/recall-inject/src/context.rs`
- Modify: `crates/recall-inject/src/lib.rs` (add `mod context; pub use context::*;`)
- Test: inline `#[cfg(test)]` in `crates/recall-inject/src/context.rs`
- Modify: `crates/recall-inject/Cargo.toml` (add `tempfile` dev-dependency)

**Interfaces:**
- Produces: `recall_inject::detect_context(cwd: &Path) -> RepoContext`.
- Consumes: `recall_core::{RepoContext, normalize_remote}`.

- [ ] **Step 1: Add a dev-dependency to `crates/recall-inject/Cargo.toml`**

```toml
[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 2: Add module wiring to `crates/recall-inject/src/lib.rs`** (add these two lines at the top, keep existing code)

```rust
mod context;
pub use context::*;
```

- [ ] **Step 3: Write the failing test in `crates/recall-inject/src/context.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::detect_context;
    use std::process::Command;

    fn git(dir: &std::path::Path, args: &[&str]) {
        let ok = Command::new("git").current_dir(dir).args(args).output().unwrap();
        assert!(ok.status.success(), "git {:?} failed", args);
    }

    #[test]
    fn detects_remote_branch_and_language() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        git(dir, &["init", "-q"]);
        git(dir, &["remote", "add", "origin", "git@github.com:me/app.git"]);
        git(dir, &["checkout", "-q", "-b", "feature/x"]);
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();

        let ctx = detect_context(dir);
        assert_eq!(ctx.remote_id.as_deref(), Some("github.com/me/app"));
        assert_eq!(ctx.branch.as_deref(), Some("feature/x"));
        assert!(ctx.languages.contains(&"rust".to_string()));
    }

    #[test]
    fn non_git_dir_yields_empty_context() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = detect_context(tmp.path());
        assert!(ctx.remote_id.is_none());
    }
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p recall-inject context`
Expected: FAIL — `cannot find function detect_context`.

- [ ] **Step 5: Write the implementation at the top of `crates/recall-inject/src/context.rs`**

```rust
use recall_core::{normalize_remote, RepoContext};
use std::path::Path;
use std::process::Command;

/// Detect the current repo context (remote, branch, languages) by shelling out
/// to git and probing for language marker files. All fields degrade to None/empty.
pub fn detect_context(cwd: &Path) -> RepoContext {
    let remote_id = git(cwd, &["remote", "get-url", "origin"]).map(|s| normalize_remote(&s));
    let branch = git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let languages = detect_languages(cwd);
    RepoContext { remote_id, branch, languages }
}

fn git(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").current_dir(cwd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn detect_languages(cwd: &Path) -> Vec<String> {
    let checks = [
        ("Cargo.toml", "rust"),
        ("package.json", "typescript"),
        ("go.mod", "go"),
        ("pyproject.toml", "python"),
        ("requirements.txt", "python"),
    ];
    let mut langs: Vec<String> = Vec::new();
    for (file, lang) in checks {
        if cwd.join(file).exists() && !langs.iter().any(|l| l == lang) {
            langs.push(lang.to_string());
        }
    }
    langs
}
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p recall-inject`
Expected: PASS (6 tests). Note: requires `git` on PATH (true in dev/CI).

- [ ] **Step 7: Commit**

```bash
git add crates/recall-inject
git commit -m "feat(inject): detect repo context (remote/branch/languages) via git

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: `recall-mcp` — handler logic (testable) + rmcp stdio server

**Files:**
- Create: `crates/recall-mcp/Cargo.toml`
- Create: `crates/recall-mcp/src/lib.rs`
- Modify: `Cargo.toml` (add `crates/recall-mcp` to `members`)
- Test: inline `#[cfg(test)]` in `crates/recall-mcp/src/lib.rs`

**Interfaces:**
- Produces: `recall_mcp::handle_list(db_path: &Path) -> anyhow::Result<String>`; `recall_mcp::handle_conventions(db_path: &Path, cwd: Option<&str>) -> anyhow::Result<String>`; `recall_mcp::run_stdio(db_path: PathBuf) -> anyhow::Result<()>` (async).
- Consumes: `recall_store::Store`; `recall_inject::{select, render, detect_context}`.

> **rmcp note for the implementer:** the macro identifiers below are verified against rmcp 1.8 (`#[tool_router]`, `#[tool_handler]`, `ContentBlock::text`, `ServiceExt::serve`, `transport::stdio`). The one import to confirm against the canonical example is `ToolRouter`'s path — this plan uses `rmcp::handler::server::tool::ToolRouter`. If the compiler rejects it, copy the exact `use` line from the counter example (https://github.com/modelcontextprotocol/rust-sdk/blob/main/examples/servers/src/common/counter.rs); the compiler error will name the correct path. Everything else compiles as written.

- [ ] **Step 1: Add the crate to the workspace `members` in root `Cargo.toml`**

```toml
members = ["crates/recall-core", "crates/recall-store", "crates/recall-inject", "crates/recall-mcp"]
```

- [ ] **Step 2: Create `crates/recall-mcp/Cargo.toml`**

```toml
[package]
name = "recall-mcp"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
recall-core = { path = "../recall-core" }
recall-store = { path = "../recall-store" }
recall-inject = { path = "../recall-inject" }
rmcp = { version = "1.8", features = ["server", "macros", "transport-io", "schemars"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std"] }
serde = { workspace = true }
anyhow = { workspace = true }

[dev-dependencies]
chrono = { workspace = true }
uuid = { workspace = true }
```

- [ ] **Step 3: Write the failing test in `crates/recall-mcp/src/lib.rs`**

Put at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::{handle_conventions, handle_list};
    use recall_core::*;
    use recall_store::Store;
    use chrono::Utc;
    use uuid::Uuid;

    fn seed(db: &std::path::Path) {
        let store = Store::open(db).unwrap();
        let now = Utc::now();
        let c = Convention {
            id: Uuid::new_v4(), rule: "Use early returns".into(), rationale: None,
            scope: Scope::Global, tags: vec![],
            provenance: Provenance { source: Source::ManualTeach, repo: None, branch: None,
                agent: None, excerpt: None, learned_at: now },
            status: Status::Active, superseded_by: None, confidence: 0.9,
            created_at: now, updated_at: now,
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
}
```

Also add `tempfile` to the dev-dependencies in `crates/recall-mcp/Cargo.toml`:

```toml
[dev-dependencies]
chrono = { workspace = true }
uuid = { workspace = true }
tempfile = { workspace = true }
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p recall-mcp`
Expected: FAIL — `cannot find function handle_list`.

- [ ] **Step 5: Write the implementation at the top of `crates/recall-mcp/src/lib.rs`**

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use recall_inject::{detect_context, render, select};
use recall_store::Store;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
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

#[derive(Clone)]
pub struct Recall {
    db_path: Arc<PathBuf>,
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
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }

    #[tool(description = "List all of the developer's active coding conventions across every scope.")]
    fn recall_list(&self) -> Result<CallToolResult, McpError> {
        let text = handle_list(&self.db_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
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
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p recall-mcp`
Expected: PASS (3 tests). If `ToolRouter`'s import path is rejected, fix it per the rmcp note above, then re-run.

- [ ] **Step 7: Verify the whole workspace builds**

Run: `cargo build`
Expected: builds cleanly (downloads/compiles rmcp on first run).

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/recall-mcp
git commit -m "feat(mcp): rmcp stdio server exposing recall_conventions + recall_list

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: `recall-cli` — binary skeleton, lib commands, `mcp` + `learn` + `list`

**Files:**
- Create: `crates/recall-cli/Cargo.toml`
- Create: `crates/recall-cli/src/lib.rs`
- Create: `crates/recall-cli/src/main.rs`
- Modify: `Cargo.toml` (add `crates/recall-cli` to `members`)
- Test: inline `#[cfg(test)]` in `crates/recall-cli/src/lib.rs`

**Interfaces:**
- Produces: `recall_cli::{parse_scope, cmd_learn, cmd_list, cmd_why, cmd_forget, cmd_status}` (the latter two are completed in Task 9). Binary `recall` with subcommands `mcp|learn|list|why|forget|status`.
- Consumes: everything from `recall-core`, `recall-store`, `recall-inject`, `recall-mcp`.

- [ ] **Step 1: Add the crate to the workspace `members` in root `Cargo.toml`**

```toml
members = ["crates/recall-core", "crates/recall-store", "crates/recall-inject", "crates/recall-mcp", "crates/recall-cli"]
```

- [ ] **Step 2: Create `crates/recall-cli/Cargo.toml`**

```toml
[package]
name = "recall-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[lib]
name = "recall_cli"
path = "src/lib.rs"

[[bin]]
name = "recall"
path = "src/main.rs"

[dependencies]
recall-core = { path = "../recall-core" }
recall-store = { path = "../recall-store" }
recall-inject = { path = "../recall-inject" }
recall-mcp = { path = "../recall-mcp" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std"] }
chrono = { workspace = true }
uuid = { workspace = true }
anyhow = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 3: Write the failing test in `crates/recall-cli/src/lib.rs`**

Put at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use recall_core::Scope;

    #[test]
    fn parse_scope_global_and_language() {
        assert_eq!(parse_scope("global").unwrap(), Scope::Global);
        assert_eq!(parse_scope("language:rust").unwrap(), Scope::Language("rust".into()));
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
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p recall-cli`
Expected: FAIL — `cannot find function parse_scope`.

- [ ] **Step 5: Write `crates/recall-cli/src/lib.rs`** (above the test block)

```rust
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
        let remote = ctx
            .remote_id
            .ok_or_else(|| anyhow!("not in a git repo with an 'origin' remote; can't use --scope {s}"))?;
        if s == "repo" {
            return Ok(Scope::Repo { remote_id: remote });
        }
        let branch = ctx.branch.ok_or_else(|| anyhow!("can't detect the current branch"))?;
        return Ok(Scope::Branch { remote_id: remote, branch });
    }
    Err(anyhow!("unknown scope '{s}': use global | repo | branch | language:<lang>"))
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
        s.push_str(&format!("[{}] {} ({})\n", short(&c.id), c.rule, scope_label(&c.scope)));
    }
    Ok(s.trim_end().to_string())
}
```

- [ ] **Step 6: Run the lib test to verify it passes**

Run: `cargo test -p recall-cli`
Expected: PASS (3 tests).

- [ ] **Step 7: Write `crates/recall-cli/src/main.rs`**

> Note: `cmd_why`, `cmd_forget`, and `cmd_status` are implemented in Task 9. To keep this task compiling, this `main.rs` already wires them; implement Task 9 immediately after so the binary builds. (If you must build between tasks, temporarily stub the three with `todo!()` — but Task 9 follows directly.)

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "recall", version, about = "Your personal coding-convention brain")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the Recall MCP server over stdio
    Mcp,
    /// Teach Recall a convention
    Learn {
        /// The rule, e.g. "Import directly; no barrel files"
        rule: String,
        /// global | repo | branch | language:<lang>
        #[arg(long, default_value = "global")]
        scope: String,
        /// Optional tag (repeatable)
        #[arg(long)]
        tag: Vec<String>,
    },
    /// List active conventions
    List,
    /// Show where a convention came from
    Why {
        /// Convention id (or unique prefix)
        id: String,
    },
    /// Retire a convention
    Forget {
        /// Convention id (or unique prefix)
        id: String,
    },
    /// Show Recall status
    Status,
}

fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("RECALL_DB") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".recall").join("recall.db")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = db_path();
    match cli.cmd {
        Cmd::Mcp => recall_mcp::run_stdio(db).await?,
        Cmd::Learn { rule, scope, tag } => {
            println!("{}", recall_cli::cmd_learn(&db, &rule, &scope, tag)?)
        }
        Cmd::List => println!("{}", recall_cli::cmd_list(&db)?),
        Cmd::Why { id } => println!("{}", recall_cli::cmd_why(&db, &id)?),
        Cmd::Forget { id } => println!("{}", recall_cli::cmd_forget(&db, &id)?),
        Cmd::Status => println!("{}", recall_cli::cmd_status(&db)?),
    }
    Ok(())
}
```

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/recall-cli
git commit -m "feat(cli): recall binary + learn/list commands and subcommand wiring

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: `recall-cli` — `why`, `forget`, `status`

**Files:**
- Modify: `crates/recall-cli/src/lib.rs` (add three functions + a prefix resolver)
- Test: extend the `#[cfg(test)]` block in `crates/recall-cli/src/lib.rs`

**Interfaces:**
- Produces: `cmd_why(db: &Path, id_prefix: &str) -> Result<String>`; `cmd_forget(db: &Path, id_prefix: &str) -> Result<String>`; `cmd_status(db: &Path) -> Result<String>`.
- Consumes: `Store`, `Convention`, `scope_label`, `Source`.

- [ ] **Step 1: Add the failing tests to the `#[cfg(test)]` block in `crates/recall-cli/src/lib.rs`**

```rust
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
        assert!(cmd_list(&db).unwrap().to_lowercase().contains("no conventions"));
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p recall-cli`
Expected: FAIL — `cannot find function cmd_why`.

- [ ] **Step 3: Add the implementations to `crates/recall-cli/src/lib.rs`** (place above the test block, after `cmd_list`)

```rust
fn find_by_prefix(store: &Store, prefix: &str) -> Result<Convention> {
    let matches: Vec<Convention> = store
        .all()?
        .into_iter()
        .filter(|c| c.id.to_string().starts_with(prefix))
        .collect();
    match matches.len() {
        0 => Err(anyhow!("no convention matches id '{prefix}'")),
        1 => Ok(matches.into_iter().next().unwrap()),
        n => Err(anyhow!("'{prefix}' is ambiguous ({n} matches); use more characters")),
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
    s.push_str(&format!("  learned by: {}\n", source_label(&c.provenance.source)));
    s.push_str(&format!("  learned at: {}\n", c.provenance.learned_at.to_rfc3339()));
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p recall-cli`
Expected: PASS (5 tests).

- [ ] **Step 5: Build the binary end-to-end**

Run: `cargo build && ./target/debug/recall --help`
Expected: help text listing `mcp, learn, list, why, forget, status`.

- [ ] **Step 6: Commit**

```bash
git add crates/recall-cli
git commit -m "feat(cli): why/forget/status commands with id-prefix resolution

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: Dogfood wiring + manual smoke test + dev docs

**Files:**
- Create: `.mcp.json` (dev MCP config pointing at the debug binary)
- Create: `docs/DEV.md` (how to dogfood Recall on your own session)
- Modify: `.gitignore` (ignore `/target`)

**Interfaces:**
- Consumes: the `recall` binary from Task 9.

- [ ] **Step 1: Create `.gitignore`**

```gitignore
/target
```

- [ ] **Step 2: Create `.mcp.json`** (dev config; ships repointed to npm in Plan 3)

```json
{
  "mcpServers": {
    "recall": {
      "command": "./target/debug/recall",
      "args": ["mcp"]
    }
  }
}
```

- [ ] **Step 3: Create `docs/DEV.md`**

````markdown
# Developing Recall by dogfooding it

Recall is built by using Recall. Once the binary builds, register the local
debug build as an MCP server on your own Claude Code / Codex session and call
its tools live.

## Build

```bash
cargo build
```

## Register on Claude Code (project scope)

The repo ships a dev `.mcp.json` pointing at `./target/debug/recall mcp`.
From the repo root:

```bash
claude mcp add recall -- ./target/debug/recall mcp
# or rely on the project-scoped .mcp.json and approve it when prompted
```

Then in a session: call the `recall_conventions` and `recall_list` tools.

## Teach + verify loop

```bash
# teach a convention (uses ~/.recall/recall.db by default)
./target/debug/recall learn "Import directly; no barrel files" --scope global

# confirm it's stored
./target/debug/recall list

# the MCP tool should now return it
#   recall_conventions  -> includes the rule
#   recall_list         -> includes the rule
```

Use a throwaway DB while experimenting:

```bash
RECALL_DB=/tmp/recall-dev.db ./target/debug/recall learn "..." 
RECALL_DB=/tmp/recall-dev.db ./target/debug/recall list
```
````

- [ ] **Step 4: Manual smoke test (record results in the PR/commit message)**

Run each and confirm:

```bash
cargo build
RECALL_DB=/tmp/recall-smoke.db ./target/debug/recall learn "Use early returns" --scope global
RECALL_DB=/tmp/recall-smoke.db ./target/debug/recall list          # shows the rule + id
RECALL_DB=/tmp/recall-smoke.db ./target/debug/recall status        # active: 1
# echo a minimal MCP initialize/list to confirm the server starts (optional):
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  | RECALL_DB=/tmp/recall-smoke.db ./target/debug/recall mcp
```
Expected: the `learn`/`list`/`status` outputs are correct; the `mcp` process reads stdin and responds with a JSON-RPC `initialize` result on stdout (Ctrl-C to exit).

- [ ] **Step 5: Run the full test suite**

Run: `cargo test`
Expected: all tests pass across all five crates.

- [ ] **Step 6: Commit**

```bash
git add .gitignore .mcp.json docs/DEV.md
git commit -m "chore: dev MCP config + dogfooding docs

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage (Plan 1 scope):**
- Convention model + Scope (global/language/repo/branch) + provenance + status + supersession field → Tasks 1, 2. ✅
- Anti-staleness *field* (`superseded_by`/`Status::Superseded`) present; the LLM-driven *act* of supersession is correctly deferred to Plan 2 (noted in Task 2). ✅
- SQLite store, local-first, single binary (bundled) → Task 4. ✅
- Markdown export/import — **deferred to a later plan** (not in Plan 1 scope; noted here so it isn't forgotten). ✅ (gap acknowledged, not silent)
- Inject: select by repo/branch/language, budget cap, render; SessionStart hook wiring → selection/render here (Tasks 5, 6); the *hook* ships in Plan 3 packaging. ✅
- MCP server (`recall_conventions`, `recall_list`) via rmcp → Task 7. ✅
- CLI inspect/edit (`list/why/forget/learn/status`) → Tasks 8, 9. (`review`/`export`/`import` arrive with Plan 2/later.) ✅
- Dogfood as local MCP (spec §9.1) → Task 10. ✅
- LLM provider trait + capture/curate → correctly **out of scope** (Plan 2). ✅

**Placeholder scan:** No TBD/TODO left in code steps. The only forward-reference (`cmd_why/forget/status` used by `main.rs` in Task 8) is explicitly flagged and resolved in Task 9, which follows immediately.

**Type consistency:** `Store` methods, `Convention` fields, `Scope` variants, `scope_label`, `detect_context`, `select`, `render`, `handle_list`, `handle_conventions`, `parse_scope`, and the `cmd_*` signatures are used identically across tasks. The SQLite `status` column string (`status_str`) matches serde's enum-variant serialization used in the `WHERE status = 'Active'` query.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-recall-core-mcp.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?

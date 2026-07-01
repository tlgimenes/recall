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
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM conventions WHERE id = ?1")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use recall_core::*;
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

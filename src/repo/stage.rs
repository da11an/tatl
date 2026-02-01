use rusqlite::Connection;
use crate::models::StageMapping;
use anyhow::{Context, Result};

pub struct StageRepo;

impl StageRepo {
    /// List all stage mappings ordered by id
    pub fn list_all(conn: &Connection) -> Result<Vec<StageMapping>> {
        let mut stmt = conn.prepare(
            "SELECT id, status, in_queue, has_sessions, has_open_session, has_externals,
                    stage, sort_order, color
             FROM stage_map ORDER BY id"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(StageMapping {
                id: row.get(0)?,
                status: row.get(1)?,
                in_queue: row.get(2)?,
                has_sessions: row.get(3)?,
                has_open_session: row.get(4)?,
                has_externals: row.get(5)?,
                stage: row.get(6)?,
                sort_order: row.get(7)?,
                color: row.get(8)?,
            })
        })?;

        let mut mappings = Vec::new();
        for row in rows {
            mappings.push(row?);
        }
        Ok(mappings)
    }

    /// Load all stage mappings for in-memory cache (same as list_all)
    pub fn load_map(conn: &Connection) -> Result<Vec<StageMapping>> {
        Self::list_all(conn)
    }

    /// Look up the stage for a given combination of state booleans.
    /// For terminal statuses (closed, cancelled), matches on status alone (wildcard rows).
    /// For open status, matches on exact boolean values.
    pub fn lookup(
        conn: &Connection,
        status: &str,
        in_queue: bool,
        has_sessions: bool,
        has_open_session: bool,
        has_externals: bool,
    ) -> Result<StageMapping> {
        // Terminal statuses use wildcard rows (in_queue = -1)
        if status == "closed" || status == "cancelled" {
            let mut stmt = conn.prepare(
                "SELECT id, status, in_queue, has_sessions, has_open_session, has_externals,
                        stage, sort_order, color
                 FROM stage_map
                 WHERE status = ?1 AND in_queue = -1
                 LIMIT 1"
            )?;
            let mapping = stmt.query_row([status], |row| {
                Ok(StageMapping {
                    id: row.get(0)?,
                    status: row.get(1)?,
                    in_queue: row.get(2)?,
                    has_sessions: row.get(3)?,
                    has_open_session: row.get(4)?,
                    has_externals: row.get(5)?,
                    stage: row.get(6)?,
                    sort_order: row.get(7)?,
                    color: row.get(8)?,
                })
            }).with_context(|| format!("No stage mapping found for terminal status '{}'", status))?;
            return Ok(mapping);
        }

        // Open status: exact match on all booleans
        let mut stmt = conn.prepare(
            "SELECT id, status, in_queue, has_sessions, has_open_session, has_externals,
                    stage, sort_order, color
             FROM stage_map
             WHERE status = ?1 AND in_queue = ?2 AND has_sessions = ?3
                   AND has_open_session = ?4 AND has_externals = ?5
             LIMIT 1"
        )?;
        let mapping = stmt.query_row(
            rusqlite::params![
                status,
                in_queue as i8,
                has_sessions as i8,
                has_open_session as i8,
                has_externals as i8,
            ],
            |row| {
                Ok(StageMapping {
                    id: row.get(0)?,
                    status: row.get(1)?,
                    in_queue: row.get(2)?,
                    has_sessions: row.get(3)?,
                    has_open_session: row.get(4)?,
                    has_externals: row.get(5)?,
                    stage: row.get(6)?,
                    sort_order: row.get(7)?,
                    color: row.get(8)?,
                })
            },
        ).with_context(|| format!(
            "No stage mapping found for status='{}' in_queue={} has_sessions={} has_open_session={} has_externals={}",
            status, in_queue, has_sessions, has_open_session, has_externals
        ))?;
        Ok(mapping)
    }

    /// Look up stage from a pre-loaded cache of mappings.
    /// Returns the stage name and sort_order.
    pub fn lookup_from_cache<'a>(
        mappings: &'a [StageMapping],
        status: &str,
        in_queue: bool,
        has_sessions: bool,
        has_open_session: bool,
        has_externals: bool,
    ) -> Option<&'a StageMapping> {
        if status == "closed" || status == "cancelled" {
            return mappings.iter().find(|m| m.status == status && m.in_queue == -1);
        }

        let iq = in_queue as i8;
        let hs = has_sessions as i8;
        let hos = has_open_session as i8;
        let he = has_externals as i8;

        mappings.iter().find(|m| {
            m.status == status
                && m.in_queue == iq
                && m.has_sessions == hs
                && m.has_open_session == hos
                && m.has_externals == he
        })
    }

    /// Update a stage mapping row
    pub fn update(conn: &Connection, id: i64, stage: Option<&str>, sort_order: Option<i64>, color: Option<Option<&str>>) -> Result<()> {
        // Build dynamic update
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(s) = stage {
            sets.push("stage = ?");
            params.push(Box::new(s.to_string()));
        }
        if let Some(so) = sort_order {
            sets.push("sort_order = ?");
            params.push(Box::new(so));
        }
        if let Some(c) = color {
            sets.push("color = ?");
            params.push(Box::new(c.map(|s| s.to_string())));
        }

        if sets.is_empty() {
            return Ok(());
        }

        // Number the parameters
        let mut numbered_sets = Vec::new();
        for (i, set) in sets.iter().enumerate() {
            numbered_sets.push(set.replace('?', &format!("?{}", i + 1)));
        }
        let id_param = params.len() + 1;
        let sql = format!(
            "UPDATE stage_map SET {} WHERE id = ?{}",
            numbered_sets.join(", "),
            id_param
        );
        params.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let updated = conn.execute(&sql, param_refs.as_slice())
            .with_context(|| format!("Failed to update stage mapping id={}", id))?;

        if updated == 0 {
            anyhow::bail!("No stage mapping found with id={}", id);
        }

        Ok(())
    }
}

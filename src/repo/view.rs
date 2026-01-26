use rusqlite::{Connection, OptionalExtension};
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ListView {
    pub name: String,
    pub entity: String,
    pub filter_tokens: Vec<String>,
    pub sort_columns: Vec<String>,
    pub group_columns: Vec<String>,
    pub hide_columns: Vec<String>,
    pub color_column: Option<String>,
    pub fill_column: Option<String>,
    pub created_ts: i64,
    pub modified_ts: i64,
}

pub struct ViewRepo;

impl ViewRepo {
    pub fn get_by_name(conn: &Connection, entity: &str, name: &str) -> Result<Option<ListView>> {
        let mut stmt = conn.prepare(
            "SELECT name, entity, filter_json, sort_json, group_json, COALESCE(hide_json, '[]'), 
                    COALESCE(color_json, 'null'), COALESCE(fill_json, 'null'), created_ts, modified_ts
             FROM list_views WHERE entity = ?1 AND name = ?2"
        )?;
        let view = stmt.query_row([entity, name], |row| {
            let filter_json: String = row.get(2)?;
            let sort_json: String = row.get(3)?;
            let group_json: String = row.get(4)?;
            let hide_json: String = row.get(5)?;
            let color_json: String = row.get(6)?;
            let fill_json: String = row.get(7)?;
            Ok(ListView {
                name: row.get(0)?,
                entity: row.get(1)?,
                filter_tokens: serde_json::from_str(&filter_json).unwrap_or_default(),
                sort_columns: serde_json::from_str(&sort_json).unwrap_or_default(),
                group_columns: serde_json::from_str(&group_json).unwrap_or_default(),
                hide_columns: serde_json::from_str(&hide_json).unwrap_or_default(),
                color_column: serde_json::from_str(&color_json).ok().flatten(),
                fill_column: serde_json::from_str(&fill_json).ok().flatten(),
                created_ts: row.get(8)?,
                modified_ts: row.get(9)?,
            })
        }).optional()?;
        Ok(view)
    }
    
    pub fn upsert(
        conn: &Connection,
        name: &str,
        entity: &str,
        filter_tokens: &[String],
        sort_columns: &[String],
        group_columns: &[String],
        hide_columns: &[String],
        color_column: &Option<String>,
        fill_column: &Option<String>,
    ) -> Result<ListView> {
        let now = chrono::Utc::now().timestamp();
        let existing = Self::get_by_name(conn, entity, name)?;
        let created_ts = existing.as_ref().map(|v| v.created_ts).unwrap_or(now);
        
        let filter_json = serde_json::to_string(filter_tokens)?;
        let sort_json = serde_json::to_string(sort_columns)?;
        let group_json = serde_json::to_string(group_columns)?;
        let hide_json = serde_json::to_string(hide_columns)?;
        let color_json = serde_json::to_string(color_column)?;
        let fill_json = serde_json::to_string(fill_column)?;
        
        conn.execute(
            "INSERT INTO list_views (name, entity, filter_json, sort_json, group_json, hide_json, color_json, fill_json, created_ts, modified_ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(name) DO UPDATE SET
               entity = excluded.entity,
               filter_json = excluded.filter_json,
               sort_json = excluded.sort_json,
               group_json = excluded.group_json,
               hide_json = excluded.hide_json,
               color_json = excluded.color_json,
               fill_json = excluded.fill_json,
               modified_ts = excluded.modified_ts",
            rusqlite::params![name, entity, filter_json, sort_json, group_json, hide_json, color_json, fill_json, created_ts, now],
        )
        .with_context(|| format!("Failed to save view '{}'", name))?;
        
        Ok(ListView {
            name: name.to_string(),
            entity: entity.to_string(),
            filter_tokens: filter_tokens.to_vec(),
            sort_columns: sort_columns.to_vec(),
            group_columns: group_columns.to_vec(),
            hide_columns: hide_columns.to_vec(),
            color_column: color_column.clone(),
            fill_column: fill_column.clone(),
            created_ts,
            modified_ts: now,
        })
    }
}

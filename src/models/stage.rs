/// Stage mapping model
/// Represents a row in the stage_map table that maps task state booleans to a stage label.
#[derive(Debug, Clone)]
pub struct StageMapping {
    pub id: i64,
    pub status: String,
    pub in_queue: i8,      // 0, 1, or -1 (wildcard for terminal statuses)
    pub has_sessions: i8,
    pub has_open_session: i8,
    pub has_externals: i8,
    pub stage: String,
    pub sort_order: i64,
    pub color: Option<String>,
}

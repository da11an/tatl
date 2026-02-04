use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task status (lifecycle state)
///
/// Plan 41 state model:
/// - Open: task is alive and actionable
/// - Closed: intent fulfilled (terminal)
/// - Cancelled: intent shifted / not fulfilled (terminal)
/// - Deleted: ledger correction (not a lifecycle state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    Closed,
    Cancelled,
    Deleted,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Open => "open",
            TaskStatus::Closed => "closed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(TaskStatus::Open),
            "closed" => Some(TaskStatus::Closed),
            "cancelled" => Some(TaskStatus::Cancelled),
            "deleted" => Some(TaskStatus::Deleted),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed | Self::Cancelled | Self::Deleted)
    }
}

/// Task model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Option<i64>,
    pub uuid: String,
    pub description: String,
    pub status: TaskStatus,
    pub project_id: Option<i64>,
    pub due_ts: Option<i64>,
    pub scheduled_ts: Option<i64>,
    pub wait_ts: Option<i64>,
    pub alloc_secs: Option<i64>,
    pub template: Option<String>,
    pub respawn: Option<String>,
    pub parent_id: Option<i64>,
    pub udas: HashMap<String, String>, // User-defined attributes (without "uda." prefix)
    pub created_ts: i64,
    pub modified_ts: i64,
    pub activity_ts: i64,
}

impl Task {
    /// Create a new task
    pub fn new(description: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: None,
            uuid: uuid::Uuid::new_v4().to_string(),
            description,
            status: TaskStatus::Open,
            project_id: None,
            due_ts: None,
            scheduled_ts: None,
            wait_ts: None,
            alloc_secs: None,
            template: None,
            respawn: None,
            parent_id: None,
            udas: HashMap::new(),
            created_ts: now,
            modified_ts: now,
            activity_ts: now,
        }
    }

    /// Check if task is waiting (wait_ts is in the future)
    pub fn is_waiting(&self) -> bool {
        if let Some(wait_ts) = self.wait_ts {
            let now = chrono::Utc::now().timestamp();
            wait_ts > now
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_conversion() {
        assert_eq!(TaskStatus::Open.as_str(), "open");
        assert_eq!(TaskStatus::from_str("open"), Some(TaskStatus::Open));
        assert_eq!(TaskStatus::Closed.as_str(), "closed");
        assert_eq!(TaskStatus::from_str("closed"), Some(TaskStatus::Closed));
        assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
        assert_eq!(TaskStatus::from_str("cancelled"), Some(TaskStatus::Cancelled));
        assert_eq!(TaskStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_task_status_terminal() {
        assert!(!TaskStatus::Open.is_terminal());
        assert!(TaskStatus::Closed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(TaskStatus::Deleted.is_terminal());
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task".to_string());
        assert_eq!(task.description, "Test task");
        assert_eq!(task.status, TaskStatus::Open);
        assert!(task.id.is_none());
        assert!(!task.uuid.is_empty());
    }

    #[test]
    fn test_task_is_waiting() {
        let mut task = Task::new("Test".to_string());

        // Not waiting if wait_ts is None
        assert!(!task.is_waiting());

        // Waiting if wait_ts is in the future
        let future = chrono::Utc::now().timestamp() + 3600;
        task.wait_ts = Some(future);
        assert!(task.is_waiting());

        // Not waiting if wait_ts is in the past
        let past = chrono::Utc::now().timestamp() - 3600;
        task.wait_ts = Some(past);
        assert!(!task.is_waiting());
    }
}

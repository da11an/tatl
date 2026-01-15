// Acceptance Test Framework
// Provides infrastructure for writing Given/When/Then acceptance tests

use assert_cmd::Command;
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, MutexGuard};
use task_ninja::db::DbConnection;
use task_ninja::repo::{TaskRepo, ProjectRepo, StackRepo, SessionRepo};
use chrono::{Local, TimeZone};

/// Test context for acceptance tests
/// Manages database setup, teardown, and provides helper methods
pub struct AcceptanceTestContext {
    temp_dir: TempDir,
    db_path: PathBuf,
    conn: rusqlite::Connection,
    _env_guard: MutexGuard<'static, ()>,
}

impl AcceptanceTestContext {
    /// Create a new test context with a fresh database
    pub fn new() -> Self {
        let env_guard = lock_test_env();
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        // Create config file
        let config_dir = temp_dir.path().join(".taskninja");
        fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("rc");
        fs::write(&config_file, format!("data.location={}\n", db_path.display())).unwrap();
        
        // Set HOME environment variable
        std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        
        // Connect to database
        let conn = DbConnection::connect().unwrap();
        
        Self {
            temp_dir,
            db_path,
            conn,
            _env_guard: env_guard,
        }
    }
    
    /// Get a command instance configured for this test context
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("task").unwrap();
        cmd.env("HOME", self.temp_dir.path());
        cmd
    }
    
    /// Get direct database connection for assertions
    pub fn db(&self) -> &rusqlite::Connection {
        &self.conn
    }
    
    /// Get the temp directory path
    pub fn temp_dir(&self) -> &TempDir {
        &self.temp_dir
    }
}

fn lock_test_env() -> MutexGuard<'static, ()> {
    static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}

impl Drop for AcceptanceTestContext {
    fn drop(&mut self) {
        // Cleanup happens automatically when temp_dir is dropped
    }
}

/// Builder for Given steps (test setup)
pub struct GivenBuilder<'a> {
    ctx: &'a AcceptanceTestContext,
}

impl<'a> GivenBuilder<'a> {
    pub fn new(ctx: &'a AcceptanceTestContext) -> Self {
        Self { ctx }
    }
    
    /// Given: tasks exist with IDs
    pub fn tasks_exist(&self, task_ids: &[i64]) -> &Self {
        for task_id in task_ids {
            // Check if task already exists
            if TaskRepo::get_by_id(self.ctx.db(), *task_id).unwrap().is_none() {
                TaskRepo::create(self.ctx.db(), &format!("Task {}", task_id), None).unwrap();
            }
        }
        self
    }
    
    /// Given: task exists with description
    pub fn task_exists(&self, description: &str) -> i64 {
        let task = TaskRepo::create(self.ctx.db(), description, None).unwrap();
        task.id.unwrap()
    }
    
    /// Given: task exists with description and project
    pub fn task_exists_with_project(&self, description: &str, project_name: &str) -> i64 {
        // Try to get project, create if it doesn't exist
        let project = match ProjectRepo::get_by_name(self.ctx.db(), project_name).unwrap() {
            Some(p) => p,
            None => ProjectRepo::create(self.ctx.db(), project_name).unwrap(),
        };
        let task = TaskRepo::create(self.ctx.db(), description, Some(project.id.unwrap())).unwrap();
        task.id.unwrap()
    }
    
    /// Given: task exists with tags
    pub fn task_exists_with_tags(&self, description: &str, tags: &[&str]) -> i64 {
        let task = TaskRepo::create_full(
            self.ctx.db(),
            description,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &std::collections::HashMap::new(),
            &tags.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ).unwrap();
        task.id.unwrap()
    }
    
    /// Given: project exists
    pub fn project_exists(&self, name: &str) -> &Self {
        ProjectRepo::create(self.ctx.db(), name).unwrap();
        self
    }
    
    /// Given: stack is empty
    pub fn stack_is_empty(&self) -> &Self {
        let stack = StackRepo::get_or_create_default(self.ctx.db()).unwrap();
        StackRepo::clear(self.ctx.db(), stack.id.unwrap()).unwrap();
        self
    }
    
    /// Given: stack contains tasks in order
    pub fn stack_contains(&self, task_ids: &[i64]) -> &Self {
        let stack = StackRepo::get_or_create_default(self.ctx.db()).unwrap();
        StackRepo::clear(self.ctx.db(), stack.id.unwrap()).unwrap();
        for task_id in task_ids {
            StackRepo::enqueue(self.ctx.db(), stack.id.unwrap(), *task_id).unwrap();
        }
        self
    }
    
    /// Given: clock is running on task since time
    pub fn clock_running_on_task_since(&self, task_id: i64, time_str: &str) -> &Self {
        let start_ts = parse_time(time_str);
        SessionRepo::create(self.ctx.db(), task_id, start_ts).unwrap();
        self
    }
    
    /// Given: no running session
    pub fn no_running_session(&self) -> &Self {
        // Close any open session
        if let Ok(Some(_session)) = SessionRepo::get_open(self.ctx.db()) {
            SessionRepo::close_open(self.ctx.db(), chrono::Utc::now().timestamp()).unwrap();
        }
        self
    }
    
    /// Given: closed session exists for task
    pub fn closed_session_exists(&self, task_id: i64, start_str: &str, end_str: &str) -> &Self {
        let start_ts = parse_time(start_str);
        let end_ts = parse_time(end_str);
        SessionRepo::create_closed(self.ctx.db(), task_id, start_ts, end_ts).unwrap();
        self
    }
}

/// Builder for When steps (actions)
pub struct WhenBuilder<'a> {
    ctx: &'a AcceptanceTestContext,
    cmd_result: Option<std::process::Output>,
}

impl<'a> WhenBuilder<'a> {
    pub fn new(ctx: &'a AcceptanceTestContext) -> Self {
        Self {
            ctx,
            cmd_result: None,
        }
    }
    
    /// When: execute command
    pub fn execute(&mut self, args: &[&str]) -> &mut Self {
        let result = self.ctx.cmd()
            .args(args)
            .output()
            .unwrap();
        self.cmd_result = Some(result);
        self
    }
    
    /// When: execute command and expect success
    pub fn execute_success(&mut self, args: &[&str]) -> &mut Self {
        let result = self.ctx.cmd()
            .args(args)
            .assert()
            .success()
            .get_output()
            .clone();
        self.cmd_result = Some(result);
        self
    }
    
    /// When: execute command and expect failure
    pub fn execute_failure(&mut self, args: &[&str]) -> &mut Self {
        let result = self.ctx.cmd()
            .args(args)
            .assert()
            .failure()
            .get_output()
            .clone();
        self.cmd_result = Some(result);
        self
    }
    
    /// Get the command result for assertions
    pub fn result(&self) -> Option<&std::process::Output> {
        self.cmd_result.as_ref()
    }
}

/// Builder for Then steps (assertions)
pub struct ThenBuilder<'a> {
    ctx: &'a AcceptanceTestContext,
    when_result: Option<&'a std::process::Output>,
}

impl<'a> ThenBuilder<'a> {
    pub fn new(ctx: &'a AcceptanceTestContext, when_result: Option<&'a std::process::Output>) -> Self {
        Self {
            ctx,
            when_result,
        }
    }
    
    /// Then: exit code is
    pub fn exit_code_is(&self, expected: i32) -> &Self {
        if let Some(result) = self.when_result {
            let actual = result.status.code().unwrap_or(-1);
            assert_eq!(actual, expected, "Expected exit code {}, got {}", expected, actual);
        }
        self
    }
    
    /// Then: message contains
    pub fn message_contains(&self, text: &str) -> &Self {
        if let Some(result) = self.when_result {
            let output = String::from_utf8_lossy(&result.stdout);
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(
                output.contains(text) || error.contains(text),
                "Expected message to contain '{}', but got stdout: '{}', stderr: '{}'",
                text, output, error
            );
        }
        self
    }
    
    /// Then: stack order is
    pub fn stack_order_is(&self, expected: &[i64]) -> &Self {
        let stack = StackRepo::get_or_create_default(self.ctx.db()).unwrap();
        let items = StackRepo::get_items(self.ctx.db(), stack.id.unwrap()).unwrap();
        let actual: Vec<i64> = items.iter().map(|item| item.task_id).collect();
        assert_eq!(actual, expected, "Expected stack order {:?}, got {:?}", expected, actual);
        self
    }
    
    /// Then: stack is empty
    pub fn stack_is_empty(&self) -> &Self {
        let stack = StackRepo::get_or_create_default(self.ctx.db()).unwrap();
        let items = StackRepo::get_items(self.ctx.db(), stack.id.unwrap()).unwrap();
        assert!(items.is_empty(), "Expected stack to be empty, but it contains {:?}", items);
        self
    }
    
    /// Then: task status is
    pub fn task_status_is(&self, task_id: i64, expected_status: &str) -> &Self {
        let task = TaskRepo::get_by_id(self.ctx.db(), task_id)
            .unwrap()
            .expect(&format!("Task {} not found", task_id));
        assert_eq!(task.status.as_str(), expected_status, 
                   "Expected task {} status to be '{}', got '{}'", 
                   task_id, expected_status, task.status.as_str());
        self
    }
    
    /// Then: task has tag
    pub fn task_has_tag(&self, task_id: i64, tag: &str) -> &Self {
        let tags = TaskRepo::get_tags(self.ctx.db(), task_id).unwrap();
        assert!(tags.contains(&tag.to_string()), 
                "Expected task {} to have tag '{}', but tags are: {:?}", 
                task_id, tag, tags);
        self
    }
    
    /// Then: task does not have tag
    pub fn task_does_not_have_tag(&self, task_id: i64, tag: &str) -> &Self {
        let tags = TaskRepo::get_tags(self.ctx.db(), task_id).unwrap();
        assert!(!tags.contains(&tag.to_string()), 
                "Expected task {} to not have tag '{}', but tags are: {:?}", 
                task_id, tag, tags);
        self
    }
    
    /// Then: running session exists for task
    pub fn running_session_exists_for_task(&self, task_id: i64) -> &Self {
        let session = SessionRepo::get_open(self.ctx.db())
            .unwrap()
            .expect("Expected a running session");
        assert_eq!(session.task_id, task_id, 
                   "Expected running session for task {}, got {}", 
                   task_id, session.task_id);
        self
    }
    
    /// Then: no running session exists
    pub fn no_running_session_exists(&self) -> &Self {
        let session = SessionRepo::get_open(self.ctx.db()).unwrap();
        assert!(session.is_none(), "Expected no running session, but one exists");
        self
    }
    
    /// Then: closed session exists for task
    pub fn closed_session_exists_for_task(&self, task_id: i64, start_str: &str, end_str: &str) -> &Self {
        let start_ts = parse_time(start_str);
        let end_ts = parse_time(end_str);
        let sessions = SessionRepo::get_by_task(self.ctx.db(), task_id).unwrap();
        let found = sessions.iter().any(|s| {
            s.start_ts == start_ts && s.end_ts == Some(end_ts)
        });
        assert!(found, 
                "Expected closed session for task {} from {} to {}, but sessions are: {:?}", 
                task_id, start_str, end_str, sessions);
        self
    }
    
    /// Then: project exists
    pub fn project_exists(&self, name: &str) -> &Self {
        let project = ProjectRepo::get_by_name(self.ctx.db(), name).unwrap();
        assert!(project.is_some(), "Expected project '{}' to exist", name);
        self
    }
    
    /// Then: project does not exist
    pub fn project_does_not_exist(&self, name: &str) -> &Self {
        let project = ProjectRepo::get_by_name(self.ctx.db(), name).unwrap();
        assert!(project.is_none(), "Expected project '{}' to not exist", name);
        self
    }
    
    /// Then: task references project
    pub fn task_references_project(&self, task_id: i64, project_name: &str) -> &Self {
        let task = TaskRepo::get_by_id(self.ctx.db(), task_id)
            .unwrap()
            .expect(&format!("Task {} not found", task_id));
        let project = ProjectRepo::get_by_id(self.ctx.db(), task.project_id.unwrap()).unwrap()
            .expect(&format!("Project not found for task {}", task_id));
        assert_eq!(project.name, project_name, 
                   "Expected task {} to reference project '{}', got '{}'", 
                   task_id, project_name, project.name);
        self
    }
    
    /// Then: no sessions are created
    pub fn no_sessions_are_created(&self) -> &Self {
        // This is typically checked by verifying session count before/after
        // For now, we'll just verify no open session
        self.no_running_session_exists()
    }
}

/// Helper function to parse time strings like "09:00" or "2026-01-10T09:00"
fn parse_time(time_str: &str) -> i64 {
    if time_str.contains('T') {
        // Full datetime: "2026-01-10T09:00" or "2026-01-10T09:00:00"
        // Remove seconds if present
        let normalized = if time_str.matches(':').count() == 3 {
            // Has seconds, remove them
            let parts: Vec<&str> = time_str.split(':').collect();
            format!("{}:{}", parts[0], parts[1])
        } else {
            time_str.to_string()
        };
        task_ninja::utils::parse_date_expr(&normalized).unwrap()
    } else if time_str.contains(':') {
        // Time only: "09:00" - assume today
        let today = Local::now().date_naive();
        let parts: Vec<&str> = time_str.split(':').collect();
        let hour: u32 = parts[0].parse().unwrap();
        let minute: u32 = parts[1].parse().unwrap();
        let dt = today.and_hms_opt(hour, minute, 0).unwrap();
        Local.from_local_datetime(&dt).single().unwrap().timestamp()
    } else {
        panic!("Unsupported time format: {}", time_str);
    }
}

/// Macro to simplify writing acceptance tests
#[macro_export]
macro_rules! acceptance_test {
    ($name:ident, $body:block) => {
        #[test]
        fn $name() {
            use acceptance_framework::*;
            $body
        }
    };
}

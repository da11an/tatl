//! Respawn module for task respawning on completion
//!
//! Unlike traditional recurrence (which pre-generates many instances),
//! respawn creates a new task instance only when the current one is completed.

pub mod parser;
pub mod generator;

pub use parser::*;
pub use generator::*;

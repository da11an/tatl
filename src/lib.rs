//! Tatl (Task and Time Ledger) - A powerful command-line task and time tracking tool
//!
//! This library provides the core functionality for Tatl, including:
//! - Database operations and migrations
//! - Data models for tasks, projects, sessions, and more
//! - Repository layer for data access
//! - CLI command parsing and execution
//! - Filter expression parsing and evaluation
//! - Respawn rule parsing and task respawning on completion
//! - Date/time and duration utilities
//!
//! # Example
//!
//! ```no_run
//! use tatl::cli::run;
//!
//! fn main() {
//!     if let Err(e) = run() {
//!         eprintln!("Error: {}", e);
//!         std::process::exit(1);
//!     }
//! }
//! ```

pub mod db;
pub mod models;
pub mod repo;
pub mod cli;
pub mod utils;
pub mod filter;
pub mod respawn;
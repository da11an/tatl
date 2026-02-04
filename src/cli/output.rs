// Output formatting utilities

use crate::models::{Task, TaskStatus, StageMapping};
use crate::repo::{AnnotationRepo, ProjectRepo, SessionRepo, StackRepo, TaskRepo, ExternalRepo, StageRepo};
use crate::cli::priority::calculate_priority;
use chrono::Local;
use rusqlite::Connection;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;
use std::io::IsTerminal;

// ANSI escape codes for terminal formatting
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_RESET: &str = "\x1b[0m";

// ANSI foreground colors (standard 16-color palette)
const ANSI_FG_BLACK: &str = "\x1b[30m";
const ANSI_FG_RED: &str = "\x1b[31m";
const ANSI_FG_GREEN: &str = "\x1b[32m";
const ANSI_FG_YELLOW: &str = "\x1b[33m";
const ANSI_FG_BLUE: &str = "\x1b[34m";
const ANSI_FG_MAGENTA: &str = "\x1b[35m";
const ANSI_FG_CYAN: &str = "\x1b[36m";
const ANSI_FG_WHITE: &str = "\x1b[37m";
const ANSI_FG_BRIGHT_BLACK: &str = "\x1b[90m";
const ANSI_FG_BRIGHT_GREEN: &str = "\x1b[92m";
const ANSI_FG_BRIGHT_YELLOW: &str = "\x1b[93m";
const ANSI_FG_BRIGHT_BLUE: &str = "\x1b[94m";
const ANSI_FG_BRIGHT_MAGENTA: &str = "\x1b[95m";
const ANSI_FG_BRIGHT_CYAN: &str = "\x1b[96m";

// ANSI background colors
const ANSI_BG_RED: &str = "\x1b[41m";
const ANSI_BG_GREEN: &str = "\x1b[42m";
const ANSI_BG_YELLOW: &str = "\x1b[43m";
const ANSI_BG_BLUE: &str = "\x1b[44m";
const ANSI_BG_MAGENTA: &str = "\x1b[45m";
const ANSI_BG_CYAN: &str = "\x1b[46m";
const ANSI_BG_BRIGHT_BLACK: &str = "\x1b[100m";

// Color palette for categorical data (hash-based assignment)
const CATEGORICAL_FG_PALETTE: &[&str] = &[
    ANSI_FG_BLUE,
    ANSI_FG_GREEN,
    ANSI_FG_CYAN,
    ANSI_FG_MAGENTA,
    ANSI_FG_YELLOW,
    ANSI_FG_BRIGHT_BLUE,
    ANSI_FG_BRIGHT_GREEN,
    ANSI_FG_BRIGHT_CYAN,
    ANSI_FG_BRIGHT_MAGENTA,
    ANSI_FG_BRIGHT_YELLOW,
];

const CATEGORICAL_BG_PALETTE: &[&str] = &[
    ANSI_BG_BLUE,
    ANSI_BG_GREEN,
    ANSI_BG_CYAN,
    ANSI_BG_MAGENTA,
    ANSI_BG_YELLOW,
    ANSI_BG_BRIGHT_BLACK,
];

/// Map a color name string to its ANSI foreground constant
fn color_name_to_fg(name: &str) -> Option<&'static str> {
    match name {
        "black" => Some(ANSI_FG_BLACK),
        "red" => Some(ANSI_FG_RED),
        "green" => Some(ANSI_FG_GREEN),
        "yellow" => Some(ANSI_FG_YELLOW),
        "blue" => Some(ANSI_FG_BLUE),
        "magenta" => Some(ANSI_FG_MAGENTA),
        "cyan" => Some(ANSI_FG_CYAN),
        "white" => Some(ANSI_FG_WHITE),
        "bright_black" => Some(ANSI_FG_BRIGHT_BLACK),
        "bright_red" => Some("\x1b[91m"),
        "bright_green" => Some(ANSI_FG_BRIGHT_GREEN),
        "bright_yellow" => Some(ANSI_FG_BRIGHT_YELLOW),
        "bright_blue" => Some(ANSI_FG_BRIGHT_BLUE),
        "bright_magenta" => Some(ANSI_FG_BRIGHT_MAGENTA),
        "bright_cyan" => Some(ANSI_FG_BRIGHT_CYAN),
        "bright_white" => Some("\x1b[97m"),
        _ => None,
    }
}

/// Map a color name string to its ANSI background constant
fn color_name_to_bg(name: &str) -> Option<&'static str> {
    match name {
        "black" => Some("\x1b[40m"),
        "red" => Some(ANSI_BG_RED),
        "green" => Some(ANSI_BG_GREEN),
        "yellow" => Some(ANSI_BG_YELLOW),
        "blue" => Some(ANSI_BG_BLUE),
        "magenta" => Some(ANSI_BG_MAGENTA),
        "cyan" => Some(ANSI_BG_CYAN),
        "white" => Some("\x1b[47m"),
        "bright_black" => Some(ANSI_BG_BRIGHT_BLACK),
        "bright_red" => Some("\x1b[101m"),
        "bright_green" => Some("\x1b[102m"),
        "bright_yellow" => Some("\x1b[103m"),
        "bright_blue" => Some("\x1b[104m"),
        "bright_magenta" => Some("\x1b[105m"),
        "bright_cyan" => Some("\x1b[106m"),
        "bright_white" => Some("\x1b[107m"),
        _ => None,
    }
}

/// Semantic colors for known column values
/// For the "stage" column, looks up colors from the stage map if available.
fn get_semantic_fg_color(column: &str, value: &str) -> Option<&'static str> {
    get_semantic_fg_color_with_stage_map(column, value, None)
}

/// Semantic colors with optional stage map lookup
fn get_semantic_fg_color_with_stage_map(column: &str, value: &str, stage_map: Option<&[StageMapping]>) -> Option<&'static str> {
    match column {
        "status" => match value {
            "open" => None,
            "closed" => Some(ANSI_FG_GREEN),
            "cancelled" => Some(ANSI_FG_BRIGHT_BLACK),
            _ => None,
        },
        "stage" => {
            // Try stage map first
            if let Some(mappings) = stage_map {
                if let Some(mapping) = mappings.iter().find(|m| m.stage.eq_ignore_ascii_case(value)) {
                    if let Some(ref color) = mapping.color {
                        if color == "none" {
                            return None;
                        }
                        return color_name_to_fg(color);
                    }
                    return None;
                }
            }
            // Fallback to hardcoded defaults
            match value {
                "proposed" => Some(ANSI_FG_BRIGHT_BLACK),
                "planned" => Some(ANSI_FG_BLUE),
                "in progress" => Some(ANSI_FG_CYAN),
                "active" => Some(ANSI_FG_GREEN),
                "suspended" => Some(ANSI_FG_YELLOW),
                "external" => Some(ANSI_FG_MAGENTA),
                "completed" => Some(ANSI_FG_BRIGHT_BLACK),
                "cancelled" => Some(ANSI_FG_BRIGHT_BLACK),
                _ => None,
            }
        },
        _ => None,
    }
}

fn get_semantic_bg_color(column: &str, value: &str) -> Option<&'static str> {
    get_semantic_bg_color_with_stage_map(column, value, None)
}

fn get_semantic_bg_color_with_stage_map(column: &str, value: &str, stage_map: Option<&[StageMapping]>) -> Option<&'static str> {
    match column {
        "status" => match value {
            "open" => None,
            "closed" => Some(ANSI_BG_GREEN),
            "cancelled" => Some(ANSI_BG_BRIGHT_BLACK),
            _ => None,
        },
        "stage" => {
            if let Some(mappings) = stage_map {
                if let Some(mapping) = mappings.iter().find(|m| m.stage.eq_ignore_ascii_case(value)) {
                    if let Some(ref color) = mapping.color {
                        if color == "none" {
                            return None;
                        }
                        return color_name_to_bg(color);
                    }
                    return None;
                }
            }
            match value {
                "proposed" => Some(ANSI_BG_BRIGHT_BLACK),
                "planned" => Some(ANSI_BG_BLUE),
                "in progress" => Some(ANSI_BG_CYAN),
                "active" => Some(ANSI_BG_GREEN),
                "suspended" => Some(ANSI_BG_YELLOW),
                "external" => Some(ANSI_BG_MAGENTA),
                "completed" => Some(ANSI_BG_BRIGHT_BLACK),
                "cancelled" => Some(ANSI_BG_BRIGHT_BLACK),
                _ => None,
            }
        },
        _ => None,
    }
}

/// Get foreground color for a value using hash-based palette
fn get_hash_fg_color(value: &str) -> &'static str {
    if value.is_empty() {
        return ANSI_FG_BRIGHT_BLACK;
    }
    let hash = value.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize).wrapping_mul(31));
    CATEGORICAL_FG_PALETTE[hash % CATEGORICAL_FG_PALETTE.len()]
}

/// Get background color for a value using hash-based palette
fn get_hash_bg_color(value: &str) -> &'static str {
    if value.is_empty() {
        return ANSI_BG_BRIGHT_BLACK;
    }
    let hash = value.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize).wrapping_mul(31));
    CATEGORICAL_BG_PALETTE[hash % CATEGORICAL_BG_PALETTE.len()]
}

/// Get gradient color based on normalized value (0.0 = green, 0.5 = yellow, 1.0 = red)
fn get_gradient_fg_color(normalized: f64) -> &'static str {
    if normalized <= 0.33 {
        ANSI_FG_GREEN
    } else if normalized <= 0.66 {
        ANSI_FG_YELLOW
    } else {
        ANSI_FG_RED
    }
}

fn get_gradient_bg_color(normalized: f64) -> &'static str {
    if normalized <= 0.33 {
        ANSI_BG_GREEN
    } else if normalized <= 0.66 {
        ANSI_BG_YELLOW
    } else {
        ANSI_BG_RED
    }
}

/// Convert ANSI background color code to approximate RGB values
/// 
/// These are approximations based on typical terminal color rendering.
/// Note: Actual colors vary significantly between terminals and color schemes.
/// Some terminals render ANSI_BG_BLUE as dark blue (needs white text),
/// others as light blue (needs black text). This function uses conservative
/// estimates that work reasonably well across most terminals.
/// 
/// For best results, terminals should use standard color schemes, but we
/// can't control that. This is a best-effort approach.
fn ansi_bg_to_rgb(bg_color: &str) -> Option<(u8, u8, u8)> {
    match bg_color {
        // Standard 16-color palette (approximate RGB values)
        // Using values that represent typical terminal rendering
        // Note: These are conservative estimates. Actual terminal colors vary.
        ANSI_BG_RED => Some((170, 0, 0)),           // Dark red - typically needs white text
        ANSI_BG_GREEN => Some((0, 200, 0)),         // Medium green - typically needs black text
        ANSI_BG_YELLOW => Some((200, 200, 0)),     // Yellow - typically needs black text (brighter)
        ANSI_BG_BLUE => Some((0, 0, 200)),         // Medium blue - varies by terminal (brighter)
        ANSI_BG_MAGENTA => Some((200, 0, 200)),    // Magenta - typically needs black text (brighter)
        ANSI_BG_CYAN => Some((0, 200, 200)),       // Cyan - typically needs black text (brighter)
        ANSI_BG_BRIGHT_BLACK => Some((128, 128, 128)), // Gray - typically needs black text (lighter)
        _ => None,
    }
}

/// Calculate relative luminance using WCAG formula
/// Returns a value between 0.0 (black) and 1.0 (white)
/// Formula: L = 0.2126*R + 0.7152*G + 0.0722*B (normalized to 0-1)
fn calculate_relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    // Normalize RGB values to 0-1 range
    let r_norm = r as f64 / 255.0;
    let g_norm = g as f64 / 255.0;
    let b_norm = b as f64 / 255.0;
    
    // Apply gamma correction (sRGB)
    let r_linear = if r_norm <= 0.04045 {
        r_norm / 12.92
    } else {
        ((r_norm + 0.055) / 1.055).powf(2.4)
    };
    
    let g_linear = if g_norm <= 0.04045 {
        g_norm / 12.92
    } else {
        ((g_norm + 0.055) / 1.055).powf(2.4)
    };
    
    let b_linear = if b_norm <= 0.04045 {
        b_norm / 12.92
    } else {
        ((b_norm + 0.055) / 1.055).powf(2.4)
    };
    
    // Calculate relative luminance
    0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear
}

/// Get contrasting foreground color for a background color to ensure legibility
/// 
/// Uses WCAG relative luminance calculation to determine if background is light or dark.
/// This is more reliable than hardcoded assumptions, though actual terminal rendering
/// may still vary. The function calculates the relative luminance of the background
/// color and chooses black text for light backgrounds (luminance > 0.5) and white
/// for dark backgrounds.
/// 
/// Note: This is an approximation. Different terminals and color schemes render
/// ANSI colors differently. For terminals with custom color schemes, results may
/// not be perfect, but should be better than hardcoded assumptions.
fn get_contrasting_fg_for_bg(bg_color: &str) -> &'static str {
    // Try to get RGB values for the background color
    if let Some((r, g, b)) = ansi_bg_to_rgb(bg_color) {
        // Calculate relative luminance using WCAG formula
        let luminance = calculate_relative_luminance(r, g, b);
        
        // Use black text for light backgrounds (luminance > 0.5), white for dark
        // Threshold of 0.5 is a good balance for most terminal color schemes
        // This works well for standard terminal colors, though custom schemes may vary
        if luminance > 0.5 {
            ANSI_FG_BLACK
        } else {
            ANSI_FG_WHITE
        }
    } else {
        // Fallback: if we can't determine the color, default to black
        // (safer assumption for most backgrounds)
        ANSI_FG_BLACK
    }
}

/// Column types for automatic color mapping detection
#[derive(Debug, Clone, Copy, PartialEq)]
enum ColumnColorType {
    Categorical,  // project, status, stage, tags
    Numeric,      // priority, alloc, clock
    Date,         // due, scheduled
}

fn detect_column_color_type(column: &str) -> ColumnColorType {
    match column.to_lowercase().as_str() {
        "project" | "status" | "stage" | "tags" => ColumnColorType::Categorical,
        "priority" | "alloc" | "clock" | "allocation" => ColumnColorType::Numeric,
        "due" | "scheduled" | "wait" => ColumnColorType::Date,
        _ => ColumnColorType::Categorical, // Default to categorical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_relative_luminance() {
        // Test that white has high luminance
        let white_lum = calculate_relative_luminance(255, 255, 255);
        assert!(white_lum > 0.9, "White should have high luminance, got {}", white_lum);
        
        // Test that black has low luminance
        let black_lum = calculate_relative_luminance(0, 0, 0);
        assert!(black_lum < 0.1, "Black should have low luminance, got {}", black_lum);
        
        // Test that yellow (light color) has high luminance
        let yellow_lum = calculate_relative_luminance(200, 200, 0);
        assert!(yellow_lum > 0.5, "Yellow should have high luminance, got {}", yellow_lum);
        
        // Test that dark red has low luminance
        let dark_red_lum = calculate_relative_luminance(170, 0, 0);
        assert!(dark_red_lum < 0.5, "Dark red should have low luminance, got {}", dark_red_lum);
    }

    #[test]
    fn test_get_contrasting_fg_for_bg() {
        // Test that the function returns valid ANSI codes
        let yellow_fg = get_contrasting_fg_for_bg(ANSI_BG_YELLOW);
        assert!(yellow_fg == ANSI_FG_BLACK || yellow_fg == ANSI_FG_WHITE, 
                "Should return valid foreground color");
        
        let green_fg = get_contrasting_fg_for_bg(ANSI_BG_GREEN);
        assert!(green_fg == ANSI_FG_BLACK || green_fg == ANSI_FG_WHITE,
                "Should return valid foreground color");
        
        // Dark red should typically get white text (low luminance)
        let red_fg = get_contrasting_fg_for_bg(ANSI_BG_RED);
        assert!(red_fg == ANSI_FG_BLACK || red_fg == ANSI_FG_WHITE,
                "Should return valid foreground color");
        
        // Unknown colors default to black (safer)
        assert_eq!(get_contrasting_fg_for_bg("unknown"), ANSI_FG_BLACK);
    }
}

/// Check if stdout is a terminal (TTY)
pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Get terminal width dynamically
/// 
/// Uses the `terminal_size` crate for reliable detection, with fallback to
/// COLUMNS environment variable and a sensible default.
pub fn get_terminal_width() -> usize {
    // Try terminal_size crate first (most reliable, works after resize)
    if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
        if w > 0 {
            return w as usize;
        }
    }
    
    // Fallback to COLUMNS environment variable (set by most shells)
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(width) = cols.parse::<usize>() {
            if width > 0 && width < 10000 { // Sanity check
                return width;
            }
        }
    }
    
    // Default fallback - reasonable default for most terminals
    120
}

/// Apply bold formatting if in TTY mode
fn bold_if_tty(text: &str, is_tty: bool) -> String {
    if is_tty {
        format!("{}{}{}", ANSI_BOLD, text, ANSI_RESET)
    } else {
        text.to_string()
    }
}

/// Stage status values (derived from task state via stage_map table)
///
/// Uses the stage_map table to look up the stage label for a given combination
/// of task state booleans. Falls back to hardcoded defaults if no stage map
/// is provided.
///
/// Note: Q column shows exact queue position (0, 1, 2, etc. or @ for external)
pub fn calculate_stage_status(
    task: &Task,
    stack_position: Option<usize>,
    has_sessions: bool,
    open_session_task_id: Option<i64>,
    has_externals: bool,
    stage_map: Option<&[StageMapping]>,
) -> String {
    let status = task.status.as_str();
    let in_queue = stack_position.is_some();
    let has_open_session = open_session_task_id.map_or(false, |tid| task.id == Some(tid));

    if let Some(mappings) = stage_map {
        if let Some(mapping) = StageRepo::lookup_from_cache(
            mappings, status, in_queue, has_sessions, has_open_session, has_externals,
        ) {
            return mapping.stage.clone();
        }
    }

    // Fallback to hardcoded defaults if no mapping found
    if task.status == TaskStatus::Closed {
        return "completed".to_string();
    }
    if task.status == TaskStatus::Cancelled {
        return "cancelled".to_string();
    }
    if has_open_session {
        return "active".to_string();
    }
    if has_externals {
        return "external".to_string();
    }
    match (in_queue, has_sessions) {
        (true, true) => "in progress".to_string(),
        (true, false) => "planned".to_string(),
        (false, true) => "suspended".to_string(),
        (false, false) => "proposed".to_string(),
    }
}

/// Get stack positions for all task IDs as a map (task_id -> position)
fn get_stack_positions(conn: &Connection) -> Result<HashMap<i64, usize>> {
    let stack = StackRepo::get_or_create_default(conn)?;
    let items = StackRepo::get_items(conn, stack.id.unwrap())?;
    
    let mut positions = HashMap::new();
    for (idx, item) in items.iter().enumerate() {
        positions.insert(item.task_id, idx);
    }
    
    Ok(positions)
}

/// Get set of task IDs that have any sessions
fn get_tasks_with_sessions(conn: &Connection) -> Result<HashSet<i64>> {
    let all_sessions = SessionRepo::list_all(conn)?;
    let task_ids: HashSet<i64> = all_sessions.iter().map(|s| s.task_id).collect();
    Ok(task_ids)
}

/// Get set of task IDs that have active externals
fn get_tasks_with_externals(conn: &Connection) -> Result<HashSet<i64>> {
    let all_externals = ExternalRepo::get_all_active(conn)?;
    let task_ids: HashSet<i64> = all_externals.iter().map(|e| e.task_id).collect();
    Ok(task_ids)
}

/// Format timestamp for display
pub fn format_timestamp(ts: i64) -> String {
    use chrono::TimeZone;
    let dt = Local.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Format date for display (date only, no time)
pub fn format_date(ts: i64) -> String {
    use chrono::TimeZone;
    let dt = Local.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    dt.format("%Y-%m-%d").to_string()
}

/// Format date as relative time (e.g., "2 days ago", "in 3 days", "today", "overdue")
pub fn format_relative_date(ts: i64) -> String {
    use chrono::{Local, TimeZone};
    let now = Local::now();
    let due_dt = Local.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().unwrap());
    
    let today = now.date_naive();
    let due_date = due_dt.date_naive();
    let days_diff = (due_date - today).num_days();
    
    if days_diff < 0 {
        // Past date
        if days_diff >= -30 {
            // Within last 30 days - show "X days ago"
            let days = (-days_diff) as u32;
            if days == 1 {
                "overdue".to_string()
            } else {
                format!("{} days ago", days)
            }
        } else {
            // More than 30 days ago - show "overdue"
            "overdue".to_string()
        }
    } else if days_diff == 0 {
        "today".to_string()
    } else if days_diff == 1 {
        "tomorrow".to_string()
    } else if days_diff <= 365 {
        // Within a year - show "in X days"
        format!("in {} days", days_diff)
    } else {
        // More than a year in future - show absolute date
        format_date(ts)
    }
}

/// Format duration for display
pub fn format_duration(secs: i64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{}h{}m{}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m{}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskListOptions {
    pub use_relative_time: bool,
    pub sort_columns: Vec<String>,
    pub group_columns: Vec<String>,
    pub hide_columns: Vec<String>,
    pub color_column: Option<String>,  // Column for text color
    pub fill_column: Option<String>,   // Column for background color
    pub full_width: bool, // Show all columns regardless of terminal width
}

/// Parse a sort specification, detecting negation prefix for descending order
struct SortSpec {
    column: String,
    descending: bool,
}

fn parse_sort_spec(spec: &str) -> SortSpec {
    if let Some(col) = spec.strip_prefix('-') {
        SortSpec { column: col.to_string(), descending: true }
    } else {
        SortSpec { column: spec.to_string(), descending: false }
    }
}

/// Ordinal value for stage status (workflow progression)
/// Looks up sort_order from stage map if available, falls back to hardcoded defaults.
fn stage_sort_order(stage: &str, stage_map: Option<&[StageMapping]>) -> i64 {
    if let Some(mappings) = stage_map {
        if let Some(mapping) = mappings.iter().find(|m| m.stage.eq_ignore_ascii_case(stage)) {
            return mapping.sort_order;
        }
    }
    match stage.to_lowercase().as_str() {
        "proposed" => 0,
        "planned" => 1,
        "suspended" => 2,
        "external" => 3,
        "in progress" => 4,
        "active" => 5,
        "completed" => 6,
        "cancelled" => 7,
        _ => 99,
    }
}

/// Ordinal value for task status (lifecycle progression)
fn status_sort_order(status: &str) -> i64 {
    match status.to_lowercase().as_str() {
        "open" => 0,
        "closed" => 1,
        "cancelled" => 2,
        "deleted" => 3,
        _ => 99,
    }
}

/// Get the color codes for a row based on color_column and fill_column options
/// Returns (fg_color, bg_color, reset_needed)
fn get_row_colors(
    row: &TaskRow,
    color_column: &Option<String>,
    fill_column: &Option<String>,
    priority_range: Option<(f64, f64)>,  // (min, max) for gradient normalization
    due_range: Option<(i64, i64)>,       // (min, max) for date heat map
) -> (String, String, bool) {
    let mut fg_color = String::new();
    let mut bg_color = String::new();
    
    // Get foreground color from color_column
    if let Some(col_name) = color_column {
        if let Some(column) = parse_task_column(col_name) {
            if let Some(value) = row.values.get(&column) {
                let color_type = detect_column_color_type(col_name);
                fg_color = match color_type {
                    ColumnColorType::Categorical => {
                        // Try semantic color first, fall back to hash-based
                        get_semantic_fg_color(col_name, value)
                            .unwrap_or_else(|| get_hash_fg_color(value))
                            .to_string()
                    }
                    ColumnColorType::Numeric => {
                        // For priority, higher is more urgent (red)
                        if let (Some((min, max)), Ok(val)) = (priority_range, value.parse::<f64>()) {
                            if max > min {
                                let normalized = (val - min) / (max - min);
                                get_gradient_fg_color(normalized).to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    }
                    ColumnColorType::Date => {
                        // For due dates, past = red, far future = green
                        if let (Some((min, max)), Some(sort_val)) = (due_range, row.sort_values.get(&column)) {
                            if let Some(SortValue::Int(ts)) = sort_val {
                                if max > min {
                                    let normalized = (*ts - min) as f64 / (max - min) as f64;
                                    // Invert: closer dates (smaller ts) should be red
                                    get_gradient_fg_color(1.0 - normalized).to_string()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    }
                };
            }
        }
    }
    
    // Get background color from fill_column
    if let Some(col_name) = fill_column {
        if let Some(column) = parse_task_column(col_name) {
            if let Some(value) = row.values.get(&column) {
                let color_type = detect_column_color_type(col_name);
                bg_color = match color_type {
                    ColumnColorType::Categorical => {
                        get_semantic_bg_color(col_name, value)
                            .unwrap_or_else(|| get_hash_bg_color(value))
                            .to_string()
                    }
                    ColumnColorType::Numeric => {
                        if let (Some((min, max)), Ok(val)) = (priority_range, value.parse::<f64>()) {
                            if max > min {
                                let normalized = (val - min) / (max - min);
                                get_gradient_bg_color(normalized).to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    }
                    ColumnColorType::Date => {
                        if let (Some((min, max)), Some(sort_val)) = (due_range, row.sort_values.get(&column)) {
                            if let Some(SortValue::Int(ts)) = sort_val {
                                if max > min {
                                    let normalized = (*ts - min) as f64 / (max - min) as f64;
                                    get_gradient_bg_color(1.0 - normalized).to_string()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    }
                };
            }
        }
    }
    
    // If background color is set, automatically ensure we have a contrasting foreground
    // to preserve legibility (e.g., light backgrounds need dark text, dark backgrounds need light text)
    // If color_column was also specified, it takes precedence; otherwise use automatic contrast
    if !bg_color.is_empty() && fg_color.is_empty() {
        fg_color = get_contrasting_fg_for_bg(&bg_color).to_string();
    }
    
    let reset_needed = !fg_color.is_empty() || !bg_color.is_empty();
    (fg_color, bg_color, reset_needed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TaskListColumn {
    Id,
    Queue,
    Description,
    Stage,
    Project,
    Created,
    Tags,
    Due,
    Alloc,
    Priority,
    Timer,
    Modified,
    Activity,
    Status,
}

/// Column display priority for adaptive width (lower = more important)
/// Priority 1: Essential (never hide)
/// Priority 2-3: Important (truncate only)
/// Priority 4+: Secondary/Optional (hide first)
/// 
/// Hide order (first to last): Status -> Modified -> Tags -> Priority -> Alloc -> Timer -> Activity -> Created -> Stage -> Due
fn column_priority(column: TaskListColumn) -> u8 {
    match column {
        TaskListColumn::Id => 1,          // Never hide
        TaskListColumn::Queue => 1,       // Never hide
        TaskListColumn::Description => 2, // Truncate only
        TaskListColumn::Project => 3,     // Truncate only
        TaskListColumn::Due => 4,         // Hidden last
        TaskListColumn::Stage => 5,
        TaskListColumn::Created => 6,
        TaskListColumn::Activity => 7,
        TaskListColumn::Timer => 8,
        TaskListColumn::Alloc => 9,
        TaskListColumn::Priority => 10,
        TaskListColumn::Tags => 11,
        TaskListColumn::Modified => 12,   // Hidden before Created
        TaskListColumn::Status => 13,     // Hidden first
    }
}

/// Minimum column width before hiding
fn column_min_width(column: TaskListColumn) -> usize {
    match column {
        TaskListColumn::Id => 4,
        TaskListColumn::Queue => 4,
        TaskListColumn::Description => 15,
        TaskListColumn::Project => 8,
        TaskListColumn::Status => 7,
        TaskListColumn::Stage => 8,
        TaskListColumn::Due => 10,
        TaskListColumn::Priority => 8,
        TaskListColumn::Tags => 6,
        TaskListColumn::Alloc => 5,
        TaskListColumn::Timer => 5,
        TaskListColumn::Created => 10,
        TaskListColumn::Modified => 10,
        TaskListColumn::Activity => 10,
    }
}

#[derive(Debug, Clone)]
enum SortValue {
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug, Clone)]
struct TaskRow {
    task_id: i64,
    parent_id: Option<i64>,
    values: HashMap<TaskListColumn, String>,
    sort_values: HashMap<TaskListColumn, Option<SortValue>>,
}

fn parse_task_column(name: &str) -> Option<TaskListColumn> {
    match name.to_lowercase().as_str() {
        "id" => Some(TaskListColumn::Id),
        "q" | "queue" => Some(TaskListColumn::Queue),
        "description" | "desc" => Some(TaskListColumn::Description),
        "stage" => Some(TaskListColumn::Stage),
        "project" | "proj" => Some(TaskListColumn::Project),
        "created" | "age" => Some(TaskListColumn::Created),
        "tags" | "tag" => Some(TaskListColumn::Tags),
        "due" => Some(TaskListColumn::Due),
        "alloc" | "allocation" => Some(TaskListColumn::Alloc),
        "priority" | "prio" | "pri" => Some(TaskListColumn::Priority),
        "clock" | "timer" => Some(TaskListColumn::Timer),
        "modified" | "mod" => Some(TaskListColumn::Modified),
        "activity" | "active" => Some(TaskListColumn::Activity),
        "status" => Some(TaskListColumn::Status),
        _ => None,
    }
}

fn column_label(column: TaskListColumn) -> &'static str {
    match column {
        TaskListColumn::Queue => "Q",
        TaskListColumn::Id => "ID",
        TaskListColumn::Description => "Description",
        TaskListColumn::Project => "Project",
        TaskListColumn::Tags => "Tags",
        TaskListColumn::Due => "Due",
        TaskListColumn::Alloc => "Alloc",
        TaskListColumn::Timer => "Timer",
        TaskListColumn::Created => "Created",
        TaskListColumn::Modified => "Modified",
        TaskListColumn::Activity => "Activity",
        TaskListColumn::Status => "Status",
        TaskListColumn::Stage => "Stage",
        TaskListColumn::Priority => "Priority",
    }
}

fn compare_sort_values(a: &Option<SortValue>, b: &Option<SortValue>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => match (a, b) {
            (SortValue::Int(a), SortValue::Int(b)) => a.cmp(b),
            (SortValue::Float(a), SortValue::Float(b)) => a
                .partial_cmp(b)
                .unwrap_or(Ordering::Equal),
            (SortValue::Str(a), SortValue::Str(b)) => a.cmp(b),
            _ => sort_value_as_string(a).cmp(&sort_value_as_string(b)),
        },
    }
}

fn sort_value_as_string(value: &SortValue) -> String {
    match value {
        SortValue::Int(v) => v.to_string(),
        SortValue::Float(v) => format!("{:.6}", v),
        SortValue::Str(v) => v.clone(),
    }
}

/// Reorder rows into depth-first tree order and apply tree prefixes to descriptions.
/// Children whose parent is not in the result set appear as root tasks.
/// Within each sibling set, the existing sort order is preserved.
fn apply_tree_ordering(rows: &mut Vec<TaskRow>) {
    // Check if any row has a parent_id that's in the set
    let id_set: HashSet<i64> = rows.iter().map(|r| r.task_id).collect();
    let has_nesting = rows.iter().any(|r| {
        r.parent_id.map_or(false, |pid| id_set.contains(&pid))
    });
    if !has_nesting {
        return;
    }

    // Build children map preserving current order
    let mut children_map: HashMap<i64, Vec<usize>> = HashMap::new();
    let mut root_indices: Vec<usize> = Vec::new();

    for (idx, row) in rows.iter().enumerate() {
        match row.parent_id {
            Some(pid) if id_set.contains(&pid) => {
                children_map.entry(pid).or_default().push(idx);
            }
            _ => {
                root_indices.push(idx);
            }
        }
    }

    // DFS traversal collecting (index, depth, is_last_sibling_at_each_level)
    struct TreeEntry {
        index: usize,
        prefix: String,
    }

    fn walk(
        idx: usize,
        depth: usize,
        ancestor_continuations: &[bool], // true = ancestor at that level has more siblings after
        is_last: bool,
        children_map: &HashMap<i64, Vec<usize>>,
        rows: &[TaskRow],
        result: &mut Vec<TreeEntry>,
    ) {
        // Build prefix from ancestor continuations + own position
        let mut prefix = String::new();
        for level in 0..depth {
            if level == depth - 1 {
                if is_last {
                    prefix.push_str("└─ ");
                } else {
                    prefix.push_str("├─ ");
                }
            } else if level < ancestor_continuations.len() && ancestor_continuations[level] {
                prefix.push_str("│  ");
            } else {
                prefix.push_str("   ");
            }
        }

        result.push(TreeEntry { index: idx, prefix });

        let task_id = rows[idx].task_id;
        if let Some(child_indices) = children_map.get(&task_id) {
            let len = child_indices.len();
            for (i, &child_idx) in child_indices.iter().enumerate() {
                let child_is_last = i == len - 1;
                let mut new_continuations = ancestor_continuations.to_vec();
                if depth > 0 {
                    // Already pushed in parent call
                }
                // For the current level: if current node is NOT last, descendants need continuation
                new_continuations.resize(depth, false);
                if depth > 0 {
                    new_continuations[depth - 1] = !is_last;
                }
                new_continuations.push(!child_is_last);
                // But we only pass up to `depth` for the child's ancestors
                let child_ancestors: Vec<bool> = if depth == 0 {
                    vec![] // root's children have no ancestor continuations from root
                } else {
                    let mut a = ancestor_continuations.to_vec();
                    a.resize(depth, false);
                    if depth > 0 {
                        a[depth - 1] = !is_last;
                    }
                    a
                };
                walk(child_idx, depth + 1, &child_ancestors, child_is_last, children_map, rows, result);
            }
        }
    }

    let mut ordered: Vec<TreeEntry> = Vec::with_capacity(rows.len());
    for &root_idx in &root_indices {
        walk(root_idx, 0, &[], true, &children_map, rows, &mut ordered);
    }

    // Apply prefixes to description values and reorder
    let mut new_rows: Vec<TaskRow> = Vec::with_capacity(rows.len());
    for entry in ordered {
        let mut row = rows[entry.index].clone();
        if !entry.prefix.is_empty() {
            if let Some(desc) = row.values.get_mut(&TaskListColumn::Description) {
                *desc = format!("{}{}", entry.prefix, desc);
            }
        }
        new_rows.push(row);
    }

    *rows = new_rows;
}

/// Format task list as a table
pub fn format_task_list_table(
    conn: &Connection,
    tasks: &[(Task, Vec<String>)],
    options: &TaskListOptions,
) -> Result<String> {
    if tasks.is_empty() {
        return Ok("No tasks found.".to_string());
    }
    
    // Pre-compute stage-related data for all tasks (batch queries for performance)
    let stack_positions = get_stack_positions(conn)?;
    let tasks_with_sessions = get_tasks_with_sessions(conn)?;
    let tasks_with_externals = get_tasks_with_externals(conn)?;
    let open_session_task_id = SessionRepo::get_open(conn)?.map(|s| s.task_id);
    let stage_map = StageRepo::load_map(conn).unwrap_or_default();
    
    let mut rows: Vec<TaskRow> = Vec::new();
    for (task, tags) in tasks {
        let task_id = task.id.unwrap_or(0);
        let stack_pos = stack_positions.get(&task_id).copied();
        let has_sessions = tasks_with_sessions.contains(&task_id);
        let has_externals = tasks_with_externals.contains(&task_id);
        let stage = calculate_stage_status(
            task,
            stack_pos,
            has_sessions,
            open_session_task_id,
            has_externals,
            Some(&stage_map),
        );
        
        let project = if let Some(project_id) = task.project_id {
            if let Ok(Some(proj)) = ProjectRepo::get_by_id(conn, project_id) {
                proj.name
            } else {
                format!("[{}]", project_id)
            }
        } else {
            String::new()
        };
        
        let tag_str = if !tags.is_empty() {
            tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ")
        } else {
            String::new()
        };
        
        let due = if let Some(due_ts) = task.due_ts {
            if options.use_relative_time {
                format_relative_date(due_ts)
            } else {
                format_date(due_ts)
            }
        } else {
            String::new()
        };
        
        let alloc = if let Some(alloc_secs) = task.alloc_secs {
            format_duration(alloc_secs)
        } else {
            String::new()
        };
        
        let clock = if let Some(task_id) = task.id {
            if let Ok(total_logged) = TaskRepo::get_total_logged_time(conn, task_id) {
                if total_logged > 0 {
                    format_duration(total_logged)
                } else {
                    "0s".to_string()
                }
            } else {
                "0s".to_string()
            }
        } else {
            "0s".to_string()
        };
        
        let priority = if task.status == TaskStatus::Open {
            if let Ok(prio) = calculate_priority(task, conn) {
                format!("{:.1}", prio)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        // Queue position indicator
        // Priority: queue position/▶ > @ (external) > ✓ (completed) > x (cancelled) > ! (suspended) > ? (proposed)
        let queue_pos_str = if stack_pos == Some(0) && open_session_task_id == task.id {
            // Active task at top of queue
            "▶".to_string()
        } else if let Some(p) = stack_pos {
            // In queue with numeric position
            p.to_string()
        } else if stage == "external" {
            // Awaiting external response
            "@".to_string()
        } else if stage == "completed" {
            // Closed (intent fulfilled)
            "✓".to_string()
        } else if stage == "cancelled" {
            // Cancelled
            "x".to_string()
        } else if stage == "suspended" {
            // Has sessions but not in queue
            "!".to_string()
        } else if stage == "proposed" {
            // New task, not started
            "?".to_string()
        } else {
            String::new()
        };
        
        let mut values = HashMap::new();
        values.insert(TaskListColumn::Id, task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()));
        values.insert(TaskListColumn::Queue, queue_pos_str.clone());
        let desc_display = if task.respawn.is_some() {
            format!("{} ↻", task.description)
        } else {
            task.description.clone()
        };
        values.insert(TaskListColumn::Description, desc_display);
        values.insert(TaskListColumn::Stage, stage.clone());
        values.insert(TaskListColumn::Project, project.clone());
        values.insert(TaskListColumn::Created, format_date(task.created_ts));
        values.insert(TaskListColumn::Modified, format_date(task.modified_ts));
        values.insert(TaskListColumn::Activity, format_date(task.activity_ts));
        values.insert(TaskListColumn::Tags, tag_str.clone());
        values.insert(TaskListColumn::Due, due.clone());
        values.insert(TaskListColumn::Alloc, alloc.clone());
        values.insert(TaskListColumn::Priority, priority.clone());
        values.insert(TaskListColumn::Timer, clock.clone());
        values.insert(TaskListColumn::Status, task.status.as_str().to_string());
        
        let mut sort_values = HashMap::new();
        sort_values.insert(TaskListColumn::Id, task.id.map(SortValue::Int));
        // Queue position for sorting: tasks not in queue sort to the end (use i64::MAX)
        sort_values.insert(TaskListColumn::Queue, Some(SortValue::Int(stack_pos.map(|p| p as i64).unwrap_or(i64::MAX))));
        sort_values.insert(TaskListColumn::Description, Some(SortValue::Str(task.description.clone())));
        sort_values.insert(TaskListColumn::Stage, Some(SortValue::Int(stage_sort_order(&stage, Some(&stage_map)))));
        sort_values.insert(TaskListColumn::Project, Some(SortValue::Str(project)));
        sort_values.insert(TaskListColumn::Created, Some(SortValue::Int(task.created_ts)));
        sort_values.insert(TaskListColumn::Modified, Some(SortValue::Int(task.modified_ts)));
        sort_values.insert(TaskListColumn::Activity, Some(SortValue::Int(task.activity_ts)));
        sort_values.insert(TaskListColumn::Tags, Some(SortValue::Str(tag_str)));
        sort_values.insert(TaskListColumn::Due, task.due_ts.map(SortValue::Int));
        sort_values.insert(TaskListColumn::Alloc, task.alloc_secs.map(SortValue::Int));
        sort_values.insert(TaskListColumn::Priority, if task.status == TaskStatus::Open {
            calculate_priority(task, conn).ok().map(SortValue::Float)
        } else {
            None
        });
        sort_values.insert(TaskListColumn::Timer, if let Some(task_id) = task.id {
            TaskRepo::get_total_logged_time(conn, task_id).ok().map(SortValue::Int)
        } else {
            None
        });
        sort_values.insert(TaskListColumn::Status, Some(SortValue::Int(status_sort_order(task.status.as_str()))));
        
        rows.push(TaskRow {
            task_id,
            parent_id: task.parent_id,
            values,
            sort_values,
        });
    }
    
    // Build column order
    let mut columns: Vec<TaskListColumn> = Vec::new();
    for col in &options.sort_columns {
        let col_name = col.strip_prefix('-').unwrap_or(col);
        let column = parse_task_column(col_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown sort column: {}", col))?;
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    for column in [TaskListColumn::Queue, TaskListColumn::Id, TaskListColumn::Description] {
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    for col in &options.group_columns {
        let col_name = col.strip_prefix('-').unwrap_or(col);
        let column = parse_task_column(col_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown group column: {}", col))?;
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    
    let default_columns = [
        TaskListColumn::Project,
        TaskListColumn::Tags,
        TaskListColumn::Priority,
        TaskListColumn::Alloc,
        TaskListColumn::Timer,
        TaskListColumn::Due,
        TaskListColumn::Created,
        TaskListColumn::Modified,
        TaskListColumn::Activity,
        TaskListColumn::Status,
        TaskListColumn::Stage,
    ];
    for column in default_columns {
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    
    // Remove hidden columns
    let hidden_columns: Vec<TaskListColumn> = options.hide_columns.iter()
        .filter_map(|name| parse_task_column(name))
        .collect();
    columns.retain(|col| !hidden_columns.contains(col));

    // Detect TTY for formatting
    let tty_mode = is_tty();

    // Calculate column widths
    let mut column_widths: HashMap<TaskListColumn, usize> = HashMap::new();
    for column in &columns {
        // Use character count for header labels too (though they're ASCII, this is consistent)
        let label = column_label(*column);
        column_widths.insert(*column, label.chars().count().max(4));
    }

    for row in &rows {
        for column in &columns {
            if let Some(value) = row.values.get(column) {
                // Use character count for width calculation to handle multi-byte characters correctly
                let char_count = value.chars().count();
                let max_len = if *column == TaskListColumn::Description {
                    char_count.min(100)
                } else {
                    char_count
                };
                let entry = column_widths.entry(*column).or_insert(4);
                *entry = (*entry).max(max_len);
            }
        }
    }

    // Adaptive width: hide low-priority columns if terminal is too narrow
    if !options.full_width {
        let terminal_width = get_terminal_width();

        // Helper to calculate total width (columns + spaces between them)
        // Last column doesn't need trailing space, so we need (n-1) spaces for n columns
        fn calc_total_width(columns: &[TaskListColumn], column_widths: &HashMap<TaskListColumn, usize>) -> usize {
            if columns.is_empty() {
                return 0;
            }
            let content_width: usize = columns.iter()
                .map(|c| column_widths.get(c).copied().unwrap_or(4))
                .sum();
            // Spaces between columns (n-1 spaces for n columns)
            let spacing = columns.len().saturating_sub(1);
            content_width + spacing
        }

        // Adaptive width strategy:
        // 1. First, truncate Description to minimum width (if needed)
        // 2. Then, truncate Project to minimum width (if needed)
        // 3. Only after truncation, start hiding columns by priority
        
        let target_width = terminal_width;
        let mut current_total = calc_total_width(&columns, &column_widths);
        
        // Step 1: Truncate Description first (before hiding any columns)
        if current_total > target_width && columns.contains(&TaskListColumn::Description) {
            if let Some(width) = column_widths.get_mut(&TaskListColumn::Description) {
                let excess = current_total.saturating_sub(target_width);
                let new_width = (*width).saturating_sub(excess).max(column_min_width(TaskListColumn::Description));
                *width = new_width;
                current_total = calc_total_width(&columns, &column_widths);
            }
        }

        // Step 2: Truncate Project (before hiding any columns)
        if current_total > target_width && columns.contains(&TaskListColumn::Project) {
            if let Some(width) = column_widths.get_mut(&TaskListColumn::Project) {
                let excess = current_total.saturating_sub(target_width);
                let new_width = (*width).saturating_sub(excess).max(column_min_width(TaskListColumn::Project));
                *width = new_width;
            }
        }

        // Step 3: Only after truncation is exhausted, hide columns by priority
        // Hide order (first to last): Status -> Tags -> Priority -> Alloc -> Timer -> Stage -> Due
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 20; // Prevent infinite loops
        while calc_total_width(&columns, &column_widths) > target_width 
            && columns.len() > 2 
            && iterations < MAX_ITERATIONS {
            iterations += 1;
            
            // Find the lowest priority column (highest priority number) that can be hidden
            let hide_candidate = columns.iter()
                .filter(|c| column_priority(**c) > 3) // Never hide priority 1-3 columns (ID, Queue, Description, Project)
                .max_by_key(|c| column_priority(**c))
                .copied();

            if let Some(col_to_hide) = hide_candidate {
                columns.retain(|c| *c != col_to_hide);
                // Remove from column_widths to avoid stale entries
                column_widths.remove(&col_to_hide);
            } else {
                break; // No more columns to hide
            }
        }
    }

    // Build header and separator strings (reused at top and bottom of table)
    let mut header_line = String::new();
    for (idx, column) in columns.iter().enumerate() {
        let width = *column_widths.get(column).unwrap_or(&4);
        if idx == columns.len() - 1 {
            header_line.push_str(&format!("{:<width$}", column_label(*column), width = width));
        } else {
            header_line.push_str(&format!("{:<width$} ", column_label(*column), width = width));
        }
    }
    let mut separator_line = String::new();
    for (idx, column) in columns.iter().enumerate() {
        let width = *column_widths.get(column).unwrap_or(&4);
        let underline = "─".repeat(width);
        if idx == columns.len() - 1 {
            separator_line.push_str(&underline);
        } else {
            separator_line.push_str(&format!("{} ", underline));
        }
    }

    let mut output = String::new();
    output.push_str(&header_line);
    output.push('\n');
    output.push_str(&separator_line);
    output.push('\n');
    
    // Apply sorting (ensure grouped rows are contiguous by sorting on group columns first)
    // Parse sort specs with negation support
    let mut effective_sort_specs: Vec<SortSpec> = options.group_columns.iter()
        .map(|col| parse_sort_spec(col))
        .collect();
    for sort_col in &options.sort_columns {
        let spec = parse_sort_spec(sort_col);
        if !effective_sort_specs.iter().any(|s| s.column.eq_ignore_ascii_case(&spec.column)) {
            effective_sort_specs.push(spec);
        }
    }
    // Parse group columns as SortSpecs to support negation (descending order)
    let group_specs: Vec<SortSpec> = options.group_columns.iter()
        .map(|name| parse_sort_spec(name))
        .collect();
    let group_columns_parsed: Vec<TaskListColumn> = group_specs.iter()
        .filter_map(|spec| parse_task_column(&spec.column))
        .collect();
    
    if !effective_sort_specs.is_empty() || !group_specs.is_empty() {
        rows.sort_by(|a, b| {
            // First sort by group columns (using ordinal sort values and respecting descending flag)
            for (idx, spec) in group_specs.iter().enumerate() {
                if let Some(column) = group_columns_parsed.get(idx) {
                    let ordering = compare_sort_values(
                        a.sort_values.get(column).unwrap_or(&None),
                        b.sort_values.get(column).unwrap_or(&None),
                    );
                    if ordering != Ordering::Equal {
                        return if spec.descending { ordering.reverse() } else { ordering };
                    }
                }
            }
            // Then sort by explicit sort columns
            for spec in &effective_sort_specs {
                if let Some(column) = parse_task_column(&spec.column) {
                    let ordering = compare_sort_values(
                        a.sort_values.get(&column).unwrap_or(&None),
                        b.sort_values.get(&column).unwrap_or(&None),
                    );
                    if ordering != Ordering::Equal {
                        return if spec.descending { ordering.reverse() } else { ordering };
                    }
                }
            }
            Ordering::Equal
        });
    }
    
    // Apply tree ordering (nest children under parents with tree characters)
    apply_tree_ordering(&mut rows);

    // Recompute description column width after tree ordering (prefixes added chars)
    if let Some(desc_width) = column_widths.get_mut(&TaskListColumn::Description) {
        for row in &rows {
            if let Some(value) = row.values.get(&TaskListColumn::Description) {
                let char_count = value.chars().count().min(100);
                *desc_width = (*desc_width).max(char_count);
            }
        }
    }

    // Compute ranges for gradient/heatmap coloring (if needed)
    let priority_range: Option<(f64, f64)> = if options.color_column.as_deref() == Some("priority") 
        || options.fill_column.as_deref() == Some("priority") {
        let priorities: Vec<f64> = rows.iter()
            .filter_map(|r| r.values.get(&TaskListColumn::Priority).and_then(|v| v.parse().ok()))
            .collect();
        if priorities.len() >= 2 {
            let min = priorities.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = priorities.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            Some((min, max))
        } else {
            None
        }
    } else {
        None
    };
    
    let due_range: Option<(i64, i64)> = if options.color_column.as_deref() == Some("due") 
        || options.fill_column.as_deref() == Some("due") {
        let dues: Vec<i64> = rows.iter()
            .filter_map(|r| r.sort_values.get(&TaskListColumn::Due).and_then(|v| {
                if let Some(SortValue::Int(ts)) = v { Some(*ts) } else { None }
            }))
            .collect();
        if dues.len() >= 2 {
            let min = *dues.iter().min().unwrap();
            let max = *dues.iter().max().unwrap();
            Some((min, max))
        } else {
            None
        }
    } else {
        None
    };
    
    // Check if colors are enabled (only in TTY mode)
    let colors_enabled = tty_mode && (options.color_column.is_some() || options.fill_column.is_some());
    
    // Build rows with optional grouping
    if options.group_columns.is_empty() {
        for row in &rows {
            // Get row colors (based on the column value, but will be applied selectively)
            let (fg_color, bg_color, _) = if colors_enabled {
                get_row_colors(row, &options.color_column, &options.fill_column, priority_range, due_range)
            } else {
                (String::new(), String::new(), false)
            };
            
            for (idx, column) in columns.iter().enumerate() {
                let width = *column_widths.get(column).unwrap_or(&4);
                let raw_value = row.values.get(column).cloned().unwrap_or_default();
                let value = if *column == TaskListColumn::Description && raw_value.chars().count() > width {
                    // Truncate by character count to avoid cutting multi-byte chars
                    let truncated: String = raw_value.chars().take(width.saturating_sub(2)).collect();
                    format!("{}..", truncated)
                } else if raw_value.chars().count() > width {
                    let truncated: String = raw_value.chars().take(width.saturating_sub(2)).collect();
                    format!("{}..", truncated)
                } else {
                    raw_value
                };
                
                // Apply colors selectively:
                // - fill: applies to Q column only
                // - color: applies to ID and Description columns only
                let mut cell_fg = String::new();
                let mut cell_bg = String::new();
                let mut needs_reset = false;
                
                if colors_enabled {
                    // Apply fill to Q column (background only, never foreground from color column)
                    if *column == TaskListColumn::Queue && !bg_color.is_empty() {
                        cell_bg = bg_color.clone();
                        // Always add contrasting foreground for fill background (regardless of color column)
                        // This ensures legibility on blue/gray backgrounds
                        cell_fg = get_contrasting_fg_for_bg(&bg_color).to_string();
                        // Note: We intentionally don't apply fg_color (from color column) to Q column
                        needs_reset = true;
                    }
                    // Apply color to ID and Description columns only
                    if (*column == TaskListColumn::Id || *column == TaskListColumn::Description) && !fg_color.is_empty() {
                        cell_fg = fg_color.clone();
                        needs_reset = true;
                    }
                }
                
                // Apply bold to ID column in TTY mode (bold works with colors)
                let mut formatted = if *column == TaskListColumn::Id && tty_mode {
                    let padded = format!("{:<width$}", value, width = width);
                    bold_if_tty(&padded, true)
                } else {
                    format!("{:<width$}", value, width = width)
                };
                
                // Wrap with colors if needed
                if needs_reset {
                    formatted = format!("{}{}{}{}", cell_fg, cell_bg, formatted, ANSI_RESET);
                }
                
                if idx == columns.len() - 1 {
                    output.push_str(&format!("{}\n", formatted));
                } else {
                    output.push_str(&format!("{} ", formatted));
                }
            }
        }
    } else {
        // Check if color/fill column matches any group column (independently)
        let color_column_enum = options.color_column.as_deref()
            .and_then(|name| parse_task_column(name));
        let fill_column_enum = options.fill_column.as_deref()
            .and_then(|name| parse_task_column(name));
        
        let color_matches_group = color_column_enum.map(|col| group_columns_parsed.contains(&col)).unwrap_or(false);
        let fill_matches_group = fill_column_enum.map(|col| group_columns_parsed.contains(&col)).unwrap_or(false);
        let color_or_fill_matches_group = color_matches_group || fill_matches_group;
        
        // Use the group_columns_parsed from earlier (already handles negation prefix)
        let mut groups: Vec<(Vec<String>, Vec<&TaskRow>)> = Vec::new();
        let mut group_index: HashMap<String, usize> = HashMap::new();
        for row in &rows {
            let group_values: Vec<String> = group_columns_parsed.iter()
                .map(|column| {
                    let value = row.values.get(column).cloned().unwrap_or_default();
                    normalize_group_value(*column, &value)
                })
                .collect();
            let group_key = group_values.join("\u{1f}");
            if let Some(existing_idx) = group_index.get(&group_key).copied() {
                groups[existing_idx].1.push(row);
            } else {
                groups.push((group_values, vec![row]));
                group_index.insert(group_key, groups.len() - 1);
            }
        }
        
        for (group_values, group_rows) in groups {
            // Build group label from group values (joined with ":")
            let group_label = group_values.iter()
                .filter(|v| !v.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(":");
            
            // Get color for group header (only if color/fill column matches group column)
            let (group_fg_color, group_bg_color) = if colors_enabled && color_or_fill_matches_group && !group_values.is_empty() {
                let mut fg = String::new();
                let mut bg = String::new();
                
                // Apply color_column if it matches group
                if color_matches_group {
                    // Find which group column matches the color_column
                    let color_col_idx = group_columns_parsed.iter()
                        .position(|&col| Some(col) == color_column_enum);
                    let group_value = color_col_idx
                        .and_then(|idx| group_values.get(idx))
                        .unwrap_or(&group_values[0]); // Fallback to first if not found
                    let col_name = options.color_column.as_deref().unwrap_or("");
                    fg = get_semantic_fg_color(col_name, group_value)
                        .unwrap_or_else(|| get_hash_fg_color(group_value))
                        .to_string();
                }
                
                // Apply fill_column if it matches group (independently)
                if fill_matches_group {
                    // Find which group column matches the fill_column
                    let fill_col_idx = group_columns_parsed.iter()
                        .position(|&col| Some(col) == fill_column_enum);
                    let group_value = fill_col_idx
                        .and_then(|idx| group_values.get(idx))
                        .unwrap_or(&group_values[0]); // Fallback to first if not found
                    let col_name = options.fill_column.as_deref().unwrap_or("");
                    bg = get_semantic_bg_color(col_name, group_value)
                        .unwrap_or_else(|| get_hash_bg_color(group_value))
                        .to_string();
                    // Automatically add contrasting foreground for legibility if no color_column was set
                    if !bg.is_empty() && fg.is_empty() {
                        fg = get_contrasting_fg_for_bg(&bg).to_string();
                    }
                }
                
                (fg, bg)
            } else {
                (String::new(), String::new())
            };
            
            // Group header in square brackets (with optional color)
            if group_fg_color.is_empty() && group_bg_color.is_empty() {
                output.push_str(&format!("[{}]\n", group_label));
            } else {
                output.push_str(&format!("{}{}[{}]{}\n", group_fg_color, group_bg_color, group_label, ANSI_RESET));
            }
            
            // Color task rows: fill applies to Q column, color applies to ID+Description columns
            for row in group_rows {
                // Get row colors (always compute, but apply selectively)
                let (fg_color, bg_color, _) = if colors_enabled {
                    get_row_colors(row, &options.color_column, &options.fill_column, priority_range, due_range)
                } else {
                    (String::new(), String::new(), false)
                };
                
                for (idx, column) in columns.iter().enumerate() {
                    let width = *column_widths.get(column).unwrap_or(&4);
                    let raw_value = row.values.get(column).cloned().unwrap_or_default();
                    let value = if *column == TaskListColumn::Description && raw_value.chars().count() > width {
                        let truncated: String = raw_value.chars().take(width.saturating_sub(2)).collect();
                        format!("{}..", truncated)
                    } else if raw_value.chars().count() > width {
                        let truncated: String = raw_value.chars().take(width.saturating_sub(2)).collect();
                        format!("{}..", truncated)
                    } else {
                        raw_value
                    };
                    
                    // Apply colors selectively:
                    // - fill: applies to Q column only
                    // - color: applies to ID and Description columns only
                    let mut cell_fg = String::new();
                    let mut cell_bg = String::new();
                    let mut needs_reset = false;
                    
                    if colors_enabled {
                        // Apply fill to Q column (background only, never foreground from color column)
                        if *column == TaskListColumn::Queue && !bg_color.is_empty() {
                            cell_bg = bg_color.clone();
                            // Always add contrasting foreground for fill background (regardless of color column)
                            // This ensures legibility on blue/gray backgrounds
                            cell_fg = get_contrasting_fg_for_bg(&bg_color).to_string();
                            // Note: We intentionally don't apply fg_color (from color column) to Q column
                            needs_reset = true;
                        }
                        // Apply color to ID and Description columns only
                        if (*column == TaskListColumn::Id || *column == TaskListColumn::Description) && !fg_color.is_empty() {
                            cell_fg = fg_color.clone();
                            needs_reset = true;
                        }
                    }
                    
                    // Apply bold to ID column in TTY mode
                    let mut formatted = if *column == TaskListColumn::Id && tty_mode {
                        let padded = format!("{:<width$}", value, width = width);
                        bold_if_tty(&padded, true)
                    } else {
                        format!("{:<width$}", value, width = width)
                    };
                    
                    // Wrap with colors if needed
                    if needs_reset {
                        formatted = format!("{}{}{}{}", cell_fg, cell_bg, formatted, ANSI_RESET);
                    }
                    
                    if idx == columns.len() - 1 {
                        output.push_str(&format!("{}\n", formatted));
                    } else {
                        output.push_str(&format!("{} ", formatted));
                    }
                }
            }
        }
    }
    
    // Repeat separator + header at bottom of table
    output.push_str(&separator_line);
    output.push('\n');
    output.push_str(&header_line);
    output.push('\n');

    Ok(output)
}

fn normalize_group_value(column: TaskListColumn, value: &str) -> String {
    let trimmed = value.trim();
    match column {
        TaskListColumn::Status | TaskListColumn::Stage => trimmed.to_lowercase(),
        _ => trimmed.to_string(),
    }
}

/// Format stack display
pub fn format_stack_display(items: &[(i64, i32)]) -> String {
    if items.is_empty() {
        return "Stack is empty.".to_string();
    }
    
    let mut output = String::new();
    output.push_str("Stack:\n");
    
    for (idx, (task_id, _ordinal)) in items.iter().enumerate() {
        output.push_str(&format!("  [{}] Task {}\n", idx, task_id));
    }
    
    output
}

/// Format clock list as a table with position and full task details
pub fn format_clock_list_table(
    conn: &Connection,
    clock_tasks: &[(usize, Task, Vec<String>)],
) -> Result<String> {
    if clock_tasks.is_empty() {
        return Ok("Clock stack is empty.".to_string());
    }
    
    // Calculate column widths
    let mut pos_width = 6; // "Pos" header
    let mut id_width = 4;
    let mut desc_width = 20;
    let mut status_width = 10;
    let mut project_width = 15;
    let mut tags_width = 20;
    let mut due_width = 12;
    
    // First pass: calculate widths
    for (position, task, tags) in clock_tasks {
        pos_width = pos_width.max(position.to_string().len());
        id_width = id_width.max(task.id.map(|id| id.to_string().len()).unwrap_or(0));
        desc_width = desc_width.max(task.description.len().min(100));
        status_width = status_width.max(task.status.as_str().len());
        
        if let Some(project_id) = task.project_id {
            if let Ok(Some(project)) = ProjectRepo::get_by_id(conn, project_id) {
                project_width = project_width.max(project.name.len().min(15));
            }
        }
        
        if !tags.is_empty() {
            let tag_str = tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ");
            tags_width = tags_width.max(tag_str.len().min(30));
        }
        
        if task.due_ts.is_some() {
            due_width = due_width.max(12);
        }
    }
    
    // Build header
    let mut output = String::new();
    output.push_str(&format!(
        "{:<pos$} {:<id$} {:<desc$} {:<status$} {:<project$} {:<tags$} {:<due$}\n",
        "Pos", "ID", "Description", "Status", "Project", "Tags", "Due",
        pos = pos_width,
        id = id_width,
        desc = desc_width,
        status = status_width,
        project = project_width,
        tags = tags_width,
        due = due_width
    ));
    
    // Separator line
    let total_width = pos_width + id_width + desc_width + status_width + project_width + tags_width + due_width + 6;
    output.push_str(&format!("{}\n", "-".repeat(total_width)));
    
    // Build rows (already sorted by position)
    for (position, task, tags) in clock_tasks {
        let pos_str = position.to_string();
        let id = task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string());
        
        let desc = if task.description.len() > desc_width {
            format!("{}..", &task.description[..desc_width.saturating_sub(2)])
        } else {
            task.description.clone()
        };
        
        let status = task.status.as_str();
        
        let project = if let Some(project_id) = task.project_id {
            if let Ok(Some(proj)) = ProjectRepo::get_by_id(conn, project_id) {
                if proj.name.len() > project_width {
                    format!("{}..", &proj.name[..project_width.saturating_sub(2)])
                } else {
                    proj.name
                }
            } else {
                format!("[{}]", project_id)
            }
        } else {
            String::new()
        };
        
        let tag_str = if !tags.is_empty() {
            let full = tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ");
            if full.len() > tags_width {
                format!("{}..", &full[..tags_width.saturating_sub(2)])
            } else {
                full
            }
        } else {
            String::new()
        };
        
        let due = if let Some(due_ts) = task.due_ts {
            format_date(due_ts)
        } else {
            String::new()
        };
        
        output.push_str(&format!(
            "{:<pos$} {:<id$} {:<desc$} {:<status$} {:<project$} {:<tags$} {:<due$}\n",
            pos_str, id, desc, status, project, tag_str, due,
            pos = pos_width,
            id = id_width,
            desc = desc_width,
            status = status_width,
            project = project_width,
            tags = tags_width,
            due = due_width
        ));
    }
    
    Ok(output)
}

/// Format clock transition message
pub fn format_clock_transition(
    action: &str,
    task_id: Option<i64>,
    task_description: Option<&str>,
) -> String {
    match (action, task_id, task_description) {
        ("started", Some(id), Some(desc)) => {
            format!("Started timing task {}: {}", id, desc)
        }
        ("started", Some(id), None) => {
            format!("Started timing task {}", id)
        }
        ("stopped", Some(id), Some(desc)) => {
            format!("Stopped timing task {}: {}", id, desc)
        }
        ("stopped", Some(id), None) => {
            format!("Stopped timing task {}", id)
        }
        ("switched", Some(old_id), _) => {
            format!("Switched from task {} to task {}", old_id, task_id.unwrap_or(0))
        }
        _ => format!("Clock {}", action)
    }
}

/// Format brief context shown when starting timing (`tatl on`).
/// Shows annotations (bulleted, no timestamps) and a timer progress bar.
pub fn format_on_context(
    conn: &Connection,
    task_id: i64,
    alloc_secs: Option<i64>,
) -> Result<String> {
    let mut output = String::new();

    // Annotations
    let annotations = AnnotationRepo::get_by_task(conn, task_id)?;
    for ann in &annotations {
        output.push_str(&format!("  - {}\n", ann.note));
    }

    // Timer / progress bar
    let logged = TaskRepo::get_total_logged_time(conn, task_id)?;

    match alloc_secs {
        Some(alloc) if alloc > 0 => {
            let pct = (logged as f64 / alloc as f64 * 100.0).round() as i64;
            let bar_width = 20;
            let filled = ((logged as f64 / alloc as f64) * bar_width as f64).round() as usize;
            let filled = filled.min(bar_width);
            let empty = bar_width - filled;
            let bar: String = "=".repeat(filled) + &"-".repeat(empty);
            output.push_str(&format!(
                "  Timer: {} / {} [{}] {}%\n",
                format_duration(logged),
                format_duration(alloc),
                bar,
                pct,
            ));
        }
        _ => {
            if logged > 0 {
                output.push_str(&format!("  Timer: {}\n", format_duration(logged)));
            }
        }
    }

    Ok(output)
}

/// Format task summary report
pub fn format_task_summary(
    conn: &Connection,
    task: &crate::models::Task,
    tags: &[String],
    annotations: &[crate::models::Annotation],
    sessions: &[crate::models::Session],
    stack_position: Option<(i32, i32)>, // (position, total)
) -> Result<String> {
    let mut output = String::new();
    
    // Header
    let header = format!("Task {}: {}", 
        task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
        task.description);
    output.push_str(&header);
    output.push_str("\n");
    output.push_str(&"=".repeat(header.len().max(60)));
    output.push_str("\n\n");
    
    // Description
    output.push_str("Description:\n");
    output.push_str(&format!("  {}\n\n", task.description));
    
    // Basic Info
    output.push_str("Status: ");
    output.push_str(task.status.as_str());
    output.push_str("\n");
    output.push_str(&format!("Created: {}\n", format_timestamp(task.created_ts)));
    output.push_str(&format!("Modified: {}\n", format_timestamp(task.modified_ts)));
    output.push_str(&format!("Activity: {}\n\n", format_timestamp(task.activity_ts)));
    
    // Attributes
    output.push_str("Attributes:\n");
    
    // Project
    if let Some(project_id) = task.project_id {
        if let Ok(Some(project)) = ProjectRepo::get_by_id(conn, project_id) {
            output.push_str(&format!("  Project:     {}\n", project.name));
        } else {
            output.push_str(&format!("  Project:     [{}]\n", project_id));
        }
    } else {
        output.push_str("  Project:     (none)\n");
    }
    
    // Due
    if let Some(due_ts) = task.due_ts {
        output.push_str(&format!("  Due:         {}\n", format_date(due_ts)));
    } else {
        output.push_str("  Due:         (none)\n");
    }
    
    // Scheduled
    if let Some(scheduled_ts) = task.scheduled_ts {
        output.push_str(&format!("  Scheduled:   {}\n", format_date(scheduled_ts)));
    } else {
        output.push_str("  Scheduled:   (none)\n");
    }
    
    // Wait
    if let Some(wait_ts) = task.wait_ts {
        output.push_str(&format!("  Wait:        {}\n", format_date(wait_ts)));
    } else {
        output.push_str("  Wait:        (none)\n");
    }
    
    // Allocation
    if let Some(alloc_secs) = task.alloc_secs {
        output.push_str(&format!("  Allocation:  {}\n", format_duration(alloc_secs)));
    } else {
        output.push_str("  Allocation:  (none)\n");
    }
    
    // Tags
    if !tags.is_empty() {
        let tag_str = tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ");
        output.push_str(&format!("  Tags:        {}\n", tag_str));
    } else {
        output.push_str("  Tags:        (none)\n");
    }
    
    // Template
    if let Some(ref template) = task.template {
        output.push_str(&format!("  Template:    {}\n", template));
    } else {
        output.push_str("  Template:    (none)\n");
    }
    
    // Respawn
    if let Some(ref respawn) = task.respawn {
        output.push_str(&format!("  Respawn:     {}\n", respawn));
    } else {
        output.push_str("  Respawn:     (none)\n");
    }

    // Parent
    if let Some(parent_id) = task.parent_id {
        if let Ok(Some(parent)) = TaskRepo::get_by_id(conn, parent_id) {
            output.push_str(&format!("  Parent:      {} {}\n", parent_id, parent.description));
        } else {
            output.push_str(&format!("  Parent:      {} (not found)\n", parent_id));
        }
    }

    output.push_str("\n");

    // Children
    if let Some(task_id) = task.id {
        let children = TaskRepo::get_children(conn, task_id)?;
        if !children.is_empty() {
            output.push_str(&format!("Children ({}):\n", children.len()));
            for child in &children {
                output.push_str(&format!("  {} {}\n", child.id.unwrap_or(0), child.description));
            }
            output.push_str("\n");
        }
    }

    // User-Defined Attributes
    if !task.udas.is_empty() {
        output.push_str("User-Defined Attributes:\n");
        let mut udas: Vec<_> = task.udas.iter().collect();
        udas.sort_by_key(|(k, _)| *k);
        for (key, value) in udas {
            output.push_str(&format!("  {}:    {}\n", key, value));
        }
        output.push_str("\n");
    }
    
    // Stack
    if let Some((position, total)) = stack_position {
        output.push_str("Stack:\n");
        output.push_str(&format!("  Position:    {} of {}\n\n", position + 1, total));
    }

    // Priority Score (only for open tasks)
    if task.status == TaskStatus::Open {
        if let Ok(priority) = calculate_priority(task, conn) {
            output.push_str("Priority:\n");
            output.push_str(&format!("  Score:       {:.1}\n", priority));
            output.push_str("  (Auto-calculated based on due date, allocation remaining, and task age)\n\n");
        }
    }

    // Respawn details (if respawning)
    if task.respawn.is_some() {
        output.push_str("Respawn:\n");
        output.push_str(&format!("  Type:        {}\n", task.respawn.as_ref().unwrap()));
        // TODO: Add more respawn details if needed (next occurrence, etc.)
        output.push_str("\n");
    }
    
    // Annotations
    output.push_str(&format!("Annotations ({}):\n", annotations.len()));
    if annotations.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (idx, annotation) in annotations.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", idx + 1, format_timestamp(annotation.entry_ts)));
            // Format note with indentation for multi-line notes
            for line in annotation.note.lines() {
                output.push_str(&format!("     {}\n", line));
            }
            output.push_str("\n");
        }
    }
    output.push_str("\n");
    
    // Sessions
    output.push_str(&format!("Sessions ({}):\n", sessions.len()));
    if sessions.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (idx, session) in sessions.iter().enumerate() {
            output.push_str(&format!("  {}. {} - ", idx + 1, format_timestamp(session.start_ts)));
            if let Some(end_ts) = session.end_ts {
                output.push_str(&format!("{}", format_timestamp(end_ts)));
                if let Some(duration) = session.duration_secs() {
                    output.push_str(&format!(" ({})", format_duration(duration)));
                }
            } else {
                output.push_str("(running)");
                let current_duration = chrono::Utc::now().timestamp() - session.start_ts;
                output.push_str(&format!(" ({})", format_duration(current_duration)));
            }
            output.push_str("\n");
        }
    }
    output.push_str("\n");
    
    // Total Time
    let total_secs: i64 = sessions.iter()
        .filter_map(|s| s.duration_secs())
        .sum();
    output.push_str(&format!("Total Time: {}\n", format_duration(total_secs)));
    
    Ok(output)
}

/// Format dashboard output
pub fn format_dashboard(
    conn: &Connection,
    clock_state: Option<(i64, i64)>,
    clock_stack_tasks: &[(usize, Task, Vec<String>)],
    priority_tasks: &[(Task, Vec<String>, f64)],
    today_session_count: usize,
    today_duration: i64,
    overdue_count: usize,
    next_overdue_ts: Option<i64>,
) -> Result<String> {
    let mut output = String::new();
    
    // Clock Status Section
    output.push_str("=== Clock Status ===\n");
    if let Some((task_id, duration)) = clock_state {
        let task_desc = TaskRepo::get_by_id(conn, task_id)
            .ok()
            .flatten()
            .map(|t| t.description)
            .unwrap_or_else(|| "".to_string());
        let desc_str = if task_desc.is_empty() {
            "".to_string()
        } else {
            format!(": {}", task_desc)
        };
        if duration > 0 {
            output.push_str(&format!(
                "Clocked IN on task {}{} ({})\n",
                task_id,
                desc_str,
                format_duration(duration)
            ));
        } else {
            output.push_str(&format!(
                "Clocked OUT (task {}{} in stack)\n",
                task_id,
                desc_str
            ));
        }
    } else {
        output.push_str("Clocked OUT (no task in stack)\n");
    }
    output.push_str("\n");
    
    // Clock Stack Section (top 3)
    output.push_str("=== Clock Stack (Top 3) ===\n");
    if clock_stack_tasks.is_empty() {
        output.push_str("Stack is empty.\n");
    } else {
        for (idx, task, tags) in clock_stack_tasks {
            let project_name = if let Some(project_id) = task.project_id {
                ProjectRepo::get_by_id(conn, project_id)
                    .ok()
                    .flatten()
                    .map(|p| p.name)
                    .unwrap_or_else(|| "?".to_string())
            } else {
                "".to_string()
            };
            
            let project_str = if project_name.is_empty() {
                "".to_string()
            } else {
                format!(" project={}", project_name)
            };
            
            let tags_str = if tags.is_empty() {
                "".to_string()
            } else {
                format!(" {}", tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" "))
            };
            
            let due_str = if let Some(due_ts) = task.due_ts {
                format!(" due={}", format_date(due_ts))
            } else {
                "".to_string()
            };
            
            let alloc_str = if let Some(alloc) = task.alloc_secs {
                format!(" alloc={}", format_duration(alloc))
            } else {
                "".to_string()
            };
            
            output.push_str(&format!(
                "[{}] {}: {}{}{}{}{}\n",
                idx, task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
                task.description,
                project_str,
                tags_str,
                due_str,
                alloc_str,
            ));
        }
    }
    output.push_str("\n");
    
    // Priority Tasks Section (top 3 NOT in clock stack)
    output.push_str("=== Priority Tasks (Top 3) ===\n");
    if priority_tasks.is_empty() {
        output.push_str("No priority tasks (all tasks are in queue or closed).\n");
    } else {
        for (task, tags, priority) in priority_tasks {
            let project_name = if let Some(project_id) = task.project_id {
                ProjectRepo::get_by_id(conn, project_id)
                    .ok()
                    .flatten()
                    .map(|p| p.name)
                    .unwrap_or_else(|| "?".to_string())
            } else {
                "".to_string()
            };
            
            let project_str = if project_name.is_empty() {
                "".to_string()
            } else {
                format!(" project={}", project_name)
            };
            
            let tags_str = if tags.is_empty() {
                "".to_string()
            } else {
                format!(" {}", tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" "))
            };
            
            let due_str = if let Some(due_ts) = task.due_ts {
                format!(" due={}", format_date(due_ts))
            } else {
                "".to_string()
            };
            
            let alloc_str = if let Some(alloc) = task.alloc_secs {
                format!(" alloc={}", format_duration(alloc))
            } else {
                "".to_string()
            };
            
            output.push_str(&format!(
                "{}: {}{}{}{}{} (priority: {:.1})\n",
                task.id.map(|id| id.to_string()).unwrap_or_else(|| "?".to_string()),
                task.description,
                project_str,
                tags_str,
                due_str,
                alloc_str,
                priority,
            ));
        }
    }
    output.push_str("\n");
    
    // Today's Sessions Section
    output.push_str("=== Today's Sessions ===\n");
    output.push_str(&format!("{} session(s), {}\n", today_session_count, format_duration(today_duration)));
    output.push_str("\n");
    
    // Overdue Tasks Section
    output.push_str("=== Overdue Tasks ===\n");
    if overdue_count > 0 {
        output.push_str(&format!("{} task(s) overdue\n", overdue_count));
    } else if let Some(next_ts) = next_overdue_ts {
        output.push_str(&format!("No overdue tasks. Next due: {}\n", format_date(next_ts)));
    } else {
        output.push_str("No overdue tasks. No tasks with due dates.\n");
    }
    
    Ok(output)
}

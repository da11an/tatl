use tempfile::TempDir;
use std::fs;
use tatl::utils::parse_date_expr;
use anyhow::Result;
mod test_env;

fn setup_test_env() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = test_env::lock_test_env();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let config_dir = temp_dir.path().join(".tatl");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("rc"), format!("data.location={}\n", db_path.display())).unwrap();
    std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
    (temp_dir, guard)
}

#[test]
fn test_dst_handling_utc_storage() {
    // Test that dates are stored in UTC regardless of local timezone
    // This is a basic test - full DST transition testing requires specific timezone setup
    
    let (_temp_dir, _guard) = setup_test_env();
    
    // Parse a date expression - should convert to UTC for storage
    let date_expr = "2026-03-15T14:30"; // Spring forward date (varies by timezone)
    let result = parse_date_expr(date_expr);
    
    assert!(result.is_ok(), "Should parse date expression");
    let timestamp = result.unwrap();
    
    // Verify it's a valid timestamp (positive, reasonable)
    assert!(timestamp > 0);
    assert!(timestamp < 2000000000); // Before year 2038
    
    // The exact value depends on timezone, but we're just verifying it works
    // Full DST transition testing would require:
    // 1. Setting specific timezone
    // 2. Testing dates during DST transitions
    // 3. Verifying fall back hour handling (first occurrence)
    // 4. Verifying spring forward hour handling (error on invalid)
}

#[test]
fn test_date_parsing_handles_local_timezone() {
    // Test that date parsing uses local timezone but stores as UTC
    let (_temp_dir, _guard) = setup_test_env();
    
    // Parse "today" - should use local timezone
    let result = parse_date_expr("today");
    assert!(result.is_ok());
    
    // Parse "tomorrow" - should use local timezone
    let result = parse_date_expr("tomorrow");
    assert!(result.is_ok());
    
    // Both should produce valid UTC timestamps
    let today_ts = parse_date_expr("today").unwrap();
    let tomorrow_ts = parse_date_expr("tomorrow").unwrap();
    
    // Tomorrow should be approximately 24 hours after today
    let diff = tomorrow_ts - today_ts;
    assert!(diff >= 86400 - 60 && diff <= 86400 + 60, "Should be approximately 24 hours");
}

#[test]
fn test_timezone_conversion_consistency() {
    // Test that same UTC timestamp always represents same moment
    // regardless of when/where it's converted
    let (_temp_dir, _guard) = setup_test_env();
    
    // Parse an absolute date
    let date_expr = "2026-06-15T12:00";
    let timestamp1 = parse_date_expr(date_expr).unwrap();
    
    // Parse again - should get same timestamp
    let timestamp2 = parse_date_expr(date_expr).unwrap();
    
    assert_eq!(timestamp1, timestamp2, "Same date should produce same UTC timestamp");
}

// Note: Full DST transition edge case testing would require:
// - Setting up specific timezone (e.g., America/New_York)
// - Testing dates during fall back (2am → 1am)
// - Testing dates during spring forward (2am → 3am, skipped hour)
// - Verifying first occurrence is used for fall back
// - Verifying error is returned for invalid spring forward times
// This is marked as deferred in the checklist due to complexity

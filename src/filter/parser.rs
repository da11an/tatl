//! Filter parser for task queries
//!
//! Implements boolean expression parsing with AND/OR/NOT operators.
//!
//! # Grammar
//!
//! ```text
//! filter := term | filter "or" term | "not" term
//! term := id | status=<status> | project=<name> | +tag | -tag | due=<expr> | due>expr | ...
//! ```
//!
//! # Precedence
//!
//! 1. `not` (highest)
//! 2. Implicit `and` (between adjacent terms)
//! 3. `or` (lowest)
//!
//! # Examples
//!
//! ```text
//! // Implicit AND
//! project=work +urgent
//!
//! // Explicit OR
//! +urgent or +important
//!
//! // NOT
//! not +waiting
//!
//! // Complex
//! project=work +urgent or project=home +important
//!
//! // Comparison operators
//! due>tomorrow due<=eod
//! ```

use crate::filter::evaluator::FilterExpr;

/// Comparison operators for filter expressions
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOp {
    Eq,    // =
    Neq,   // != or <>
    Gt,    // >
    Lt,    // <
    Gte,   // >=
    Lte,   // <=
}

/// Parse filter tokens into a FilterExpr
///
/// # Arguments
/// * `tokens` - Vector of filter tokens (e.g., `vec!["project=work".to_string(), "+urgent".to_string()]`)
///
/// # Returns
/// `FilterExpr` representing the parsed filter, or an error string if parsing fails
///
/// # Example
///
/// ```
/// use tatl::filter::parse_filter;
///
/// let filter = parse_filter(vec!["project=work".to_string(), "+urgent".to_string()]).unwrap();
/// ```
pub fn parse_filter(tokens: Vec<String>) -> Result<FilterExpr, String> {
    if tokens.is_empty() {
        return Ok(FilterExpr::All); // No filter = match all
    }

    // First pass: parse tokens into filter terms and operators
    let mut parsed: Vec<FilterToken> = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        // Check for operators
        if token == "or" {
            parsed.push(FilterToken::Or);
            i += 1;
            continue;
        }

        if token == "not" {
            parsed.push(FilterToken::Not);
            i += 1;
            continue;
        }

        // Parse as filter term
        match parse_filter_term(token) {
            Ok(Some(term)) => parsed.push(FilterToken::Term(term)),
            Ok(None) => return Err(format!("Invalid filter token: {}", token)),
            Err(err) => return Err(err),
        }

        i += 1;
    }

    // Second pass: build expression tree respecting precedence
    // Precedence: not > and > or
    build_expression(parsed)
}

#[derive(Debug, Clone)]
enum FilterToken {
    Term(FilterTerm),
    Not,
    Or,
}

#[derive(Debug, Clone)]
pub enum FilterTerm {
    Id(ComparisonOp, i64),
    Status(ComparisonOp, Vec<String>), // Supports comma-separated values
    Project(ComparisonOp, Vec<String>), // Supports comma-separated values (OR logic)
    Tag(String, bool), // (tag, is_positive)
    Due(ComparisonOp, String),
    Scheduled(ComparisonOp, String),
    Wait(ComparisonOp, String),
    Waiting,
    Stage(ComparisonOp, Vec<String>), // Supports comma-separated values
    Desc(ComparisonOp, String), // Description substring search (case-insensitive)
    External(ComparisonOp, String), // External recipient filter
}

/// Split a token into (key, operator, value) using operator detection.
/// Returns None if no operator is found.
fn split_on_operator(token: &str) -> Option<(String, ComparisonOp, String)> {
    // Find the first operator character position
    let op_start = token.find(|c: char| c == '=' || c == '>' || c == '<' || c == '!')?;

    let key = token[..op_start].to_string();
    if key.is_empty() {
        return None;
    }

    let rest = &token[op_start..];

    // Detect the operator from the remaining string
    let (op, op_len) = if rest.starts_with(">=") {
        (ComparisonOp::Gte, 2)
    } else if rest.starts_with("<=") {
        (ComparisonOp::Lte, 2)
    } else if rest.starts_with("!=") {
        (ComparisonOp::Neq, 2)
    } else if rest.starts_with("<>") {
        (ComparisonOp::Neq, 2)
    } else if rest.starts_with('=') {
        (ComparisonOp::Eq, 1)
    } else if rest.starts_with('>') {
        (ComparisonOp::Gt, 1)
    } else if rest.starts_with('<') {
        (ComparisonOp::Lt, 1)
    } else {
        return None;
    };

    let value = rest[op_len..].to_string();
    Some((key, op, value))
}

/// Known filter keys (exact match only)
const FILTER_KEYS: &[&str] = &[
    "id", "status", "project", "due", "scheduled", "wait",
    "stage", "desc", "description", "external",
];

/// Resolve a filter key, supporting unambiguous prefix abbreviations.
///
/// Rules:
/// - Exact (case-insensitive) match wins (e.g. "desc" resolves to "desc", not "description")
/// - Otherwise, if the prefix matches exactly one known key, expand it
/// - Otherwise, return an error listing matches (ambiguous) or known keys (unknown)
fn resolve_filter_key(key: &str) -> Result<String, String> {
    let key_lower = key.to_lowercase();

    // Exact match first (case-insensitive)
    for k in FILTER_KEYS {
        if k.eq_ignore_ascii_case(&key_lower) {
            return Ok((*k).to_string());
        }
    }

    // Prefix matches
    let matches: Vec<&str> = FILTER_KEYS
        .iter()
        .filter(|k| k.to_lowercase().starts_with(&key_lower))
        .copied()
        .collect();

    if matches.is_empty() {
        Err(format!(
            "Unknown filter field '{}'. Known fields: {}",
            key,
            FILTER_KEYS.join(", ")
        ))
    } else if matches.len() == 1 {
        Ok(matches[0].to_string())
    } else {
        Err(format!(
            "Ambiguous filter field '{}'. Matches: {}",
            key,
            matches.join(", ")
        ))
    }
}

/// Parse a single filter term token
fn parse_filter_term(token: &str) -> Result<Option<FilterTerm>, String> {
    // Bare numeric ID
    if let Ok(id) = token.parse::<i64>() {
        return Ok(Some(FilterTerm::Id(ComparisonOp::Eq, id)));
    }

    // Try to split on operator (=, >, <, >=, <=, !=, <>)
    if let Some((key, op, value)) = split_on_operator(token) {
        let key_resolved = resolve_filter_key(&key)?;
        return match key_resolved.as_str() {
            "id" => {
                if let Ok(id) = value.parse::<i64>() {
                    Ok(Some(FilterTerm::Id(op, id)))
                } else {
                    Ok(None)
                }
            }
            "status" => {
                if op != ComparisonOp::Eq && op != ComparisonOp::Neq {
                    return Err(format!("Status filter only supports '=' and '!=' operators, got '{}'", format_op(&op)));
                }
                let values: Vec<String> = value.split(',')
                    .map(|v| v.trim().to_lowercase())
                    .collect();
                Ok(Some(FilterTerm::Status(op, values)))
            },
            "project" => {
                if op != ComparisonOp::Eq && op != ComparisonOp::Neq {
                    return Err(format!("Project filter only supports '=' and '!=' operators, got '{}'", format_op(&op)));
                }
                let values: Vec<String> = value.split(',')
                    .map(|v| v.trim().to_string())
                    .collect();
                Ok(Some(FilterTerm::Project(op, values)))
            },
            "due" => Ok(Some(FilterTerm::Due(op, value))),
            "scheduled" => Ok(Some(FilterTerm::Scheduled(op, value))),
            "wait" => Ok(Some(FilterTerm::Wait(op, value))),
            "stage" => {
                if op != ComparisonOp::Eq && op != ComparisonOp::Neq {
                    return Err(format!("Stage filter only supports '=' and '!=' operators, got '{}'", format_op(&op)));
                }
                let values: Vec<String> = value.split(',')
                    .map(|v| v.trim().to_lowercase())
                    .collect();
                Ok(Some(FilterTerm::Stage(op, values)))
            },
            "desc" | "description" => {
                if op != ComparisonOp::Eq && op != ComparisonOp::Neq {
                    return Err(format!("Description filter only supports '=' and '!=' operators, got '{}'", format_op(&op)));
                }
                Ok(Some(FilterTerm::Desc(op, value)))
            },
            "external" => {
                if op != ComparisonOp::Eq && op != ComparisonOp::Neq {
                    return Err(format!("External filter only supports '=' and '!=' operators, got '{}'", format_op(&op)));
                }
                Ok(Some(FilterTerm::External(op, value)))
            },
            _ => Ok(None),
        };
    }

    // +tag or -tag
    if let Some(tag) = token.strip_prefix('+') {
        return Ok(Some(FilterTerm::Tag(tag.to_string(), true)));
    }
    if let Some(tag) = token.strip_prefix('-') {
        return Ok(Some(FilterTerm::Tag(tag.to_string(), false)));
    }

    // waiting (derived filter)
    if token == "waiting" {
        return Ok(Some(FilterTerm::Waiting));
    }

    Ok(None)
}

/// Format a ComparisonOp for display
fn format_op(op: &ComparisonOp) -> &'static str {
    match op {
        ComparisonOp::Eq => "=",
        ComparisonOp::Neq => "!=",
        ComparisonOp::Gt => ">",
        ComparisonOp::Lt => "<",
        ComparisonOp::Gte => ">=",
        ComparisonOp::Lte => "<=",
    }
}

/// Build expression tree from parsed tokens
/// Precedence: not > and > or
fn build_expression(tokens: Vec<FilterToken>) -> Result<FilterExpr, String> {
    if tokens.is_empty() {
        return Ok(FilterExpr::All);
    }

    // First, apply NOT operators (highest precedence)
    let mut after_not = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let FilterToken::Not = tokens[i] {
            if i + 1 >= tokens.len() {
                return Err("NOT operator requires a following term".to_string());
            }
            if let FilterToken::Term(term) = &tokens[i + 1] {
                after_not.push(FilterToken::Term(term.clone()));
                after_not.push(FilterToken::Not); // Mark as negated
                i += 2;
            } else {
                return Err("NOT operator must be followed by a term".to_string());
            }
        } else {
            after_not.push(tokens[i].clone());
            i += 1;
        }
    }

    // Now handle OR operators (lowest precedence)
    // Split by OR to get AND groups
    let mut or_groups: Vec<Vec<FilterToken>> = Vec::new();
    let mut current_group = Vec::new();

    for token in after_not {
        if let FilterToken::Or = token {
            if !current_group.is_empty() {
                or_groups.push(current_group);
                current_group = Vec::new();
            }
        } else {
            current_group.push(token);
        }
    }
    if !current_group.is_empty() {
        or_groups.push(current_group);
    }

    // Convert each group (AND group) to expression
    let mut or_exprs = Vec::new();
    for group in or_groups {
        let and_expr = build_and_expression(group)?;
        or_exprs.push(and_expr);
    }

    // Combine OR expressions
    if or_exprs.len() == 1 {
        Ok(or_exprs.remove(0))
    } else {
        Ok(FilterExpr::Or(or_exprs))
    }
}

/// Build AND expression from a group of terms (implicit AND)
fn build_and_expression(tokens: Vec<FilterToken>) -> Result<FilterExpr, String> {
    if tokens.is_empty() {
        return Ok(FilterExpr::All);
    }

    let mut and_terms = Vec::new();

    for token in tokens {
        match token {
            FilterToken::Term(term) => {
                and_terms.push(FilterExpr::Term(term));
            }
            FilterToken::Not => {
                // Apply NOT to the last term
                if let Some(last) = and_terms.pop() {
                    and_terms.push(FilterExpr::Not(Box::new(last)));
                } else {
                    return Err("NOT operator without preceding term".to_string());
                }
            }
            FilterToken::Or => {
                // Should not happen in AND group, but handle gracefully
                return Err("Unexpected OR in AND group".to_string());
            }
        }
    }

    if and_terms.len() == 1 {
        Ok(and_terms.remove(0))
    } else {
        Ok(FilterExpr::And(and_terms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::evaluator::FilterExpr;

    #[test]
    fn test_parse_simple_id() {
        let expr = parse_filter(vec!["10".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Id(ComparisonOp::Eq, 10)) => {}
            _ => panic!("Expected Id(10)"),
        }
    }

    #[test]
    fn test_parse_project_and_tag() {
        let expr = parse_filter(vec!["project=work".to_string(), "+urgent".to_string()]).unwrap();
        match expr {
            FilterExpr::And(terms) => {
                assert_eq!(terms.len(), 2);
            }
            _ => panic!("Expected And expression"),
        }
    }

    #[test]
    fn test_parse_or() {
        let expr = parse_filter(vec!["+urgent".to_string(), "or".to_string(), "+important".to_string()]).unwrap();
        match expr {
            FilterExpr::Or(_) => {}
            _ => panic!("Expected Or expression"),
        }
    }

    #[test]
    fn test_parse_not() {
        let expr = parse_filter(vec!["not".to_string(), "+waiting".to_string()]).unwrap();
        match expr {
            FilterExpr::Not(_) => {}
            _ => panic!("Expected Not expression"),
        }
    }

    #[test]
    fn test_parse_complex() {
        let expr = parse_filter(vec![
            "project=work".to_string(),
            "+urgent".to_string(),
            "or".to_string(),
            "project=home".to_string(),
            "+important".to_string(),
        ]).unwrap();
        match expr {
            FilterExpr::Or(_) => {}
            _ => panic!("Expected Or expression"),
        }
    }

    #[test]
    fn test_filter_status() {
        let expr = parse_filter(vec!["status=open".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Status(op, statuses)) => {
                assert_eq!(op, ComparisonOp::Eq);
                assert_eq!(statuses, vec!["open".to_string()]);
            }
            _ => panic!("Expected Status term"),
        }
    }

    #[test]
    fn test_filter_key_abbreviation_unambiguous() {
        // stat=... should expand to status=... (unambiguous prefix)
        let expr = parse_filter(vec!["stat=open".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Status(op, statuses)) => {
                assert_eq!(op, ComparisonOp::Eq);
                assert_eq!(statuses, vec!["open".to_string()]);
            }
            _ => panic!("Expected Status term"),
        }
    }

    #[test]
    fn test_filter_key_abbreviation_st_now_ambiguous() {
        // st=... is now ambiguous between status and stage
        let err = parse_filter(vec!["st=open".to_string()]).unwrap_err();
        assert!(err.contains("Ambiguous filter field"));
        assert!(err.contains("status"));
        assert!(err.contains("stage"));
    }

    #[test]
    fn test_filter_key_abbreviation_ambiguous_errors() {
        // d=... is ambiguous between due, desc, description
        let err = parse_filter(vec!["d=tomorrow".to_string()]).unwrap_err();
        assert!(err.contains("Ambiguous filter field"));
        assert!(err.contains("due"));
        assert!(err.contains("desc"));
    }

    #[test]
    fn test_filter_unknown_field_error() {
        let result = parse_filter(vec!["bogus=value".to_string()]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown filter field"));
    }

    #[test]
    fn test_filter_comparison_operators() {
        // due>tomorrow
        let expr = parse_filter(vec!["due>tomorrow".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Due(op, val)) => {
                assert_eq!(op, ComparisonOp::Gt);
                assert_eq!(val, "tomorrow");
            }
            _ => panic!("Expected Due term with Gt operator"),
        }

        // due<=eod
        let expr = parse_filter(vec!["due<=eod".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Due(op, val)) => {
                assert_eq!(op, ComparisonOp::Lte);
                assert_eq!(val, "eod");
            }
            _ => panic!("Expected Due term with Lte operator"),
        }

        // due!=none
        let expr = parse_filter(vec!["due!=none".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Due(op, val)) => {
                assert_eq!(op, ComparisonOp::Neq);
                assert_eq!(val, "none");
            }
            _ => panic!("Expected Due term with Neq operator"),
        }

        // due>=tomorrow
        let expr = parse_filter(vec!["due>=tomorrow".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Due(op, val)) => {
                assert_eq!(op, ComparisonOp::Gte);
                assert_eq!(val, "tomorrow");
            }
            _ => panic!("Expected Due term with Gte operator"),
        }
    }

    #[test]
    fn test_status_rejects_comparison() {
        let result = parse_filter(vec!["status>open".to_string()]);
        assert!(result.is_err());
    }
}

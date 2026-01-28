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
    Id(i64),
    Status(Vec<String>), // Status filter (pending, completed, closed) - supports comma-separated values
    Project(Vec<String>), // Project filter - supports comma-separated values (OR logic)
    Tag(String, bool), // (tag, is_positive)
    Due(ComparisonOp, String),
    Scheduled(ComparisonOp, String),
    Wait(ComparisonOp, String),
    Waiting,
    Kanban(Vec<String>), // Kanban status filter (proposed, stalled, queued, external, done) - supports comma-separated values
    Desc(String), // Description substring search (case-insensitive)
    External(String), // External recipient filter
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
    "kanban", "desc", "description", "external",
];

/// Parse a single filter term token
fn parse_filter_term(token: &str) -> Result<Option<FilterTerm>, String> {
    // Bare numeric ID
    if let Ok(id) = token.parse::<i64>() {
        return Ok(Some(FilterTerm::Id(id)));
    }

    // Try to split on operator (=, >, <, >=, <=, !=, <>)
    if let Some((key, op, value)) = split_on_operator(token) {
        let key_lower = key.to_lowercase();

        // Check for exact match in known keys
        if !FILTER_KEYS.contains(&key_lower.as_str()) {
            return Err(format!("Unknown filter field '{}'. Known fields: {}", key, FILTER_KEYS.join(", ")));
        }

        return match key_lower.as_str() {
            "id" => {
                if let Ok(id) = value.parse::<i64>() {
                    Ok(Some(FilterTerm::Id(id)))
                } else {
                    Ok(None)
                }
            }
            "status" => {
                if op != ComparisonOp::Eq {
                    return Err(format!("Status filter only supports '=' operator, got '{}'", format_op(&op)));
                }
                let values: Vec<String> = value.split(',')
                    .map(|v| v.trim().to_lowercase())
                    .collect();
                Ok(Some(FilterTerm::Status(values)))
            },
            "project" => {
                if op != ComparisonOp::Eq && op != ComparisonOp::Neq {
                    return Err(format!("Project filter only supports '=' and '!=' operators, got '{}'", format_op(&op)));
                }
                let values: Vec<String> = value.split(',')
                    .map(|v| v.trim().to_string())
                    .collect();
                if op == ComparisonOp::Neq {
                    // For != we still store as Project but wrap in Not at a higher level
                    // Actually, we need to handle this differently - negate is handled by evaluator
                    // For now, store with the Eq op and the caller handles Not wrapping
                    // Better approach: just return it and let the evaluator handle != via the existing Not mechanism
                    // Actually the simplest approach: return a Project term and let parse_filter wrap it in Not
                    // But that changes the API. Instead, let's just error for now and users can use "not project=X"
                    return Err("Use 'not project=value' instead of 'project!=value' for negation.".to_string());
                }
                Ok(Some(FilterTerm::Project(values)))
            },
            "due" => Ok(Some(FilterTerm::Due(op, value))),
            "scheduled" => Ok(Some(FilterTerm::Scheduled(op, value))),
            "wait" => Ok(Some(FilterTerm::Wait(op, value))),
            "kanban" => {
                if op != ComparisonOp::Eq {
                    return Err(format!("Kanban filter only supports '=' operator, got '{}'", format_op(&op)));
                }
                let values: Vec<String> = value.split(',')
                    .map(|v| v.trim().to_lowercase())
                    .collect();
                Ok(Some(FilterTerm::Kanban(values)))
            },
            "desc" | "description" => {
                if op != ComparisonOp::Eq {
                    return Err(format!("Description filter only supports '=' operator, got '{}'", format_op(&op)));
                }
                Ok(Some(FilterTerm::Desc(value)))
            },
            "external" => {
                if op != ComparisonOp::Eq {
                    return Err(format!("External filter only supports '=' operator, got '{}'", format_op(&op)));
                }
                Ok(Some(FilterTerm::External(value)))
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
            FilterExpr::Term(FilterTerm::Id(10)) => {}
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
        let expr = parse_filter(vec!["status=pending".to_string()]).unwrap();
        match expr {
            FilterExpr::Term(FilterTerm::Status(statuses)) => {
                assert_eq!(statuses, vec!["pending".to_string()]);
            }
            _ => panic!("Expected Status term"),
        }
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
        let result = parse_filter(vec!["status>pending".to_string()]);
        assert!(result.is_err());
    }
}

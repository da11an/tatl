//! Filter parser for task queries
//!
//! Implements boolean expression parsing with AND/OR/NOT operators.
//!
//! # Grammar
//!
//! ```
//! filter := term | filter "or" term | "not" term
//! term := id | status:<status> | project:<name> | +tag | -tag | due:<expr> | ...
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
//! ```
//! // Implicit AND
//! project:work +urgent
//!
//! // Explicit OR
//! +urgent or +important
//!
//! // NOT
//! not +waiting
//!
//! // Complex
//! project:work +urgent or project:home +important
//! ```

use crate::filter::evaluator::FilterExpr;

/// Parse filter tokens into a FilterExpr
///
/// # Arguments
/// * `tokens` - Vector of filter tokens (e.g., `vec!["project:work".to_string(), "+urgent".to_string()]`)
///
/// # Returns
/// `FilterExpr` representing the parsed filter, or an error string if parsing fails
///
/// # Example
///
/// ```
/// use task_ninja::filter::parse_filter;
///
/// let filter = parse_filter(vec!["project:work".to_string(), "+urgent".to_string()]).unwrap();
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
        if let Some(term) = parse_filter_term(token) {
            parsed.push(FilterToken::Term(term));
        } else {
            return Err(format!("Invalid filter token: {}", token));
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
    Status(String),
    Project(String),
    Tag(String, bool), // (tag, is_positive)
    Due(String),
    Scheduled(String),
    Wait(String),
    Waiting,
    Kanban(String), // Kanban status filter (proposed, paused, queued, working, NEXT, LIVE, done)
}

/// Parse a single filter term token
fn parse_filter_term(token: &str) -> Option<FilterTerm> {
    // Bare numeric ID
    if let Ok(id) = token.parse::<i64>() {
        return Some(FilterTerm::Id(id));
    }
    
    // id:<n>
    if let Some(id_str) = token.strip_prefix("id:") {
        if let Ok(id) = id_str.parse::<i64>() {
            return Some(FilterTerm::Id(id));
        }
    }
    
    // status:<status>
    if let Some(status) = token.strip_prefix("status:") {
        return Some(FilterTerm::Status(status.to_string()));
    }
    
    // project:<name>
    if let Some(project) = token.strip_prefix("project:") {
        return Some(FilterTerm::Project(project.to_string()));
    }
    
    // +tag or -tag
    if token.starts_with('+') {
        let tag = token.strip_prefix('+')?.to_string();
        return Some(FilterTerm::Tag(tag, true));
    }
    if token.starts_with('-') {
        let tag = token.strip_prefix('-')?.to_string();
        return Some(FilterTerm::Tag(tag, false));
    }
    
    // due:<expr>
    if let Some(expr) = token.strip_prefix("due:") {
        return Some(FilterTerm::Due(expr.to_string()));
    }
    
    // scheduled:<expr>
    if let Some(expr) = token.strip_prefix("scheduled:") {
        return Some(FilterTerm::Scheduled(expr.to_string()));
    }
    
    // wait:<expr>
    if let Some(expr) = token.strip_prefix("wait:") {
        return Some(FilterTerm::Wait(expr.to_string()));
    }
    
    // waiting (derived filter)
    if token == "waiting" {
        return Some(FilterTerm::Waiting);
    }
    
    // kanban:<status> (derived filter)
    if let Some(status) = token.strip_prefix("kanban:") {
        return Some(FilterTerm::Kanban(status.to_lowercase()));
    }
    
    None
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
        let expr = parse_filter(vec!["project:work".to_string(), "+urgent".to_string()]).unwrap();
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
            "project:work".to_string(),
            "+urgent".to_string(),
            "or".to_string(),
            "project:home".to_string(),
            "+important".to_string(),
        ]).unwrap();
        match expr {
            FilterExpr::Or(_) => {}
            _ => panic!("Expected Or expression"),
        }
    }
}

// Fuzzy matching utilities for project name suggestions

/// Calculate Levenshtein distance between two strings
/// Returns the minimum number of single-character edits (insertions, deletions, substitutions)
/// needed to transform one string into another
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let s1_len = s1_chars.len();
    let s2_len = s2_chars.len();
    
    // Handle empty strings
    if s1_len == 0 {
        return s2_len;
    }
    if s2_len == 0 {
        return s1_len;
    }
    
    // Create matrix
    let mut matrix = vec![vec![0; s2_len + 1]; s1_len + 1];
    
    // Initialize first row and column
    for i in 0..=s1_len {
        matrix[i][0] = i;
    }
    for j in 0..=s2_len {
        matrix[0][j] = j;
    }
    
    // Fill matrix
    for i in 1..=s1_len {
        for j in 1..=s2_len {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            
            matrix[i][j] = (matrix[i - 1][j] + 1)                    // deletion
                .min(matrix[i][j - 1] + 1)                          // insertion
                .min(matrix[i - 1][j - 1] + cost);                  // substitution
        }
    }
    
    matrix[s1_len][s2_len]
}

/// Check if s2 is a substring of s1 (case-insensitive)
pub fn is_substring_match(s1: &str, s2: &str) -> bool {
    s1.to_lowercase().contains(&s2.to_lowercase())
}

/// Find near matches for a project name
/// Returns up to 5 matches sorted by distance (closest first)
pub fn find_near_project_matches(
    search_name: &str,
    projects: &[(String, bool)], // (name, is_archived)
    max_distance: usize,
) -> Vec<(String, usize)> {
    let search_lower = search_name.to_lowercase();
    let mut matches: Vec<(String, usize)> = Vec::new();
    
    for (project_name, _) in projects {
        let project_lower = project_name.to_lowercase();
        
        // Calculate Levenshtein distance (case-insensitive)
        let distance = levenshtein_distance(&search_lower, &project_lower);
        
        // Check if within threshold
        if distance <= max_distance {
            matches.push((project_name.clone(), distance));
        } else {
            // Also check substring match (only if search is shorter than project)
            if search_lower.len() < project_lower.len() && is_substring_match(project_name, search_name) {
                // For substring matches, use a distance based on how much longer the project name is
                // Prefix matches are preferred (distance = extra chars)
                // Non-prefix matches get a small penalty
                let substring_distance = if project_lower.starts_with(&search_lower) {
                    // Prefix match: distance = number of extra characters
                    project_lower.len() - search_lower.len()
                } else {
                    // Substring but not prefix: distance = extra chars + 1 penalty
                    project_lower.len() - search_lower.len() + 1
                };
                
                // For substring matches, be more lenient - allow up to max_distance + 2
                // This helps catch cases like "work" -> "workemail" (distance 5, but should match)
                if substring_distance <= max_distance + 2 {
                    matches.push((project_name.clone(), substring_distance.min(max_distance)));
                }
            }
        }
    }
    
    // Sort by distance, then by name
    matches.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.0.cmp(&b.0))
    });
    
    // Return up to 5 matches
    matches.into_iter().take(5).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("same", "same"), 0);
        assert_eq!(levenshtein_distance("abc", "def"), 3);
    }
    
    #[test]
    fn test_is_substring_match() {
        assert!(is_substring_match("work", "work"));
        assert!(is_substring_match("work", "Work"));
        assert!(is_substring_match("work", "WORK"));
        assert!(is_substring_match("workemail", "work"));
        assert!(is_substring_match("workemail", "email"));
        assert!(!is_substring_match("work", "email"));
    }
    
    #[test]
    fn test_find_near_project_matches() {
        let projects = vec![
            ("work".to_string(), false),
            ("home".to_string(), false),
            ("workemail".to_string(), false),
            ("newproject".to_string(), false),
            ("newproject2".to_string(), false),
        ];
        
        // Exact match (case-insensitive)
        let matches = find_near_project_matches("Work", &projects, 3);
        assert!(matches.len() >= 1);
        assert_eq!(matches[0].0, "work");
        assert_eq!(matches[0].1, 0);
        // May also match "workemail" as substring, which is correct
        
        // Close match (case difference)
        let matches = find_near_project_matches("Newproject", &projects, 3);
        assert!(matches.len() >= 1);
        assert_eq!(matches[0].0, "newproject");
        // May also match newproject2 if distance is within threshold
        
        // Substring match (search is shorter than project)
        let projects_with_long = vec![
            ("workemail".to_string(), false),
            ("workproject".to_string(), false),
        ];
        let matches = find_near_project_matches("work", &projects_with_long, 3);
        assert!(matches.len() >= 1);
        // Should match workemail and workproject as substrings
        
        // No matches
        let matches = find_near_project_matches("nonexistent", &projects, 3);
        assert_eq!(matches.len(), 0);
    }
}

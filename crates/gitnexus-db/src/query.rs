//! Query utilities for Cypher string handling and result formatting.

use serde_json::Value;

/// Escape a string for safe embedding in a Cypher query.
///
/// Prevents Cypher injection by escaping single quotes, double quotes,
/// backslashes, and newlines.
pub fn escape_cypher_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\'' => escaped.push_str("\\'"),
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\0' => {} // strip null bytes
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Strip single-line (`//`) and block (`/* */`) comments from a Cypher query.
fn strip_cypher_comments(query: &str) -> String {
    let mut result = String::with_capacity(query.len());
    let chars: Vec<char> = query.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            // Skip line comment
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            // Skip block comment
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2; // skip */
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Check if a Cypher query is a write/mutation query.
///
/// Returns `true` if the query contains any write operations:
/// CREATE, DELETE, SET, MERGE, DROP, REMOVE, DETACH, ALTER.
/// Uses case-insensitive word-boundary matching to avoid false positives
/// (e.g., matching "CREATE" inside a string literal is acceptable for
/// safety since we err on the side of caution).
/// Comments are stripped before checking to prevent bypass via comment injection.
pub fn is_write_query(query: &str) -> bool {
    let cleaned = strip_cypher_comments(query);
    let upper = cleaned.to_uppercase();
    // Check for write keywords as whole words (preceded by whitespace or start-of-string)
    let write_patterns = [
        "CREATE", "DELETE", "SET ", "MERGE", "DROP", "REMOVE", "DETACH", "ALTER",
    ];

    for pattern in &write_patterns {
        // Check if the pattern appears as a standalone keyword
        if let Some(pos) = upper.find(pattern) {
            // Verify it's at a word boundary (start of string or preceded by whitespace/paren)
            if pos == 0 {
                return true;
            }
            let prev_byte = upper.as_bytes()[pos - 1];
            if prev_byte == b' '
                || prev_byte == b'\n'
                || prev_byte == b'\t'
                || prev_byte == b'('
                || prev_byte == b'{'
                || prev_byte == b';'
            {
                return true;
            }
        }
    }

    false
}

/// Format query result rows as a JSON array string.
pub fn format_query_result(rows: &[Value]) -> String {
    let arr = Value::Array(rows.to_vec());
    serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// Format query result rows as a compact JSON array string.
pub fn format_query_result_compact(rows: &[Value]) -> String {
    let arr = Value::Array(rows.to_vec());
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_escape_cypher_string() {
        assert_eq!(escape_cypher_string("hello"), "hello");
        assert_eq!(escape_cypher_string("it's"), "it\\'s");
        assert_eq!(escape_cypher_string("line\nbreak"), "line\\nbreak");
        assert_eq!(
            escape_cypher_string(r#"say "hi""#),
            r#"say \"hi\""#
        );
        assert_eq!(escape_cypher_string("back\\slash"), "back\\\\slash");
        assert_eq!(escape_cypher_string("null\x00byte"), "nullbyte");
    }

    #[test]
    fn test_is_write_query() {
        // Write queries
        assert!(is_write_query("CREATE (n:File {id: '1'})"));
        assert!(is_write_query("MATCH (n) DELETE n"));
        assert!(is_write_query("MATCH (n) SET n.name = 'x'"));
        assert!(is_write_query("MERGE (n:File {id: '1'})"));
        assert!(is_write_query("DROP TABLE File"));
        assert!(is_write_query("MATCH (n) REMOVE n.name"));
        assert!(is_write_query("MATCH (n) DETACH DELETE n"));

        // Read queries
        assert!(!is_write_query("MATCH (n:File) RETURN n"));
        assert!(!is_write_query("MATCH (n)-[r]->(m) RETURN n, r, m"));
        assert!(!is_write_query(
            "MATCH (n:File) WHERE n.name = 'test' RETURN n"
        ));
    }

    #[test]
    fn test_is_write_query_case_insensitive() {
        assert!(is_write_query("create (n:File {id: '1'})"));
        assert!(is_write_query("match (n) delete n"));
    }

    #[test]
    fn test_format_query_result() {
        let rows = vec![
            json!({"name": "main", "type": "Function"}),
            json!({"name": "User", "type": "Class"}),
        ];
        let formatted = format_query_result(&rows);
        assert!(formatted.contains("main"));
        assert!(formatted.contains("User"));

        let compact = format_query_result_compact(&rows);
        assert!(!compact.contains('\n'));
    }

    #[test]
    fn test_format_empty_result() {
        let rows: Vec<Value> = vec![];
        assert_eq!(format_query_result(&rows), "[]");
    }
}

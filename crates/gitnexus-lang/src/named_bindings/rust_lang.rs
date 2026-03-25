use super::types::NamedBinding;

/// Extract named bindings from a Rust use declaration.
///
/// Handles:
/// - `use crate::models::User`
/// - `use crate::models::{User, Repo as R}`
pub fn extract(import_text: &str) -> Option<Vec<NamedBinding>> {
    let text = import_text.trim().trim_end_matches(';');

    // Grouped imports: use xxx::{A, B as C}
    if let Some(open) = text.find('{') {
        if let Some(close) = text.find('}') {
            let inner = &text[open + 1..close];
            let mut bindings = Vec::new();
            for part in inner.split(',') {
                let part = part.trim();
                if part.is_empty() || part == "self" {
                    continue;
                }
                if let Some(as_pos) = part.find(" as ") {
                    let exported = part[..as_pos].trim();
                    let local = part[as_pos + 4..].trim();
                    bindings.push(NamedBinding::new(local, exported));
                } else {
                    bindings.push(NamedBinding::new(part, part));
                }
            }
            return if bindings.is_empty() { None } else { Some(bindings) };
        }
    }

    // Single import: use crate::models::User or use crate::models::User as U
    if let Some(as_pos) = text.rfind(" as ") {
        let before = &text[..as_pos];
        let local = text[as_pos + 4..].trim();
        if let Some(last_sep) = before.rfind("::") {
            let exported = &before[last_sep + 2..];
            return Some(vec![NamedBinding::new(local, exported.trim())]);
        }
    }

    // Simple: last path segment
    if let Some(last_sep) = text.rfind("::") {
        let name = &text[last_sep + 2..];
        let name = name.trim();
        if !name.is_empty() && name != "*" && name != "self" {
            return Some(vec![NamedBinding::new(name, name)]);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_use() {
        let bindings = extract("use crate::models::User").unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].local, "User");
    }

    #[test]
    fn test_grouped_use() {
        let bindings = extract("use crate::models::{User, Repo}").unwrap();
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_aliased_use() {
        let bindings = extract("use crate::models::{User as U}").unwrap();
        assert_eq!(bindings[0].local, "U");
        assert_eq!(bindings[0].exported, "User");
    }
}

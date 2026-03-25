use super::types::NamedBinding;

/// Extract named import bindings from a TypeScript/JavaScript import statement.
///
/// Handles:
/// - `import { Foo, Bar as Baz } from './module'`
/// - `export { X } from './y'`
pub fn extract(import_text: &str) -> Option<Vec<NamedBinding>> {
    let text = import_text.trim();

    // Find the braces containing named imports/exports
    let open = text.find('{')?;
    let close = text.find('}')?;
    if close <= open {
        return None;
    }

    let inner = &text[open + 1..close];
    let mut bindings = Vec::new();

    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Check for "X as Y" pattern
        if let Some(as_pos) = part.find(" as ") {
            let exported = part[..as_pos].trim();
            let local = part[as_pos + 4..].trim();
            if !exported.is_empty() && !local.is_empty() {
                bindings.push(NamedBinding::new(local, exported));
            }
        } else {
            // Simple import: local == exported
            if !part.is_empty() {
                bindings.push(NamedBinding::new(part, part));
            }
        }
    }

    if bindings.is_empty() {
        None
    } else {
        Some(bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_imports() {
        let bindings = extract("import { User, Repo } from './models'").unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].local, "User");
        assert_eq!(bindings[0].exported, "User");
    }

    #[test]
    fn test_aliased_import() {
        let bindings = extract("import { User as U, Repo } from './models'").unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].local, "U");
        assert_eq!(bindings[0].exported, "User");
        assert_eq!(bindings[1].local, "Repo");
    }

    #[test]
    fn test_export_from() {
        let bindings = extract("export { handler } from './api'").unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].local, "handler");
    }

    #[test]
    fn test_no_braces() {
        assert!(extract("import express from 'express'").is_none());
    }
}

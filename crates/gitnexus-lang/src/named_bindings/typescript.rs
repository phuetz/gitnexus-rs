use super::types::NamedBinding;

/// Split a string on top-level commas only, ignoring commas nested inside
/// `<...>` (generics), `{...}` (object/template type literals), `(...)`,
/// or `[...]`. Used so that an import like
/// `import { foo: Map<K, V>, bar } from './x'` is not split inside the generic.
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' | '{' | '(' | '[' => depth += 1,
            '>' | '}' | ')' | ']' => depth = (depth - 1).max(0),
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

/// Extract named import bindings from a TypeScript/JavaScript import statement.
///
/// Handles:
/// - `import { Foo, Bar as Baz } from './module'`
/// - `export { X } from './y'`
pub fn extract(import_text: &str) -> Option<Vec<NamedBinding>> {
    let text = import_text.trim();

    // Find the braces containing named imports/exports.
    // Use `rfind('}')` so a nested `}` (e.g. from inline type annotations or
    // template-literal types like `import { type Foo<{ bar }>, Baz } from ...`)
    // does not truncate the binding list.
    let open = text.find('{')?;
    let close = text.rfind('}')?;
    if close <= open {
        return None;
    }

    let inner = &text[open + 1..close];
    let mut bindings = Vec::new();

    // Split on top-level commas only — commas inside nested generics like
    // `Map<K, V>` or template-literal types must not break a binding apart.
    for part in split_top_level_commas(inner) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // Strip optional `type` modifier in TS type-only imports.
        let part = part
            .strip_prefix("type ")
            .map(str::trim_start)
            .unwrap_or(part);

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

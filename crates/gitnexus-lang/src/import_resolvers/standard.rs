//! Standard import resolver for TypeScript and JavaScript.
//!
//! Resolution order:
//! 1. Relative paths (`./`, `../`) — resolve against the importing file's directory.
//! 2. Path aliases from tsconfig.json (`@/`, `~/`, custom patterns).
//! 3. Bare specifiers — try suffix match (node_modules are excluded from the file list).

use super::types::{ImportResult, ResolveCtx};
use super::utils;

/// Resolve a TypeScript/JavaScript import path.
///
/// Handles relative imports, tsconfig path aliases, and bare specifiers.
/// Index file resolution (e.g., `./components` -> `./components/index.ts`)
/// is handled automatically by the suffix extension list.
pub fn resolve(raw_path: &str, file_path: &str, ctx: &ResolveCtx<'_>) -> ImportResult {
    let cleaned = utils::normalize_import_path(raw_path);

    if cleaned.is_empty() {
        return ImportResult::Unresolved;
    }

    // ── 1. Relative imports ──────────────────────────────────────────────
    if utils::is_relative_path(&cleaned) {
        let resolved = utils::resolve_relative(&cleaned, file_path);
        return utils::resolve_by_suffix(&resolved, ctx);
    }

    // ── 2. Path aliases (tsconfig.json) ──────────────────────────────────
    let alias_result = utils::resolve_ts_path_alias(&cleaned, ctx);
    if !matches!(alias_result, ImportResult::Unresolved) {
        return alias_result;
    }

    // ── 3. Bare specifier (suffix match) ─────────────────────────────────
    // Try the import path directly as a suffix (works for monorepo packages
    // and project-local absolute imports configured via baseUrl).
    if let Some(base_url) = &ctx.configs.ts_base_url {
        let with_base = format!("{base_url}/{cleaned}");
        let result = utils::resolve_by_suffix(&with_base, ctx);
        if !matches!(result, ImportResult::Unresolved) {
            return result;
        }
    }

    // Try direct suffix match as last resort
    utils::resolve_by_suffix(&cleaned, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{ImportConfigs, SuffixIndex};
    use std::collections::{HashMap, HashSet};

    fn make_ctx<'a>(
        files: &'a [String],
        suffix_index: &'a SuffixIndex,
        configs: &'a ImportConfigs,
    ) -> ResolveCtx<'a> {
        let all_set: HashSet<String> = files.iter().cloned().collect();
        // Leak is fine in tests
        let all_set = Box::leak(Box::new(all_set));
        ResolveCtx {
            all_file_paths: all_set,
            all_file_list: files,
            normalized_file_list: files,
            suffix_index,
            configs,
        }
    }

    #[test]
    fn test_relative_import() {
        let files = vec![
            "src/models/user.ts".to_string(),
            "src/models/types.ts".to_string(),
        ];
        let index = SuffixIndex::build(&files, &files);
        let configs = ImportConfigs::default();
        let ctx = make_ctx(&files, &index, &configs);

        match resolve("./types", "src/models/user.ts", &ctx) {
            ImportResult::Files(f) => assert_eq!(f, vec!["src/models/types.ts"]),
            other => panic!("Expected Files, got {:?}", other),
        }
    }

    #[test]
    fn test_path_alias() {
        let files = vec![
            "src/components/Button.tsx".to_string(),
        ];
        let index = SuffixIndex::build(&files, &files);
        let mut ts_paths = HashMap::new();
        ts_paths.insert("@/*".to_string(), vec!["src/*".to_string()]);
        let configs = ImportConfigs {
            ts_paths: Some(ts_paths),
            ts_base_url: None,
            ..Default::default()
        };
        let ctx = make_ctx(&files, &index, &configs);

        match resolve("@/components/Button", "src/pages/Home.tsx", &ctx) {
            ImportResult::Files(f) => assert_eq!(f, vec!["src/components/Button.tsx"]),
            other => panic!("Expected Files, got {:?}", other),
        }
    }

    #[test]
    fn test_unresolved() {
        let files = vec!["src/index.ts".to_string()];
        let index = SuffixIndex::build(&files, &files);
        let configs = ImportConfigs::default();
        let ctx = make_ctx(&files, &index, &configs);

        assert!(matches!(
            resolve("nonexistent-package", "src/index.ts", &ctx),
            ImportResult::Unresolved
        ));
    }
}

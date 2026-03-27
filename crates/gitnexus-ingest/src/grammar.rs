use gitnexus_core::config::languages::SupportedLanguage;
use tree_sitter::Language;

/// Get the tree-sitter Language for a supported language.
pub fn get_language(lang: SupportedLanguage) -> Language {
    match lang {
        SupportedLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        SupportedLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        SupportedLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SupportedLanguage::Java => tree_sitter_java::LANGUAGE.into(),
        SupportedLanguage::C => tree_sitter_c::LANGUAGE.into(),
        SupportedLanguage::CPlusPlus => tree_sitter_cpp::LANGUAGE.into(),
        SupportedLanguage::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        SupportedLanguage::Go => tree_sitter_go::LANGUAGE.into(),
        SupportedLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SupportedLanguage::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        SupportedLanguage::Ruby => tree_sitter_ruby::LANGUAGE.into(),

        #[cfg(feature = "kotlin")]
        SupportedLanguage::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
        #[cfg(not(feature = "kotlin"))]
        SupportedLanguage::Kotlin => tree_sitter_c::LANGUAGE.into(),

        #[cfg(feature = "swift")]
        SupportedLanguage::Swift => tree_sitter_swift::LANGUAGE.into(),
        #[cfg(not(feature = "swift"))]
        SupportedLanguage::Swift => tree_sitter_c::LANGUAGE.into(),

        // Razor files (.cshtml / .razor) reuse the C# grammar.
        // Razor-specific directives are extracted via regex preprocessing.
        SupportedLanguage::Razor => tree_sitter_c_sharp::LANGUAGE.into(),
    }
}

/// Check if a language has a real grammar available (not a fallback).
pub fn is_language_available(lang: SupportedLanguage) -> bool {
    match lang {
        #[cfg(not(feature = "kotlin"))]
        SupportedLanguage::Kotlin => false,
        #[cfg(not(feature = "swift"))]
        SupportedLanguage::Swift => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_all_available() {
        for lang in SupportedLanguage::all() {
            let ts_lang = get_language(*lang);
            // All languages should have at least some node kinds
            assert!(ts_lang.node_kind_count() > 0, "Language {:?} has no node kinds", lang);
        }
    }

    #[test]
    fn test_is_language_available() {
        assert!(is_language_available(SupportedLanguage::JavaScript));
        assert!(is_language_available(SupportedLanguage::TypeScript));
        assert!(is_language_available(SupportedLanguage::Python));
        assert!(is_language_available(SupportedLanguage::Java));
        assert!(is_language_available(SupportedLanguage::Rust));

        #[cfg(feature = "kotlin")]
        assert!(is_language_available(SupportedLanguage::Kotlin));
        #[cfg(not(feature = "kotlin"))]
        assert!(!is_language_available(SupportedLanguage::Kotlin));

        #[cfg(feature = "swift")]
        assert!(is_language_available(SupportedLanguage::Swift));
        #[cfg(not(feature = "swift"))]
        assert!(!is_language_available(SupportedLanguage::Swift));
    }

    #[test]
    fn test_parser_can_use_language() {
        let lang = get_language(SupportedLanguage::JavaScript);
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).expect("Failed to set JavaScript language");

        let tree = parser.parse("function hello() {}", None).unwrap();
        assert!(!tree.root_node().has_error());
    }
}

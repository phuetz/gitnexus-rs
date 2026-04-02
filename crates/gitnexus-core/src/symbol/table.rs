use std::collections::HashMap;

use super::definition::SymbolDefinition;

/// Symbol table with multiple indexes for fast lookup during resolution.
///
/// Mirrors the TypeScript SymbolTable with 4 indexes:
/// 1. File index: file_path → name → Vec<SymbolDefinition>
/// 2. Global index: name → Vec<SymbolDefinition>
/// 3. Callable index (lazy): name → Vec<SymbolDefinition> (only Function/Method/Constructor)
/// 4. Field-by-owner: "ownerNodeId\0fieldName" → SymbolDefinition
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// file_path → (name → Vec<SymbolDefinition>)
    file_index: HashMap<String, HashMap<String, Vec<SymbolDefinition>>>,
    /// name → Vec<SymbolDefinition> (all files)
    global_index: HashMap<String, Vec<SymbolDefinition>>,
    /// "ownerNodeId\0fieldName" → SymbolDefinition
    field_by_owner: HashMap<String, SymbolDefinition>,
    /// Callable-only index (lazy, built on first access)
    callable_index: Option<HashMap<String, Vec<SymbolDefinition>>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a symbol definition.
    pub fn add(&mut self, name: String, def: SymbolDefinition) {
        // Invalidate callable index cache
        self.callable_index = None;

        // File index
        self.file_index
            .entry(def.file_path.clone())
            .or_default()
            .entry(name.clone())
            .or_default()
            .push(def.clone());

        // Global index
        self.global_index
            .entry(name)
            .or_default()
            .push(def);
    }

    /// Register a field/property owned by a class/struct.
    pub fn add_field(&mut self, owner_id: &str, field_name: &str, def: SymbolDefinition) {
        let key = format!("{owner_id}\0{field_name}");
        self.field_by_owner.insert(key, def);
    }

    /// Exact lookup in a specific file: O(1).
    pub fn lookup_in_file(&self, file_path: &str, name: &str) -> Option<&[SymbolDefinition]> {
        self.file_index
            .get(file_path)
            .and_then(|names| names.get(name))
            .map(|v| v.as_slice())
    }

    /// Global fuzzy lookup by name: O(1) in the global index.
    pub fn lookup_global(&self, name: &str) -> Option<&[SymbolDefinition]> {
        self.global_index.get(name).map(|v| v.as_slice())
    }

    /// Lookup a field by owner and name.
    pub fn lookup_field(&self, owner_id: &str, field_name: &str) -> Option<&SymbolDefinition> {
        let key = format!("{owner_id}\0{field_name}");
        self.field_by_owner.get(&key)
    }

    /// Get or build the callable-only index (Function, Method, Constructor).
    pub fn callable_index(&mut self) -> &HashMap<String, Vec<SymbolDefinition>> {
        if self.callable_index.is_none() {
            let mut index: HashMap<String, Vec<SymbolDefinition>> = HashMap::new();
            for (name, defs) in &self.global_index {
                let callables: Vec<SymbolDefinition> = defs
                    .iter()
                    .filter(|d| d.symbol_type.is_callable())
                    .cloned()
                    .collect();
                if !callables.is_empty() {
                    index.insert(name.clone(), callables);
                }
            }
            self.callable_index = Some(index);
        }
        // Safety: callable_index is always set above when None
        self.callable_index.as_ref().expect("callable_index populated above")
    }

    /// Total symbol count.
    pub fn len(&self) -> usize {
        self.global_index.values().map(|v| v.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.global_index.is_empty()
    }

    /// All names registered in the file index for a file.
    pub fn names_in_file(&self, file_path: &str) -> Option<Vec<&str>> {
        self.file_index
            .get(file_path)
            .map(|names| names.keys().map(|s| s.as_str()).collect())
    }
}

use crate::graph::types::NodeLabel;

impl NodeLabel {
    /// Whether this label represents a callable symbol (for the callable index).
    pub fn is_callable(&self) -> bool {
        matches!(
            self,
            NodeLabel::Function | NodeLabel::Method | NodeLabel::Constructor
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::NodeLabel;

    fn make_def(node_id: &str, file_path: &str, symbol_type: NodeLabel) -> SymbolDefinition {
        SymbolDefinition {
            node_id: node_id.to_string(),
            file_path: file_path.to_string(),
            symbol_type,
            parameter_count: None,
            required_parameter_count: None,
            parameter_types: None,
            return_type: None,
            declared_type: None,
            owner_id: None,
            is_exported: true,
        }
    }

    #[test]
    fn test_add_and_lookup() {
        let mut table = SymbolTable::new();
        table.add(
            "handleLogin".to_string(),
            make_def("Function:a:handleLogin", "a.ts", NodeLabel::Function),
        );

        let results = table.lookup_in_file("a.ts", "handleLogin").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, "Function:a:handleLogin");

        let global = table.lookup_global("handleLogin").unwrap();
        assert_eq!(global.len(), 1);
    }

    #[test]
    fn test_multiple_definitions() {
        let mut table = SymbolTable::new();
        table.add("User".to_string(), make_def("Class:a:User", "a.ts", NodeLabel::Class));
        table.add("User".to_string(), make_def("Class:b:User", "b.ts", NodeLabel::Class));

        let global = table.lookup_global("User").unwrap();
        assert_eq!(global.len(), 2);

        let file_a = table.lookup_in_file("a.ts", "User").unwrap();
        assert_eq!(file_a.len(), 1);
    }

    #[test]
    fn test_field_lookup() {
        let mut table = SymbolTable::new();
        let def = make_def("Property:User:name", "a.ts", NodeLabel::Property);
        table.add_field("Class:User", "name", def);

        let field = table.lookup_field("Class:User", "name").unwrap();
        assert_eq!(field.node_id, "Property:User:name");

        assert!(table.lookup_field("Class:User", "nonexistent").is_none());
    }

    #[test]
    fn test_callable_index() {
        let mut table = SymbolTable::new();
        table.add("foo".to_string(), make_def("f1", "a.ts", NodeLabel::Function));
        table.add("bar".to_string(), make_def("v1", "a.ts", NodeLabel::Variable));
        table.add("baz".to_string(), make_def("m1", "a.ts", NodeLabel::Method));

        let callables = table.callable_index();
        assert!(callables.contains_key("foo"));
        assert!(callables.contains_key("baz"));
        assert!(!callables.contains_key("bar")); // Variable is not callable
    }
}

//! ASP.NET MVC 5 / Web API attribute extraction from C# source code.
//!
//! Extracts routing attributes, HTTP method decorators, data annotations,
//! controller/action detection, DbContext/Entity patterns, and Area associations
//! from C# source text using regex-based parsing (no tree-sitter dependency).
//!
//! This module provides the semantic layer needed to promote generic `Class`/`Method`
//! graph nodes into richer ASP.NET-specific types (`Controller`, `ControllerAction`,
//! `ApiEndpoint`, `View`, `ViewModel`, `DbEntity`, `DbContext`, `Area`).

use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

// ─── Result Types ────────────────────────────────────────────────────────

/// Information extracted from a C# class that may be an ASP.NET controller.
#[derive(Debug, Clone)]
pub struct ControllerInfo {
    /// Class name (e.g., "ProductsController")
    pub class_name: String,
    /// Area name from [Area("...")] attribute, if any
    pub area_name: Option<String>,
    /// Route prefix from [Route("...")] or [RoutePrefix("...")] on the class
    pub route_prefix: Option<String>,
    /// Whether this is an API controller (inherits ApiController or has [ApiController])
    pub is_api_controller: bool,
    /// The [Authorize] attribute roles/policies, if any
    pub authorize: Option<String>,
    /// Actions discovered inside this controller
    pub actions: Vec<ActionInfo>,
    /// Custom base controller name if not one of the standard base classes
    /// (Controller, ApiController, AsyncController, ControllerBase, ODataController)
    pub base_controller: Option<String>,
}

/// Information about a single controller action method.
#[derive(Debug, Clone)]
pub struct ActionInfo {
    /// Method name
    pub name: String,
    /// HTTP method: GET, POST, PUT, DELETE, PATCH (default GET for MVC)
    pub http_method: String,
    /// Route template from [Route("...")] on the method, or inferred from convention
    pub route_template: Option<String>,
    /// Parameter type for model binding (e.g., "ProductViewModel")
    pub model_type: Option<String>,
    /// Return type (e.g., "ActionResult", "JsonResult", "IHttpActionResult")
    pub return_type: Option<String>,
    /// Whether the action has [Authorize]
    pub requires_auth: bool,
    /// Start line in source (1-indexed)
    pub start_line: Option<u32>,
    /// Filter/attribute names applied to this action (e.g., "GridAction", "ValidateAntiForgeryToken")
    pub filters: Vec<String>,
}

/// Information about an Entity Framework DbContext.
#[derive(Debug, Clone)]
pub struct DbContextInfo {
    /// Class name (e.g., "ApplicationDbContext")
    pub class_name: String,
    /// Connection string name from constructor or attribute
    pub connection_string_name: Option<String>,
    /// DbSet<T> properties (entity type name → property name)
    pub entity_sets: Vec<EntitySetInfo>,
}

/// A DbSet<T> property inside a DbContext.
#[derive(Debug, Clone)]
pub struct EntitySetInfo {
    /// The entity type (e.g., "Product")
    pub entity_type: String,
    /// The property name (e.g., "Products")
    pub property_name: String,
}

/// Information about an Entity Framework entity / model class.
#[derive(Debug, Clone)]
pub struct EntityInfo {
    /// Class name
    pub class_name: String,
    /// [Table("...")] attribute value, if any
    pub table_name: Option<String>,
    /// Data annotations on properties: property name → list of annotations
    pub property_annotations: HashMap<String, Vec<String>>,
    /// Navigation property names (references to other entities)
    pub navigation_properties: Vec<NavigationProperty>,
}

/// A navigation property on an EF entity.
#[derive(Debug, Clone)]
pub struct NavigationProperty {
    /// Property name (e.g., "Orders")
    pub name: String,
    /// Target entity type (e.g., "Order")
    pub target_type: String,
    /// Whether this is a collection navigation (ICollection<T>, List<T>, etc.)
    pub is_collection: bool,
}

/// Information extracted from a Razor view file.
#[derive(Debug, Clone)]
pub struct ViewInfo {
    /// File path
    pub file_path: String,
    /// @model directive type, if any
    pub model_type: Option<String>,
    /// @Layout directive, if any
    pub layout_path: Option<String>,
    /// Area name inferred from path (Areas/<name>/Views/...)
    pub area_name: Option<String>,
    /// Whether this is a partial view (filename starts with _)
    pub is_partial: bool,
}

// ─── Constants ───────────────────────────────────────────────────────────

/// Base classes that identify a class as an MVC controller.
const CONTROLLER_BASE_CLASSES: &[&str] = &[
    "Controller",
    "AsyncController",
    "ApiController",
    "ControllerBase",
    "ODataController",
];

/// Data annotation attributes commonly used on EF entities/ViewModels.
const DATA_ANNOTATIONS: &[&str] = &[
    "Required",
    "MaxLength",
    "MinLength",
    "StringLength",
    "Range",
    "RegularExpression",
    "Compare",
    "EmailAddress",
    "Phone",
    "Url",
    "CreditCard",
    "DataType",
    "Display",
    "DisplayName",
    "DisplayFormat",
    "Key",
    "ForeignKey",
    "InverseProperty",
    "NotMapped",
    "Column",
    "Table",
    "Index",
    "Timestamp",
    "ConcurrencyCheck",
    "DatabaseGenerated",
    "ScaffoldColumn",
];

/// HTTP method attributes in ASP.NET MVC / Web API.
const HTTP_ATTRIBUTES: &[(&str, &str)] = &[
    ("HttpGet", "GET"),
    ("HttpPost", "POST"),
    ("HttpPut", "PUT"),
    ("HttpDelete", "DELETE"),
    ("HttpPatch", "PATCH"),
    ("HttpHead", "HEAD"),
    ("HttpOptions", "OPTIONS"),
    ("AcceptVerbs", "MIXED"),
    ("GridAction", "GET"),
];

// ─── Controller Detection ────────────────────────────────────────────────

/// Detect if a C# source file contains ASP.NET controller(s) and extract their info.
pub fn extract_controllers(source: &str) -> Vec<ControllerInfo> {
    let mut controllers = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        // Look for class declarations
        if let Some(class_match) = find_class_declaration(&lines, i) {
            // Check if it inherits from a controller base class
            if is_controller_class(&class_match.base_classes) {
                // Determine custom base controller: if the base class is not
                // one of the standard ones, record it for inheritance tracking
                let base_controller = class_match.base_classes.iter().find(|b| {
                    // Must look like a controller (ends with "Controller") but not be standard
                    !CONTROLLER_BASE_CLASSES.iter().any(|cb| *b == *cb)
                        && (b.ends_with("Controller") || b.ends_with("ControllerBase"))
                }).cloned();

                let mut controller = ControllerInfo {
                    class_name: class_match.name.clone(),
                    area_name: extract_attribute_value(&class_match.attributes, "Area"),
                    route_prefix: extract_attribute_value(&class_match.attributes, "Route")
                        .or_else(|| extract_attribute_value(&class_match.attributes, "RoutePrefix")),
                    is_api_controller: class_match.base_classes.iter().any(|b| b == "ApiController")
                        || class_match.attributes.iter().any(|a| a.starts_with("ApiController")),
                    authorize: extract_attribute_value(&class_match.attributes, "Authorize"),
                    actions: Vec::new(),
                    base_controller,
                };

                // Extract actions from the class body
                if let Some(body_end) = class_match.body_end_line {
                    controller.actions = extract_actions(
                        &lines,
                        class_match.body_start_line,
                        body_end,
                        controller.is_api_controller,
                    );
                }

                controllers.push(controller);
            }

            i = class_match.body_end_line.unwrap_or(class_match.body_start_line) + 1;
        } else {
            i += 1;
        }
    }

    controllers
}

/// Detect DbContext classes in C# source code.
pub fn extract_db_contexts(source: &str) -> Vec<DbContextInfo> {
    let mut contexts = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        if let Some(class_match) = find_class_declaration(&lines, i) {
            if class_match.base_classes.iter().any(|b| {
                b == "DbContext" || b == "IdentityDbContext" || b == "ObjectContext"
            }) {
                let mut ctx = DbContextInfo {
                    class_name: class_match.name.clone(),
                    connection_string_name: None,
                    entity_sets: Vec::new(),
                };

                // Extract DbSet<T> properties
                if let Some(body_end) = class_match.body_end_line {
                    for j in class_match.body_start_line..=body_end {
                        if j < lines.len() {
                            if let Some(es) = extract_dbset(lines[j]) {
                                ctx.entity_sets.push(es);
                            }
                            // Look for connection string in constructor
                            if let Some(cs) = extract_connection_string(lines[j]) {
                                ctx.connection_string_name = Some(cs);
                            }
                        }
                    }
                }

                contexts.push(ctx);
            }

            i = class_match.body_end_line.unwrap_or(class_match.body_start_line) + 1;
        } else {
            i += 1;
        }
    }

    contexts
}

/// Detect entity classes (classes with [Table] attribute or DbSet<T> references).
pub fn extract_entities(source: &str, known_entity_types: &[String]) -> Vec<EntityInfo> {
    let mut entities = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        if let Some(class_match) = find_class_declaration(&lines, i) {
            let has_table_attr = class_match
                .attributes
                .iter()
                .any(|a| a.starts_with("Table"));
            let is_known_entity = known_entity_types.contains(&class_match.name);

            if has_table_attr || is_known_entity {
                let mut entity = EntityInfo {
                    class_name: class_match.name.clone(),
                    table_name: extract_attribute_value(&class_match.attributes, "Table"),
                    property_annotations: HashMap::new(),
                    navigation_properties: Vec::new(),
                };

                // Extract properties with annotations
                if let Some(body_end) = class_match.body_end_line {
                    extract_entity_properties(
                        &lines,
                        class_match.body_start_line,
                        body_end,
                        &mut entity,
                    );
                }

                entities.push(entity);
            }

            i = class_match.body_end_line.unwrap_or(class_match.body_start_line) + 1;
        } else {
            i += 1;
        }
    }

    entities
}

/// Extract view information from a Razor file path and its source content.
pub fn extract_view_info(file_path: &str, source: &str) -> ViewInfo {
    let path_lower = file_path.to_lowercase().replace('\\', "/");
    let filename = path_lower.rsplit('/').next().unwrap_or("");

    // Extract @model directive
    let model_type = source.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed.strip_prefix("@model ").map(|rest| rest.trim().to_string())
    });

    // Extract @{ Layout = "..."; } or @Layout = "..."
    let layout_path = source.lines().find_map(|line| {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find("Layout") {
            let after = trimmed.get(idx..)?;
            // Look for quoted string after Layout =
            if let Some(q1) = after.find('"') {
                let after_q1 = after.get(q1 + 1..)?;
                if let Some(q2) = after_q1.find('"') {
                    return Some(after_q1.get(..q2).unwrap_or_default().to_string());
                }
            }
        }
        None
    });

    // Infer area from path: Areas/<name>/Views/...
    let area_name = if let Some(areas_idx) = path_lower.find("/areas/") {
        file_path
            .get(areas_idx + 7..)
            .and_then(|after| after.split('/').next())
            .map(|s| s.to_string())
    } else if path_lower.starts_with("areas/") {
        file_path
            .get(6..)
            .and_then(|after| after.split('/').next())
            .map(|s| s.to_string())
    } else {
        None
    };

    let is_partial = filename.starts_with('_');

    ViewInfo {
        file_path: file_path.to_string(),
        model_type,
        layout_path,
        area_name,
        is_partial,
    }
}

// ─── Internal Helpers ────────────────────────────────────────────────────

/// Parsed class declaration info.
struct ClassMatch {
    name: String,
    base_classes: Vec<String>,
    attributes: Vec<String>,
    body_start_line: usize,
    body_end_line: Option<usize>,
}

/// Find a class declaration starting from line `start`.
fn find_class_declaration(lines: &[&str], start: usize) -> Option<ClassMatch> {
    // Collect attributes before the class
    let mut attributes = Vec::new();
    let mut _attr_start = start;

    // Look backwards from current position for attributes
    if start > 0 {
        let mut j = start;
        loop {
            if j == 0 {
                break;
            }
            j -= 1;
            let trimmed = lines[j].trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                attributes.push(
                    trimmed.get(1..trimmed.len() - 1).unwrap_or_default().to_string(),
                );
                _attr_start = j;
            } else if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            } else {
                break;
            }
        }
    }

    // Check current line for class declaration
    let line = lines[start].trim();

    // Match pattern: [public|internal|...] [abstract|sealed|partial|static]* class ClassName [: BaseClass, IInterface]
    let class_keyword_idx = line.find(" class ")?;
    let after_class = line.get(class_keyword_idx + 7..)?;

    // Extract class name (before any : or { or < or where)
    let name_end = after_class
        .find([':', '{', '<', ' '])
        .unwrap_or(after_class.len());
    let name = after_class.get(..name_end).unwrap_or_default().trim().to_string();

    if name.is_empty() {
        return None;
    }

    // Extract base classes (after :)
    let mut base_classes = Vec::new();
    if let Some(colon_idx) = after_class.find(':') {
        let bases_str = after_class.get(colon_idx + 1..).unwrap_or_default();
        let bases_end = bases_str
            .find(['{', '\n'])
            .unwrap_or(bases_str.len());
        for base in bases_str.get(..bases_end).unwrap_or_default().split(',') {
            let base_name = base.trim();
            // Handle generic base: Controller<T> → Controller
            let clean = if let Some(lt) = base_name.find('<') {
                base_name.get(..lt).unwrap_or_default()
            } else {
                base_name
            };
            if !clean.is_empty() {
                // Strip "where" constraints
                if clean.starts_with("where ") {
                    break;
                }
                base_classes.push(clean.trim().to_string());
            }
        }
    }

    // Also check if line itself has attributes
    if line.starts_with('[') {
        if let Some(attr_end) = line.find(']') {
            attributes.push(line.get(1..attr_end).unwrap_or_default().to_string());
        }
    }

    // Find body bounds (brace matching)
    let (body_start, body_end) = find_brace_bounds(lines, start);

    Some(ClassMatch {
        name,
        base_classes,
        attributes,
        body_start_line: body_start,
        body_end_line: body_end,
    })
}

/// Find matching braces for a class/method body starting from `start_line`.
fn find_brace_bounds(lines: &[&str], start_line: usize) -> (usize, Option<usize>) {
    let mut depth = 0;
    let mut found_first = false;
    let mut body_start = start_line;

    for (i, &line) in lines.iter().enumerate().skip(start_line) {
        for ch in line.chars() {
            if ch == '{' {
                if !found_first {
                    found_first = true;
                    body_start = i;
                }
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 && found_first {
                    return (body_start, Some(i));
                }
            }
        }
    }

    (body_start, None)
}

/// Check if any base class is a known controller base.
fn is_controller_class(base_classes: &[String]) -> bool {
    base_classes.iter().any(|b| {
        CONTROLLER_BASE_CLASSES.iter().any(|cb| b == *cb || b.ends_with(cb))
    })
}

/// Extract a simple attribute value: [Attr("value")] → Some("value")
fn extract_attribute_value(attributes: &[String], attr_name: &str) -> Option<String> {
    for attr in attributes {
        let trimmed = attr.trim();
        if trimmed.starts_with(attr_name) {
            // Check for parenthesized value
            if let Some(paren_start) = trimmed.find('(') {
                let inner = trimmed.get(paren_start + 1..).unwrap_or_default();
                if let Some(paren_end) = inner.find(')') {
                    let value = inner.get(..paren_end).unwrap_or_default();
                    // Strip quotes
                    let clean = value.trim().trim_matches('"').trim_matches('\'');
                    return Some(clean.to_string());
                }
            }
            // Attribute without value: [Authorize] → Some("")
            return Some(String::new());
        }
    }
    None
}

/// Extract action methods from a controller body.
fn extract_actions(
    lines: &[&str],
    body_start: usize,
    body_end: usize,
    is_api: bool,
) -> Vec<ActionInfo> {
    let mut actions = Vec::new();
    let mut i = body_start;

    while i <= body_end && i < lines.len() {
        let trimmed = lines[i].trim();

        // Collect attributes for this method
        let mut method_attrs = Vec::new();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            method_attrs.push(
                trimmed.get(1..trimmed.len() - 1).unwrap_or_default().to_string(),
            );
            i += 1;
            // Collect multiple attribute lines
            while i <= body_end && i < lines.len() {
                let next = lines[i].trim();
                if next.starts_with('[') && next.ends_with(']') {
                    method_attrs.push(
                        next.get(1..next.len() - 1).unwrap_or_default().to_string(),
                    );
                    i += 1;
                } else {
                    break;
                }
            }
        }

        // Check if this line is a method declaration
        if i <= body_end && i < lines.len() {
            let line = lines[i].trim();
            if let Some(action) = parse_action_method(line, &method_attrs, is_api, i as u32 + 1) {
                actions.push(action);
            }
        }

        i += 1;
    }

    actions
}

/// Parse a single method declaration line into an ActionInfo.
fn parse_action_method(
    line: &str,
    attributes: &[String],
    is_api: bool,
    start_line: u32,
) -> Option<ActionInfo> {
    // Method pattern: public [virtual|override|async] ReturnType MethodName(params)
    if !line.contains('(') || !line.starts_with("public ") {
        return None;
    }

    // Skip non-action things like constructors, properties
    if line.contains(" class ") || line.contains(" get;") || line.contains(" set;") {
        return None;
    }

    // Extract return type and method name
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    // Find method name (the part before the first '(')
    let before_paren = line.split('(').next()?;
    let method_name = before_paren.split_whitespace().last()?.to_string();

    // Skip constructor (same name as class typically)
    if method_name.starts_with(|c: char| c.is_lowercase()) && !method_name.contains("_") {
        // lowercase-starting methods are fine, just check it's not weird
    }

    // Extract return type (word before method name)
    let return_type = {
        let words: Vec<&str> = before_paren.split_whitespace().collect();
        if words.len() >= 2 {
            let rt = words[words.len() - 2];
            // Skip modifiers
            if ["async", "virtual", "override", "static", "new", "sealed"].contains(&rt) {
                if words.len() >= 3 {
                    Some(words[words.len() - 3].to_string())
                } else {
                    None
                }
            } else {
                Some(rt.to_string())
            }
        } else {
            None
        }
    };

    // Check if return type looks like an action result
    let is_action_method = return_type.as_deref().is_some_and(|rt| {
        rt.contains("Result")
            || rt.contains("Response")
            || rt == "void"
            || rt.starts_with("Task")
            || rt.starts_with("IHttpActionResult")
            || rt.starts_with("IActionResult")
            || rt.starts_with("Json")
            || rt.starts_with("View")
    }) || attributes.iter().any(|a| {
        a.starts_with("Http") || a.starts_with("Route") || a.starts_with("Action") || a.starts_with("GridAction")
    });

    if !is_action_method {
        return None;
    }

    // Extract HTTP method from attributes
    let http_method = extract_http_method(attributes, is_api);

    // Extract route template
    let route_template = attributes.iter().find_map(|attr| {
        if attr.starts_with("Route(") || attr.starts_with("Http") {
            extract_attribute_value(std::slice::from_ref(attr), "Route")
                .or_else(|| {
                    // [HttpGet("path")] → extract path
                    for (http_attr, _) in HTTP_ATTRIBUTES {
                        if attr.starts_with(http_attr) {
                            if let Some(v) = extract_attribute_value(std::slice::from_ref(attr), http_attr) {
                                if !v.is_empty() {
                                    return Some(v);
                                }
                            }
                        }
                    }
                    None
                })
        } else {
            None
        }
    });

    // Extract model type from parameters
    let model_type = extract_model_type_from_params(line);

    // Check for [Authorize]
    let requires_auth = attributes.iter().any(|a| a.starts_with("Authorize"));

    // Collect filter/attribute names (anything ending with Attribute, Filter, or Action,
    // plus standard filter names like Authorize, ValidateAntiForgeryToken, etc.)
    let filters = extract_action_filters(attributes);

    Some(ActionInfo {
        name: method_name,
        http_method,
        route_template,
        model_type,
        return_type,
        requires_auth,
        start_line: Some(start_line),
        filters,
    })
}

/// Standard filter attribute names recognized at high confidence.
const STANDARD_FILTERS: &[&str] = &[
    "Authorize",
    "ValidateAntiForgeryToken",
    "OutputCache",
    "HandleError",
    "AllowAnonymous",
    "RequireHttps",
    "ActionFilter",
    "ExceptionFilter",
    "ResultFilter",
];

/// Extract filter/attribute names from action method attributes.
///
/// Recognizes standard ASP.NET filters as well as custom attributes that follow
/// the naming conventions `*Attribute`, `*Filter`, or `*Action` (e.g.,
/// `[GridAction]`, `[AuthorizeADAttribute]`, `[VerifActionFilter]`).
fn extract_action_filters(attributes: &[String]) -> Vec<String> {
    let mut filters = Vec::new();
    for attr in attributes {
        let attr_name = attr.split('(').next().unwrap_or(attr).trim();
        // Skip HTTP method attributes and Route attributes (those are not filters)
        if attr_name.starts_with("Http")
            || attr_name == "Route"
            || attr_name == "RoutePrefix"
            || attr_name == "Area"
            || attr_name == "ApiController"
            || attr_name == "NonAction"
        {
            continue;
        }
        // Include standard filters
        if STANDARD_FILTERS.contains(&attr_name) {
            filters.push(attr_name.to_string());
            continue;
        }
        // Include custom attributes ending with Attribute, Filter, or Action
        if attr_name.ends_with("Attribute")
            || attr_name.ends_with("Filter")
            || attr_name.ends_with("Action")
        {
            filters.push(attr_name.to_string());
        }
    }
    filters
}

/// Determine HTTP method from attributes.
fn extract_http_method(attributes: &[String], _is_api: bool) -> String {
    for attr in attributes {
        for (attr_name, method) in HTTP_ATTRIBUTES {
            if attr.starts_with(attr_name) {
                return method.to_string();
            }
        }
    }
    // Default: GET for both MVC and API
    "GET".to_string()
}

/// Extract model type from method parameters (look for complex types).
fn extract_model_type_from_params(line: &str) -> Option<String> {
    let paren_start = line.find('(')?;
    let paren_end = line.rfind(')')?;
    let params = line.get(paren_start + 1..paren_end)?;

    for param in params.split(',') {
        let parts: Vec<&str> = param.split_whitespace().collect();
        if parts.len() >= 2 {
            let type_name = parts[parts.len() - 2];
            // Skip primitive types and common framework types
            if !is_primitive_type(type_name) && !type_name.starts_with('[') {
                // Looks like a model type
                let clean = type_name.trim_start_matches('[').split('<').next()?;
                if clean.chars().next()?.is_uppercase() {
                    return Some(clean.to_string());
                }
            }
        }
    }
    None
}

/// Check if a type name is a primitive or common simple type.
fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "int" | "long" | "string" | "bool" | "float" | "double" | "decimal"
            | "byte" | "char" | "short" | "uint" | "ulong" | "ushort"
            | "DateTime" | "Guid" | "int?" | "long?" | "bool?" | "Nullable"
            | "CancellationToken" | "FormCollection" | "HttpPostedFileBase"
    )
}

/// Extract DbSet<T> from a line.
fn extract_dbset(line: &str) -> Option<EntitySetInfo> {
    let trimmed = line.trim();
    // Pattern: public DbSet<EntityType> PropertyName { get; set; }
    // or: public virtual DbSet<EntityType> PropertyName { get; set; }
    // or: public IDbSet<EntityType> PropertyName { get; set; }
    let dbset_markers = ["DbSet<", "IDbSet<", "ObjectSet<"];
    for marker in dbset_markers {
        if let Some(idx) = trimmed.find(marker) {
            let after = trimmed.get(idx + marker.len()..)?;
            let type_end = after.find('>')?;
            let entity_type = after.get(..type_end).unwrap_or_default().trim().to_string();

            // Property name is after > and before {
            let after_type = after.get(type_end + 1..)?;
            let prop_name = after_type
                .split('{')
                .next()?
                .trim()
                .to_string();

            if !entity_type.is_empty() && !prop_name.is_empty() {
                return Some(EntitySetInfo {
                    entity_type,
                    property_name: prop_name,
                });
            }
        }
    }
    None
}

/// Extract connection string name from a constructor line.
fn extract_connection_string(line: &str) -> Option<String> {
    let trimmed = line.trim();
    // Pattern: : base("connectionStringName") or "name=ConnectionStringName"
    if trimmed.contains("base(") || trimmed.contains("nameOrConnectionString") {
        if let Some(q1) = trimmed.find('"') {
            let after_q1 = trimmed.get(q1 + 1..)?;
            if let Some(q2) = after_q1.find('"') {
                let value = after_q1.get(..q2).unwrap_or_default();
                // Handle "name=X" format
                if let Some(rest) = value.strip_prefix("name=") {
                    return Some(rest.to_string());
                }
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Extract entity properties and their data annotations.
fn extract_entity_properties(
    lines: &[&str],
    body_start: usize,
    body_end: usize,
    entity: &mut EntityInfo,
) {
    let mut pending_annotations: Vec<String> = Vec::new();
    let collection_types = [
        "ICollection<",
        "IList<",
        "List<",
        "IEnumerable<",
        "HashSet<",
        "Collection<",
    ];

    for line in &lines[body_start..=body_end.min(lines.len() - 1)] {
        let trimmed = line.trim();

        // Collect attributes
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let attr_content = trimmed.get(1..trimmed.len() - 1).unwrap_or_default();
            // May have multiple attributes: [Required, MaxLength(100)]
            for attr in attr_content.split(',') {
                let a = attr.trim().to_string();
                if DATA_ANNOTATIONS.iter().any(|da| a.starts_with(da)) {
                    pending_annotations.push(a);
                }
            }
            continue;
        }

        // Check for property declaration
        if trimmed.starts_with("public ") && (trimmed.contains("{ get;") || trimmed.contains("{ get ")) {
            // Extract property name
            let parts: Vec<&str> = trimmed.split('{').next().unwrap_or("").split_whitespace().collect();
            if parts.len() >= 3 {
                let prop_name = parts.last().unwrap_or(&"").to_string();
                let prop_type = parts[parts.len() - 2];

                // Store annotations
                if !pending_annotations.is_empty() {
                    entity
                        .property_annotations
                        .insert(prop_name.clone(), pending_annotations.clone());
                }

                // Check for navigation property
                let is_collection = collection_types.iter().any(|ct| prop_type.contains(ct));
                if is_collection {
                    // Extract target type from generic
                    if let Some(start) = prop_type.find('<') {
                        if let Some(end) = prop_type.find('>') {
                            let target = prop_type.get(start + 1..end).unwrap_or_default().trim().to_string();
                            entity.navigation_properties.push(NavigationProperty {
                                name: prop_name,
                                target_type: target,
                                is_collection: true,
                            });
                        }
                    }
                } else if prop_type.starts_with("virtual ") || trimmed.contains("virtual ") {
                    // Single navigation: public virtual Order Order { get; set; }
                    let clean_type = prop_type.replace("virtual ", "");
                    if clean_type.chars().next().is_some_and(|c| c.is_uppercase())
                        && !is_primitive_type(&clean_type)
                    {
                        entity.navigation_properties.push(NavigationProperty {
                            name: prop_name,
                            target_type: clean_type,
                            is_collection: false,
                        });
                    }
                }
            }

            pending_annotations.clear();
        } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
            pending_annotations.clear();
        }
    }
}

// ─── AJAX Call Extraction ───────────────────────────────────────────────

/// AJAX call targeting a controller action.
#[derive(Debug, Clone)]
pub struct AjaxCallInfo {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Controller name extracted from the URL, if any
    pub controller_name: Option<String>,
    /// Action name extracted from the URL, if any
    pub action_name: Option<String>,
    /// The raw URL pattern matched
    pub url_pattern: String,
    /// Line number where the call was found (1-indexed)
    pub line_number: u32,
}

// Compiled regexes for AJAX patterns.

/// $.ajax({...type: "POST"...url: '/Controller/Action'...}) or similar
static RE_AJAX_CALL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\$\.\s*ajax\s*\("#).unwrap()
});

/// type/method inside $.ajax options: type: "POST" or method: "GET"
static RE_AJAX_TYPE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:type|method)\s*:\s*["'](\w+)["']"#).unwrap()
});

/// url inside $.ajax options: url: '/Controller/Action' or url: "/Controller/Action"
static RE_AJAX_URL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"url\s*:\s*["']([^"']+)["']"#).unwrap()
});

/// $.post('/Controller/Action', ...)
static RE_JQUERY_POST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\$\.\s*post\s*\(\s*["']([^"']+)["']"#).unwrap()
});

/// $.get('/Controller/Action', ...)
static RE_JQUERY_GET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\$\.\s*get\s*\(\s*["']([^"']+)["']"#).unwrap()
});

/// @Url.Action("Action", "Controller")
static RE_URL_ACTION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"@?Url\.Action\s*\(\s*"(\w+)"\s*,\s*"(\w+)""#).unwrap()
});

/// fetch('/Controller/Action') or fetch("/Controller/Action")
static RE_FETCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"fetch\s*\(\s*["']([^"']+)["']"#).unwrap()
});

/// $.getJSON('/Controller/Action', ...)
static RE_GETJSON: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\$\.getJSON\s*\(\s*['"]/?(\w+)/(\w+)['"]"#)
        .expect("RE_GETJSON regex must compile")
});

/// $(...).load('/Controller/Action', ...)
static RE_LOAD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\$\([^)]+\)\.load\s*\(\s*['"]/?(\w+)/(\w+)['"]"#)
        .expect("RE_LOAD regex must compile")
});

/// Helper: split a URL like "/Controller/Action" or "/api/Controller/Action" into
/// (Option<controller>, Option<action>).
fn parse_url_segments(url: &str) -> (Option<String>, Option<String>) {
    let trimmed = url.trim_start_matches('/');
    let segments: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();

    match segments.len() {
        0 => (None, None),
        1 => (Some(segments[0].to_string()), None),
        2 => (Some(segments[0].to_string()), Some(segments[1].to_string())),
        _ => {
            // Skip leading segments like "api" — take last two meaningful segments
            // e.g. /api/Products/GetAll → controller=Products, action=GetAll
            let controller = segments[segments.len() - 2].to_string();
            let action = segments[segments.len() - 1].to_string();
            (Some(controller), Some(action))
        }
    }
}

/// Extract AJAX / fetch calls from C# / Razor / JavaScript source that target controller actions.
pub fn extract_ajax_calls(source: &str) -> Vec<AjaxCallInfo> {
    let mut results = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let line_number = (line_idx + 1) as u32;

        // --- $.ajax({...}) ---
        if RE_AJAX_CALL.is_match(line) {
            // Gather context: look ahead up to 15 lines for url and type within the ajax block
            let context = gather_context(source, line_idx, 15);

            let method = RE_AJAX_TYPE
                .captures(&context)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_else(|| "GET".to_string());

            if let Some(url_cap) = RE_AJAX_URL.captures(&context) {
                let url = url_cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let (controller, action) = parse_url_segments(url);
                results.push(AjaxCallInfo {
                    method,
                    controller_name: controller,
                    action_name: action,
                    url_pattern: url.to_string(),
                    line_number,
                });
            }
            continue;
        }

        // --- $.post(...) ---
        if let Some(cap) = RE_JQUERY_POST.captures(line) {
            let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let (controller, action) = parse_url_segments(url);
            results.push(AjaxCallInfo {
                method: "POST".to_string(),
                controller_name: controller,
                action_name: action,
                url_pattern: url.to_string(),
                line_number,
            });
            continue;
        }

        // --- $.get(...) ---
        if let Some(cap) = RE_JQUERY_GET.captures(line) {
            let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let (controller, action) = parse_url_segments(url);
            results.push(AjaxCallInfo {
                method: "GET".to_string(),
                controller_name: controller,
                action_name: action,
                url_pattern: url.to_string(),
                line_number,
            });
            continue;
        }

        // --- @Url.Action("Action", "Controller") ---
        if let Some(cap) = RE_URL_ACTION.captures(line) {
            let action_name = cap.get(1).map(|m| m.as_str().to_string());
            let controller_name = cap.get(2).map(|m| m.as_str().to_string());
            let url = format!(
                "/{}/{}",
                controller_name.as_deref().unwrap_or(""),
                action_name.as_deref().unwrap_or("")
            );
            results.push(AjaxCallInfo {
                method: "GET".to_string(),
                controller_name,
                action_name,
                url_pattern: url,
                line_number,
            });
            continue;
        }

        // --- fetch('/Controller/Action') ---
        if let Some(cap) = RE_FETCH.captures(line) {
            let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let (controller, action) = parse_url_segments(url);
            results.push(AjaxCallInfo {
                method: "GET".to_string(),
                controller_name: controller,
                action_name: action,
                url_pattern: url.to_string(),
                line_number,
            });
            continue;
        }

        // --- $.getJSON('/Controller/Action', ...) ---
        if let Some(cap) = RE_GETJSON.captures(line) {
            let controller = cap.get(1).map(|m| m.as_str().to_string());
            let action = cap.get(2).map(|m| m.as_str().to_string());
            let url = format!(
                "/{}/{}",
                controller.as_deref().unwrap_or(""),
                action.as_deref().unwrap_or("")
            );
            results.push(AjaxCallInfo {
                method: "GET".to_string(),
                controller_name: controller,
                action_name: action,
                url_pattern: url,
                line_number,
            });
            continue;
        }

        // --- $(...).load('/Controller/Action', ...) ---
        if let Some(cap) = RE_LOAD.captures(line) {
            let controller = cap.get(1).map(|m| m.as_str().to_string());
            let action = cap.get(2).map(|m| m.as_str().to_string());
            let url = format!(
                "/{}/{}",
                controller.as_deref().unwrap_or(""),
                action.as_deref().unwrap_or("")
            );
            results.push(AjaxCallInfo {
                method: "GET".to_string(),
                controller_name: controller,
                action_name: action,
                url_pattern: url,
                line_number,
            });
        }
    }

    results
}

/// Gather a context window of lines starting at `start` for up to `lookahead` additional lines.
fn gather_context(source: &str, start: usize, lookahead: usize) -> String {
    source
        .lines()
        .skip(start)
        .take(lookahead + 1)
        .collect::<Vec<&str>>()
        .join("\n")
}

// ─── Telerik / Kendo Component Extraction ───────────────────────────────

/// An action in a DataSource transport configuration.
#[derive(Debug, Clone)]
pub struct DataSourceAction {
    /// CRUD operation: "Read", "Create", "Update", "Destroy"
    pub operation: String,
    /// Controller name
    pub controller_name: String,
    /// Action name
    pub action_name: String,
}

/// Telerik or Kendo UI component extracted from a Razor view.
#[derive(Debug, Clone)]
pub struct TelerikComponentInfo {
    /// Component type (e.g., "Grid", "ComboBox", "DropDownList")
    pub component_type: String,
    /// Vendor identifier: "Kendo" or "Telerik"
    pub vendor: String,
    /// Generic model type, if any (e.g., "ProductViewModel")
    pub model_type: Option<String>,
    /// DataSource transport actions found nearby
    pub data_source_actions: Vec<DataSourceAction>,
    /// Client-side events: (event_name, js_function_name)
    pub client_events: Vec<(String, String)>,
    /// Line number where the component declaration starts (1-indexed)
    pub line_number: u32,
}

/// Html.Kendo().Grid<Model>() or Html.Kendo().ComboBox()
static RE_KENDO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Html\.\s*Kendo\s*\(\s*\)\s*\.\s*(\w+)(?:<(\w+)>)?"#).unwrap()
});

/// Html.Telerik().Grid() — older Telerik MVC Extensions syntax
static RE_TELERIK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Html\.\s*Telerik\s*\(\s*\)\s*\.\s*(\w+)(?:<(\w+)>)?"#).unwrap()
});

/// DataSource action: .Read(.Action("GetAll", "Products")) or .Create(... etc.
static RE_DS_ACTION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\.\s*(Read|Create|Update|Destroy)\s*\(.*?\.Action\s*\(\s*"(\w+)"\s*,\s*"(\w+)""#)
        .unwrap()
});

// Legacy Telerik Extensions syntax: .Select("Action", "Controller"), .Insert(...), .Update(...), .Delete(...)
static RE_DS_LEGACY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\.\s*(Select|Insert|Update|Delete)\s*\(\s*"(\w+)"\s*,\s*"(\w+)""#)
        .unwrap()
});

/// Client events: .Events(e => e.OnChange("onGridChange")) or .On("change", "handler")
static RE_CLIENT_EVENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\.On(\w+)\s*\(\s*"(\w+)""#).unwrap()
});

/// jQuery Kendo widget initialization: .kendoGrid(, .kendoComboBox( etc.
static RE_KENDO_JQUERY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\.\s*kendo(\w+)\s*\("#).unwrap()
});

/// Extract Telerik / Kendo UI component declarations from Razor or JavaScript source.
pub fn extract_telerik_components(source: &str) -> Vec<TelerikComponentInfo> {
    let mut results = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, &line) in lines.iter().enumerate() {
        let line_number = (line_idx + 1) as u32;

        // --- Html.Kendo().Widget<T>() ---
        if let Some(cap) = RE_KENDO.captures(line) {
            let component_type = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let model_type = cap.get(2).map(|m| m.as_str().to_string());
            let (ds_actions, events) = scan_component_body(&lines, line_idx, 50);

            results.push(TelerikComponentInfo {
                component_type,
                vendor: "Kendo".to_string(),
                model_type,
                data_source_actions: ds_actions,
                client_events: events,
                line_number,
            });
            continue;
        }

        // --- Html.Telerik().Widget() ---
        if let Some(cap) = RE_TELERIK.captures(line) {
            let component_type = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let model_type = cap.get(2).map(|m| m.as_str().to_string());
            let (ds_actions, events) = scan_component_body(&lines, line_idx, 50);

            results.push(TelerikComponentInfo {
                component_type,
                vendor: "Telerik".to_string(),
                model_type,
                data_source_actions: ds_actions,
                client_events: events,
                line_number,
            });
            continue;
        }

        // --- jQuery: $(...).kendoGrid({ ... }) ---
        if let Some(cap) = RE_KENDO_JQUERY.captures(line) {
            let component_type = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let (ds_actions, events) = scan_component_body(&lines, line_idx, 50);

            results.push(TelerikComponentInfo {
                component_type,
                vendor: "Kendo".to_string(),
                model_type: None,
                data_source_actions: ds_actions,
                client_events: events,
                line_number,
            });
        }
    }

    results
}

/// Scan up to `lookahead` lines after a component declaration for DataSource actions and events.
fn scan_component_body(
    lines: &[&str],
    start: usize,
    lookahead: usize,
) -> (Vec<DataSourceAction>, Vec<(String, String)>) {
    let mut ds_actions = Vec::new();
    let mut events = Vec::new();
    let end = (start + lookahead).min(lines.len());

    for &line in &lines[start..end] {
        // Kendo syntax: .Read(read => read.Action("Action", "Controller"))
        if let Some(cap) = RE_DS_ACTION.captures(line) {
            let operation = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let action_name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let controller_name = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
            ds_actions.push(DataSourceAction {
                operation,
                controller_name,
                action_name,
            });
        }
        // Legacy Telerik syntax: .Select("Action", "Controller")
        else if let Some(cap) = RE_DS_LEGACY.captures(line) {
            let raw_op = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let operation = match raw_op {
                "Select" => "Read",
                "Insert" => "Create",
                "Delete" => "Destroy",
                other => other,
            }.to_string();
            let action_name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let controller_name = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
            ds_actions.push(DataSourceAction {
                operation,
                controller_name,
                action_name,
            });
        }

        if let Some(cap) = RE_CLIENT_EVENT.captures(line) {
            let event_name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let handler = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            events.push((event_name, handler));
        }
    }

    (ds_actions, events)
}

// ─── Service / Repository Extraction ────────────────────────────────────

/// A service or repository class detected via naming conventions and DI patterns.
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Class name (e.g., "ProductService", "OrderRepository")
    pub class_name: String,
    /// Detected layer type: "Service", "Repository", "Manager", "Provider", "UnitOfWork", "Factory", or "Facade"
    pub layer_type: String,
    /// Interface implemented (e.g., "IProductService")
    pub implements_interface: Option<String>,
    /// Constructor-injected dependencies: (interface_type, parameter_name)
    pub dependencies: Vec<(String, String)>,
}

/// Pattern: public class FooService : IFooService
/// Also matches names containing UnitOfWork (e.g., UnitOfWorkAide) since UnitOfWork
/// classes in legacy codebases often have domain-specific suffixes.
static RE_SERVICE_CLASS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"public\s+class\s+(\w*(?:Service|Repository|Manager|Provider|UnitOfWork|Factory|Facade)\w*)\s*:\s*(I\w+)"#,
    )
    .unwrap()
});

/// Constructor parameter matching an interface: ISomeService someService
static RE_CTOR_PARAM: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(I[A-Z]\w+)\s+(\w+)"#).unwrap()
});

/// Extract service / repository / manager / provider classes from C# source.
pub fn extract_services_and_repositories(source: &str) -> Vec<ServiceInfo> {
    let mut results = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, &line) in lines.iter().enumerate() {
        if let Some(cap) = RE_SERVICE_CLASS.captures(line) {
            let class_name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let interface_name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

            let layer_type = if class_name.ends_with("Repository") {
                "Repository"
            } else if class_name.ends_with("Service") {
                "Service"
            } else if class_name.ends_with("Manager") {
                "Manager"
            } else if class_name.contains("UnitOfWork") {
                "UnitOfWork"
            } else if class_name.ends_with("Factory") {
                "Factory"
            } else if class_name.ends_with("Facade") {
                "Facade"
            } else {
                "Provider"
            }
            .to_string();

            let dependencies = extract_constructor_dependencies(source, &class_name);

            results.push(ServiceInfo {
                class_name: class_name.clone(),
                layer_type,
                implements_interface: Some(interface_name),
                dependencies,
            });

            // Skip past the class body to avoid re-matching inner classes
            if let Some(body_end) = find_brace_bounds(&lines, line_idx).1 {
                // We can't mutate the iterator, but duplicates are prevented by the
                // regex requiring "public class" which won't match again inside the body
                let _ = body_end;
            }
        }
    }

    results
}

/// Extract constructor-injected dependencies for a specific class.
///
/// Finds `public ClassName(IFoo foo, IBar bar)` and returns `[(IFoo, foo), (IBar, bar)]`.
pub fn extract_constructor_dependencies(source: &str, class_name: &str) -> Vec<(String, String)> {
    // Build a regex for this specific constructor: public ClassName(...)
    let pattern = format!(r"public\s+{}\s*\(([^)]*)\)", regex::escape(class_name));
    let re = Regex::new(&pattern).unwrap();

    let mut deps = Vec::new();

    if let Some(cap) = re.captures(source) {
        let params = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        for param_cap in RE_CTOR_PARAM.captures_iter(params) {
            let iface = param_cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let name = param_cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            deps.push((iface, name));
        }
    }

    deps
}

// ─── Dependency Injection Registration Extraction ───────────────────────

/// A DI container registration extracted from C# source.
#[derive(Debug, Clone)]
pub struct DiRegistration {
    /// The concrete implementation type (e.g., "ProductService")
    pub implementation_type: String,
    /// The service/interface type (e.g., "IProductService")
    pub service_type: String,
    /// The DI framework: "Autofac", "Unity", "Ninject", "Microsoft"
    pub framework: String,
    /// Lifetime scope, if detected: "Singleton", "Transient", "Scoped", "PerRequest", etc.
    pub lifetime: Option<String>,
}

/// Autofac: builder.RegisterType<ProductService>().As<IProductService>()
static RE_AUTOFAC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"RegisterType<(\w+)>\s*\(\s*\)\s*\.As<(\w+)>"#)
        .expect("RE_AUTOFAC regex must compile")
});

/// Autofac lifetime: .SingleInstance(), .InstancePerRequest(), .InstancePerLifetimeScope(), .InstancePerDependency()
static RE_AUTOFAC_LIFETIME: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\.(SingleInstance|InstancePerRequest|InstancePerLifetimeScope|InstancePerDependency)\s*\("#)
        .expect("RE_AUTOFAC_LIFETIME regex must compile")
});

/// Unity: container.RegisterType<IProductService, ProductService>()
static RE_UNITY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"RegisterType<(\w+),\s*(\w+)>"#).unwrap()
});

/// Ninject: Bind<IProductService>().To<ProductService>()
static RE_NINJECT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Bind<(\w+)>\s*\(\s*\)\s*\.To<(\w+)>"#).unwrap()
});

/// MS DI: services.AddScoped<IProductService, ProductService>()
static RE_MS_DI: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:AddScoped|AddTransient|AddSingleton)<(\w+),\s*(\w+)>"#).unwrap()
});

/// MS DI lifetime from method name
static RE_MS_DI_LIFETIME: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(AddScoped|AddTransient|AddSingleton)<"#).unwrap()
});

/// Extract DI container registrations from C# source (Autofac, Unity, Ninject, MS DI).
pub fn extract_di_registrations(source: &str) -> Vec<DiRegistration> {
    let mut results = Vec::new();

    for line in source.lines() {
        // --- Autofac: RegisterType<Impl>().As<IService>() ---
        if let Some(cap) = RE_AUTOFAC.captures(line) {
            let impl_type = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let svc_type = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

            let lifetime = RE_AUTOFAC_LIFETIME.captures(line).map(|lc| {
                let raw = lc.get(1).map(|m| m.as_str()).unwrap_or_default();
                match raw {
                    "SingleInstance" => "Singleton".to_string(),
                    "InstancePerRequest" => "PerRequest".to_string(),
                    "InstancePerLifetimeScope" => "Scoped".to_string(),
                    "InstancePerDependency" => "Transient".to_string(),
                    other => other.to_string(),
                }
            });

            results.push(DiRegistration {
                implementation_type: impl_type,
                service_type: svc_type,
                framework: "Autofac".to_string(),
                lifetime,
            });
            continue;
        }

        // --- Unity: RegisterType<IService, Impl>() ---
        if let Some(cap) = RE_UNITY.captures(line) {
            let svc_type = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let impl_type = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            results.push(DiRegistration {
                implementation_type: impl_type,
                service_type: svc_type,
                framework: "Unity".to_string(),
                lifetime: None,
            });
            continue;
        }

        // --- Ninject: Bind<IService>().To<Impl>() ---
        if let Some(cap) = RE_NINJECT.captures(line) {
            let svc_type = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let impl_type = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            results.push(DiRegistration {
                implementation_type: impl_type,
                service_type: svc_type,
                framework: "Ninject".to_string(),
                lifetime: None,
            });
            continue;
        }

        // --- MS DI: AddScoped/AddTransient/AddSingleton<IService, Impl>() ---
        if let Some(cap) = RE_MS_DI.captures(line) {
            let svc_type = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let impl_type = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

            let lifetime = RE_MS_DI_LIFETIME.captures(line).map(|lc| {
                let raw = lc.get(1).map(|m| m.as_str()).unwrap_or_default();
                match raw {
                    "AddScoped" => "Scoped".to_string(),
                    "AddTransient" => "Transient".to_string(),
                    "AddSingleton" => "Singleton".to_string(),
                    other => other.to_string(),
                }
            });

            results.push(DiRegistration {
                implementation_type: impl_type,
                service_type: svc_type,
                framework: "Microsoft".to_string(),
                lifetime,
            });
        }
    }

    results
}

// ─── Razor Form Action Extraction ───────────────────────────────────────

/// Information about an Html.BeginForm() call in a Razor view.
#[derive(Debug, Clone)]
pub struct FormActionInfo {
    /// The action name (first argument)
    pub action_name: String,
    /// The controller name (second argument)
    pub controller_name: String,
    /// HTTP method from FormMethod enum (defaults to "POST")
    pub http_method: String,
    /// Line number where the form was found (1-indexed)
    pub line_number: u32,
}

/// Html.BeginForm("Action", "Controller", FormMethod.Post)
static RE_BEGIN_FORM: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"Html\.BeginForm\s*\(\s*"(\w+)"\s*,\s*"(\w+)"(?:\s*,\s*FormMethod\.(\w+))?"#)
        .expect("RE_BEGIN_FORM regex must compile")
});

/// Extract Html.BeginForm() calls from Razor view source.
pub fn extract_form_actions(source: &str) -> Vec<FormActionInfo> {
    let mut results = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        if let Some(cap) = RE_BEGIN_FORM.captures(line) {
            let action_name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let controller_name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let http_method = cap
                .get(3)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_else(|| "POST".to_string());

            results.push(FormActionInfo {
                action_name,
                controller_name,
                http_method,
                line_number: (line_idx + 1) as u32,
            });
        }
    }

    results
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_controllers_basic() {
        let source = r#"
using System.Web.Mvc;

[Authorize]
[RoutePrefix("products")]
public class ProductsController : Controller
{
    [HttpGet]
    [Route("")]
    public ActionResult Index()
    {
        return View();
    }

    [HttpPost]
    [Route("create")]
    public ActionResult Create(ProductViewModel model)
    {
        return RedirectToAction("Index");
    }

    [HttpGet]
    [Route("{id}")]
    public ActionResult Details(int id)
    {
        return View();
    }
}
"#;
        let controllers = extract_controllers(source);
        assert_eq!(controllers.len(), 1);

        let ctrl = &controllers[0];
        assert_eq!(ctrl.class_name, "ProductsController");
        assert_eq!(ctrl.route_prefix.as_deref(), Some("products"));
        assert!(!ctrl.is_api_controller);
        assert!(ctrl.authorize.is_some());
        assert!(ctrl.actions.len() >= 2);

        // Check first action
        let index = ctrl.actions.iter().find(|a| a.name == "Index");
        assert!(index.is_some());
        let index = index.unwrap();
        assert_eq!(index.http_method, "GET");
    }

    #[test]
    fn test_extract_api_controller() {
        let source = r#"
using System.Web.Http;

[RoutePrefix("api/orders")]
public class OrdersController : ApiController
{
    [HttpGet]
    [Route("")]
    public IHttpActionResult GetAll()
    {
        return Ok(orders);
    }

    [HttpPost]
    public IHttpActionResult Create(OrderDto dto)
    {
        return Created(dto);
    }
}
"#;
        let controllers = extract_controllers(source);
        assert_eq!(controllers.len(), 1);

        let ctrl = &controllers[0];
        assert!(ctrl.is_api_controller);
        assert_eq!(ctrl.route_prefix.as_deref(), Some("api/orders"));
    }

    #[test]
    fn test_extract_db_context() {
        let source = r#"
public class ApplicationDbContext : DbContext
{
    public ApplicationDbContext()
        : base("DefaultConnection")
    {
    }

    public DbSet<Product> Products { get; set; }
    public DbSet<Order> Orders { get; set; }
    public DbSet<Customer> Customers { get; set; }
}
"#;
        let contexts = extract_db_contexts(source);
        assert_eq!(contexts.len(), 1);

        let ctx = &contexts[0];
        assert_eq!(ctx.class_name, "ApplicationDbContext");
        assert_eq!(ctx.connection_string_name.as_deref(), Some("DefaultConnection"));
        assert_eq!(ctx.entity_sets.len(), 3);
        assert!(ctx.entity_sets.iter().any(|es| es.entity_type == "Product"));
    }

    #[test]
    fn test_extract_entities() {
        let source = r#"
[Table("Products")]
public class Product
{
    [Key]
    public int Id { get; set; }

    [Required]
    [MaxLength(200)]
    public string Name { get; set; }

    [Range(0, 99999)]
    public decimal Price { get; set; }

    public int CategoryId { get; set; }

    [ForeignKey("CategoryId")]
    public virtual Category Category { get; set; }

    public virtual ICollection<OrderItem> OrderItems { get; set; }
}
"#;
        let entities = extract_entities(source, &["Product".to_string()]);
        assert_eq!(entities.len(), 1);

        let entity = &entities[0];
        assert_eq!(entity.class_name, "Product");
        assert_eq!(entity.table_name.as_deref(), Some("Products"));
        assert!(!entity.navigation_properties.is_empty());

        // Check collection navigation
        let order_items = entity
            .navigation_properties
            .iter()
            .find(|np| np.name == "OrderItems");
        assert!(order_items.is_some());
        assert!(order_items.unwrap().is_collection);
        assert_eq!(order_items.unwrap().target_type, "OrderItem");
    }

    #[test]
    fn test_extract_view_info() {
        let info = extract_view_info(
            "Areas/Admin/Views/Products/Index.cshtml",
            "@model IEnumerable<MyApp.Models.Product>\n@{ Layout = \"~/Views/Shared/_Layout.cshtml\"; }",
        );
        assert_eq!(
            info.model_type.as_deref(),
            Some("IEnumerable<MyApp.Models.Product>")
        );
        assert_eq!(
            info.layout_path.as_deref(),
            Some("~/Views/Shared/_Layout.cshtml")
        );
        assert_eq!(info.area_name.as_deref(), Some("Admin"));
        assert!(!info.is_partial);
    }

    #[test]
    fn test_extract_dbset() {
        assert!(extract_dbset("public DbSet<Product> Products { get; set; }").is_some());
        assert!(extract_dbset("public virtual DbSet<Order> Orders { get; set; }").is_some());
        assert!(extract_dbset("public IDbSet<Customer> Customers { get; set; }").is_some());
        assert!(extract_dbset("public int Count { get; set; }").is_none());
    }

    #[test]
    fn test_extract_connection_string() {
        assert_eq!(
            extract_connection_string("        : base(\"DefaultConnection\")"),
            Some("DefaultConnection".to_string())
        );
        assert_eq!(
            extract_connection_string("        : base(\"name=MyDb\")"),
            Some("MyDb".to_string())
        );
    }

    // ─── AJAX extraction tests ──────────────────────────────────────────

    #[test]
    fn test_extract_ajax_calls_jquery() {
        let source = r#"
<script>
    $.ajax({
        url: '/Products/GetAll',
        type: 'GET',
        success: function(data) { }
    });
</script>
"#;
        let calls = extract_ajax_calls(source);
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert_eq!(call.method, "GET");
        assert_eq!(call.controller_name.as_deref(), Some("Products"));
        assert_eq!(call.action_name.as_deref(), Some("GetAll"));
        assert_eq!(call.url_pattern, "/Products/GetAll");
    }

    #[test]
    fn test_extract_ajax_calls_post_get() {
        let source = r#"
$.post('/Orders/Create', data, function(result) { });
$.get('/Orders/Details', { id: 5 }, function(result) { });
"#;
        let calls = extract_ajax_calls(source);
        assert_eq!(calls.len(), 2);

        let post = calls.iter().find(|c| c.method == "POST").unwrap();
        assert_eq!(post.controller_name.as_deref(), Some("Orders"));
        assert_eq!(post.action_name.as_deref(), Some("Create"));

        let get = calls.iter().find(|c| c.method == "GET").unwrap();
        assert_eq!(get.controller_name.as_deref(), Some("Orders"));
        assert_eq!(get.action_name.as_deref(), Some("Details"));
    }

    #[test]
    fn test_extract_ajax_calls_url_action() {
        let source = r#"
var url = @Url.Action("Delete", "Products");
"#;
        let calls = extract_ajax_calls(source);
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert_eq!(call.controller_name.as_deref(), Some("Products"));
        assert_eq!(call.action_name.as_deref(), Some("Delete"));
    }

    #[test]
    fn test_extract_ajax_calls_fetch() {
        let source = r#"
fetch('/api/Products/Search')
    .then(r => r.json());
"#;
        let calls = extract_ajax_calls(source);
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert_eq!(call.controller_name.as_deref(), Some("Products"));
        assert_eq!(call.action_name.as_deref(), Some("Search"));
    }

    // ─── Telerik / Kendo extraction tests ───────────────────────────────

    #[test]
    fn test_extract_telerik_kendo_grid() {
        let source = r#"
@(Html.Kendo().Grid<ProductViewModel>()
    .Name("productsGrid")
    .Columns(columns => {
        columns.Bound(p => p.Name);
        columns.Bound(p => p.Price);
    })
    .DataSource(ds => ds
        .Ajax()
        .Read(read => read.Action("GetProducts", "Products"))
        .Create(create => create.Action("CreateProduct", "Products"))
        .Update(update => update.Action("UpdateProduct", "Products"))
        .Destroy(destroy => destroy.Action("DeleteProduct", "Products"))
    )
)
"#;
        let components = extract_telerik_components(source);
        assert_eq!(components.len(), 1);

        let grid = &components[0];
        assert_eq!(grid.component_type, "Grid");
        assert_eq!(grid.vendor, "Kendo");
        assert_eq!(grid.model_type.as_deref(), Some("ProductViewModel"));
        assert_eq!(grid.data_source_actions.len(), 4);

        let read = grid.data_source_actions.iter().find(|a| a.operation == "Read").unwrap();
        assert_eq!(read.action_name, "GetProducts");
        assert_eq!(read.controller_name, "Products");
    }

    #[test]
    fn test_extract_telerik_legacy() {
        let source = r#"
@(Html.Telerik().Grid<OrderViewModel>()
    .Name("ordersGrid")
    .DataBinding(db => db
        .Ajax()
        .Select("GetOrders", "Orders")
    )
)
"#;
        let components = extract_telerik_components(source);
        assert_eq!(components.len(), 1);

        let grid = &components[0];
        assert_eq!(grid.component_type, "Grid");
        assert_eq!(grid.vendor, "Telerik");
        assert_eq!(grid.model_type.as_deref(), Some("OrderViewModel"));
        // Legacy .Select("Action", "Controller") should be captured as a Read DataSource action
        assert_eq!(grid.data_source_actions.len(), 1);
        assert_eq!(grid.data_source_actions[0].operation, "Read");
        assert_eq!(grid.data_source_actions[0].action_name, "GetOrders");
        assert_eq!(grid.data_source_actions[0].controller_name, "Orders");
    }

    #[test]
    fn test_extract_telerik_real_world_grid() {
        // Real-world Telerik Extensions for ASP.NET MVC pattern from a legacy MVC5 app
        let source = r#"
@(Html.Telerik().Grid<Export>()
    .Name("GridExports")
    .DataBinding(dataBinding => dataBinding.Ajax()
        .Select("GetExportElodieGrid", "Factures"))
    .Columns(columns => {
        columns.Bound(e => e.DateCréation).Title("Date export");
        columns.Bound(e => e.NomExport).Title("Nom du fichier");
    })
    .ClientEvents(events => events
        .OnDataBinding("onGridDataBinding")
        .OnDataBound("onGridDataBound")
        .OnError("onGridError"))
)
"#;
        let components = extract_telerik_components(source);
        assert_eq!(components.len(), 1, "Should detect one Telerik Grid component");

        let grid = &components[0];
        assert_eq!(grid.component_type, "Grid");
        assert_eq!(grid.vendor, "Telerik");
        assert_eq!(grid.model_type.as_deref(), Some("Export"));

        // DataSource: .Select("GetExportElodieGrid", "Factures") → Read action
        assert_eq!(grid.data_source_actions.len(), 1);
        assert_eq!(grid.data_source_actions[0].operation, "Read");
        assert_eq!(grid.data_source_actions[0].action_name, "GetExportElodieGrid");
        assert_eq!(grid.data_source_actions[0].controller_name, "Factures");

        // Client events
        assert!(grid.client_events.len() >= 2, "Should detect at least OnDataBinding and OnDataBound");
    }

    #[test]
    fn test_extract_telerik_kendo_jquery() {
        let source = r##"
<script>
    $("#grid").kendoGrid({
        dataSource: { transport: { read: "/api/data" } }
    });
</script>
"##;
        let components = extract_telerik_components(source);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "Grid");
        assert_eq!(components[0].vendor, "Kendo");
    }

    #[test]
    fn test_extract_telerik_client_events() {
        let source = r#"
@(Html.Kendo().Grid<ProductViewModel>()
    .Name("grid")
    .Events(e => e
        .OnChange("onGridChange")
        .OnDataBound("onDataBound")
    )
)
"#;
        let components = extract_telerik_components(source);
        assert_eq!(components.len(), 1);

        let grid = &components[0];
        assert_eq!(grid.client_events.len(), 2);
        assert!(grid.client_events.iter().any(|(e, h)| e == "Change" && h == "onGridChange"));
        assert!(grid.client_events.iter().any(|(e, h)| e == "DataBound" && h == "onDataBound"));
    }

    // ─── Service / Repository extraction tests ──────────────────────────

    #[test]
    fn test_extract_service_class() {
        let source = r#"
public class ProductService : IProductService
{
    private readonly IProductRepository _repo;

    public ProductService(IProductRepository repo)
    {
        _repo = repo;
    }

    public Product GetById(int id) => _repo.GetById(id);
}
"#;
        let services = extract_services_and_repositories(source);
        assert_eq!(services.len(), 1);

        let svc = &services[0];
        assert_eq!(svc.class_name, "ProductService");
        assert_eq!(svc.layer_type, "Service");
        assert_eq!(svc.implements_interface.as_deref(), Some("IProductService"));
        assert_eq!(svc.dependencies.len(), 1);
        assert_eq!(svc.dependencies[0].0, "IProductRepository");
        assert_eq!(svc.dependencies[0].1, "repo");
    }

    #[test]
    fn test_extract_repository_class() {
        let source = r#"
public class OrderRepository : IOrderRepository
{
    private readonly ApplicationDbContext _context;

    public OrderRepository(IUnitOfWork unitOfWork, ILogger logger)
    {
        _context = unitOfWork.Context;
    }
}
"#;
        let services = extract_services_and_repositories(source);
        assert_eq!(services.len(), 1);

        let repo = &services[0];
        assert_eq!(repo.class_name, "OrderRepository");
        assert_eq!(repo.layer_type, "Repository");
        assert_eq!(repo.implements_interface.as_deref(), Some("IOrderRepository"));
        assert_eq!(repo.dependencies.len(), 2);
        assert!(repo.dependencies.iter().any(|(t, _)| t == "IUnitOfWork"));
        assert!(repo.dependencies.iter().any(|(t, _)| t == "ILogger"));
    }

    #[test]
    fn test_extract_constructor_deps() {
        let source = r#"
public class InvoiceManager : IInvoiceManager
{
    public InvoiceManager(IOrderService orderService, IEmailService emailService)
    {
    }
}
"#;
        let deps = extract_constructor_dependencies(source, "InvoiceManager");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], ("IOrderService".to_string(), "orderService".to_string()));
        assert_eq!(deps[1], ("IEmailService".to_string(), "emailService".to_string()));
    }

    // ─── New regression tests (Alise_v2 legacy gaps) ────────────────────

    #[test]
    fn test_extract_ajax_getjson() {
        let source = r#"
$.getJSON('/Dossiers/AfficherAides', { id: dossierId }, function(data) {
    $('#aides-container').html(data);
});
"#;
        let calls = extract_ajax_calls(source);
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.method, "GET");
        assert_eq!(call.controller_name.as_deref(), Some("Dossiers"));
        assert_eq!(call.action_name.as_deref(), Some("AfficherAides"));
    }

    #[test]
    fn test_extract_ajax_load() {
        let source = r#"
$('#details-panel').load('/Parametrage/GetDetails', { id: paramId });
"#;
        let calls = extract_ajax_calls(source);
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.method, "GET");
        assert_eq!(call.controller_name.as_deref(), Some("Parametrage"));
        assert_eq!(call.action_name.as_deref(), Some("GetDetails"));
    }

    #[test]
    fn test_extract_di_autofac() {
        let source = r#"
builder.RegisterType<ParametrageService>().As<IParametrageService>().SingleInstance();
builder.RegisterType<DossierRepository>().As<IDossierRepository>().InstancePerRequest();
"#;
        let regs = extract_di_registrations(source);
        assert_eq!(regs.len(), 2);

        let first = &regs[0];
        assert_eq!(first.implementation_type, "ParametrageService");
        assert_eq!(first.service_type, "IParametrageService");
        assert_eq!(first.framework, "Autofac");
        assert_eq!(first.lifetime.as_deref(), Some("Singleton"));

        let second = &regs[1];
        assert_eq!(second.implementation_type, "DossierRepository");
        assert_eq!(second.service_type, "IDossierRepository");
        assert_eq!(second.framework, "Autofac");
        assert_eq!(second.lifetime.as_deref(), Some("PerRequest"));
    }

    #[test]
    fn test_extract_unitofwork() {
        let source = r#"
public class UnitOfWorkAide : IUnitOfWork
{
    private readonly ApplicationDbContext _context;

    public UnitOfWorkAide(IApplicationDbContext context)
    {
        _context = (ApplicationDbContext)context;
    }
}
"#;
        let services = extract_services_and_repositories(source);
        assert_eq!(services.len(), 1);

        let uow = &services[0];
        assert_eq!(uow.class_name, "UnitOfWorkAide");
        assert_eq!(uow.layer_type, "UnitOfWork");
        assert_eq!(uow.implements_interface.as_deref(), Some("IUnitOfWork"));
    }

    #[test]
    fn test_extract_form_action() {
        let source = r#"
@using (Html.BeginForm("LogOff", "Home", FormMethod.Post, new { id = "logoutForm" }))
{
    <button type="submit">Log off</button>
}

@using (Html.BeginForm("Search", "Dossiers"))
{
    <input type="text" name="q" />
}
"#;
        let forms = extract_form_actions(source);
        assert_eq!(forms.len(), 2);

        let logoff = &forms[0];
        assert_eq!(logoff.action_name, "LogOff");
        assert_eq!(logoff.controller_name, "Home");
        assert_eq!(logoff.http_method, "POST");

        let search = &forms[1];
        assert_eq!(search.action_name, "Search");
        assert_eq!(search.controller_name, "Dossiers");
        assert_eq!(search.http_method, "POST"); // default when FormMethod not specified
    }
}

//! ASP.NET MVC 5 / Web API attribute extraction from C# source code.
//!
//! Extracts routing attributes, HTTP method decorators, data annotations,
//! controller/action detection, DbContext/Entity patterns, and Area associations
//! from C# source text using regex-based parsing (no tree-sitter dependency).
//!
//! This module provides the semantic layer needed to promote generic `Class`/`Method`
//! graph nodes into richer ASP.NET-specific types (`Controller`, `ControllerAction`,
//! `ApiEndpoint`, `View`, `ViewModel`, `DbEntity`, `DbContext`, `Area`).

mod types;
mod helpers;
mod controllers;
mod views;
mod database;
mod ajax;
mod telerik;
mod services;
mod di;
mod forms;
mod tracing;
mod external;

// Re-export all public types
pub use types::*;

// Re-export all public functions
pub use controllers::extract_controllers;
pub use views::extract_view_info;
pub use database::{extract_db_contexts, extract_entities};
pub use ajax::extract_ajax_calls;
pub use telerik::extract_telerik_components;
pub use services::{extract_services_and_repositories, extract_constructor_dependencies};
pub use di::extract_di_registrations;
pub use forms::{extract_form_actions, extract_partial_references};
pub use tracing::extract_tracing_info;
pub use external::extract_external_service_calls;

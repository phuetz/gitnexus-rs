//! The `generate` command: produces AI context files (AGENTS.md, wiki/, skills/) from the knowledge graph.

mod utils;
mod markdown;
mod enrichment;
mod cross_ref;
mod agents;
mod wiki;
mod skills;
mod docs;
mod health;
mod analytics;
mod functional;
mod deployment;
mod html;

pub(crate) use enrichment::load_llm_config;

use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use tracing::info;

use gitnexus_core::storage::repo_manager;
use gitnexus_db::snapshot;

// ─── Constants ──────────────────────────────────────────────────────────
const TARGET_CONTEXT: &str = "context";
const TARGET_AGENTS: &str = "agents";
const TARGET_WIKI: &str = "wiki";
const TARGET_SKILLS: &str = "skills";
const TARGET_DOCS: &str = "docs";
const TARGET_DOCX: &str = "docx";
const TARGET_HTML: &str = "html";
const TARGET_ALL: &str = "all";

pub fn run(what: &str, path: Option<&str>, enrich: bool, enrich_profile: &str, enrich_lang: &str, enrich_citations: bool) -> Result<()> {
    let repo_path = Path::new(path.unwrap_or(".")).canonicalize()?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap_path = snapshot::snapshot_path(&storage.storage_path);
    let graph = snapshot::load_snapshot(&snap_path)?;

    info!("Generating {} for {}", what, repo_path.display());

    match what {
        TARGET_CONTEXT | TARGET_AGENTS => agents::generate_agents_md(&graph, &repo_path)?,
        TARGET_WIKI => wiki::generate_wiki(&graph, &repo_path)?,
        TARGET_SKILLS => skills::generate_skills(&graph, &repo_path)?,
        TARGET_DOCS => {
            docs::generate_docs(&graph, &repo_path)?;
            enrichment::run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = cross_ref::apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
        }
        TARGET_DOCX => {
            // Generate Markdown first, enrich, cross-ref, then convert to DOCX
            docs::generate_docs(&graph, &repo_path)?;
            enrichment::run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = cross_ref::apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
            let output_path = repo_path.join(".gitnexus").join("documentation.docx");
            let repo_name = repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Project");
            super::export_docx::export_docs_as_docx(&docs_dir, &output_path, repo_name)?;
            info!("Generated DOCX documentation at {}", output_path.display());
            println!(
                "{} Generated DOCX: {}",
                "OK".green(),
                output_path.display()
            );
        }
        TARGET_HTML => {
            // Generate Markdown first, enrich, cross-ref, then convert to HTML site
            docs::generate_docs(&graph, &repo_path)?;
            enrichment::run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = cross_ref::apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
            html::generate_html_site(&graph, &repo_path)?;
        }
        TARGET_ALL => {
            agents::generate_agents_md(&graph, &repo_path)?;
            wiki::generate_wiki(&graph, &repo_path)?;
            skills::generate_skills(&graph, &repo_path)?;
            docs::generate_docs(&graph, &repo_path)?;
            enrichment::run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = cross_ref::apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
            // Also generate DOCX
            let output_path = repo_path.join(".gitnexus").join("documentation.docx");
            let repo_name = repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Project");
            super::export_docx::export_docs_as_docx(&docs_dir, &output_path, repo_name)?;
            info!("Generated DOCX documentation at {}", output_path.display());
            println!(
                "{} Generated DOCX: {}",
                "OK".green(),
                output_path.display()
            );
            // Also generate HTML site
            html::generate_html_site(&graph, &repo_path)?;
        }
        _ => {
            eprintln!(
                "Unknown target: {}. Use: context, wiki, skills, docs, docx, html, all",
                what
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::html::strip_html_tags;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World"), "hello_world");
        assert_eq!(sanitize_filename("DossiersController"), "dossierscontroller");
        assert_eq!(sanitize_filename("a-b_c"), "a-b_c");
    }

    #[test]
    fn test_extract_params_from_content() {
        assert_eq!(
            extract_params_from_content("string id, int page", "test"),
            "`string` id, `int` page"
        );
        assert_eq!(
            extract_params_from_content("", "test"),
            "-"
        );
        assert_eq!(
            extract_params_from_content("DossierPresta dossier", "test"),
            "`DossierPresta` dossier"
        );
    }

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("<h1>Hello</h1><p>World</p>"), "Hello World");
        assert_eq!(strip_html_tags("no tags here"), "no tags here");
        assert_eq!(strip_html_tags("<a href='x'>link</a> text"), "link text");
        assert_eq!(strip_html_tags(""), "");
    }
}

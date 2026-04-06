//! Process documentation generator: produces markdown for each detected Process in the graph.
//!
//! Combines three evidence sources with explicit priority:
//! 1. Code (highest) — source code from the knowledge graph
//! 2. Traces (high) — runtime behavior from is_traced/trace_call_count
//! 3. RAG docs (informative) — external documentation, marked as potentially outdated

use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use tracing::info;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::process_doc::{
    self, collect_full_evidence, generate_component_diagram, generate_sequence_diagram,
    EvidenceSource, ProcessEvidence,
};

use super::utils::sanitize_filename;

/// Generate process documentation pages.
/// Returns (id, title, filename) tuples for `_index.json` integration.
pub(super) fn generate_process_docs(
    graph: &KnowledgeGraph,
    repo_path: &Path,
    docs_dir: &Path,
) -> Result<Vec<(String, String, String)>> {
    // Collect all Process nodes
    let mut processes: Vec<(String, String, Option<u32>)> = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::Process)
        .map(|n| {
            (
                n.id.clone(),
                n.properties.name.clone(),
                n.properties.step_count,
            )
        })
        .collect();

    if processes.is_empty() {
        info!("No processes found in graph, skipping process-doc generation");
        return Ok(Vec::new());
    }

    // Sort by step_count descending (most complex first)
    processes.sort_by(|a, b| b.2.unwrap_or(0).cmp(&a.2.unwrap_or(0)));

    let processes_dir = docs_dir.join("processes");
    std::fs::create_dir_all(&processes_dir)?;

    println!(
        "{} Generating process documentation ({} processes)...",
        "->".cyan(),
        processes.len()
    );

    let mut page_entries = Vec::new();
    let mut all_evidence = Vec::new();

    for (process_id, process_name, _) in &processes {
        let evidence = collect_full_evidence(graph, process_id, repo_path);

        let slug = sanitize_filename(process_name);
        let filename = format!("processes/process-{}.md", slug);
        let filepath = docs_dir.join(&filename);

        let content = render_process_page(&evidence);
        std::fs::write(&filepath, &content)?;

        page_entries.push((
            format!("process-{}", slug),
            format!("Process: {}", process_name),
            filename,
        ));
        all_evidence.push(evidence);
    }

    // Generate overview page
    let overview_content = render_process_overview(&all_evidence);
    std::fs::write(docs_dir.join("process-overview.md"), &overview_content)?;
    page_entries.insert(
        0,
        (
            "process-overview".to_string(),
            "Business Processes".to_string(),
            "process-overview.md".to_string(),
        ),
    );

    println!(
        "{} Generated {} process documentation pages",
        "OK".green(),
        page_entries.len()
    );

    Ok(page_entries)
}

// ─── Page rendering ─────────────────────────────────────────────────────

fn render_process_page(ev: &ProcessEvidence) -> String {
    let mut md = String::with_capacity(8192);

    // Header
    md.push_str(&format!("# Process: {}\n\n", ev.process_name));
    md.push_str("<!-- GNX:LEAD -->\n\n");

    // Source files
    let mut files: Vec<&str> = ev.steps.iter().map(|s| s.file_path.as_str()).collect();
    files.sort();
    files.dedup();
    if !files.is_empty() {
        md.push_str("<details>\n<summary>Relevant source files</summary>\n\n");
        for f in &files {
            md.push_str(&format!("- `{}`\n", f));
        }
        md.push_str("\n</details>\n\n");
    }

    // Properties table
    let process_type = ev
        .process_type
        .map(|t| match t {
            ProcessType::IntraCommunity => "Intra-community",
            ProcessType::CrossCommunity => "Cross-community",
        })
        .unwrap_or("Unknown");

    let trace_assessment = if ev.trace_coverage_pct >= 80.0 {
        "Good"
    } else if ev.trace_coverage_pct >= 40.0 {
        "Partial"
    } else {
        "Low"
    };

    md.push_str("| Property | Value |\n");
    md.push_str("|----------|-------|\n");
    md.push_str(&format!("| Type | {} |\n", process_type));
    md.push_str(&format!("| Steps | {} |\n", ev.steps.len()));
    if let Some(entry) = &ev.entry_point_name {
        let file = ev.entry_point_file.as_deref().unwrap_or("");
        md.push_str(&format!("| Entry Point | `{}` in `{}` |\n", entry, file));
    }
    if let Some(term) = &ev.terminal_name {
        md.push_str(&format!("| Terminal | `{}` |\n", term));
    }
    md.push_str(&format!(
        "| Trace Coverage | {:.0}% ({}/{} steps) — {} |\n",
        ev.trace_coverage_pct,
        ev.steps.iter().filter(|s| s.is_traced).count(),
        ev.steps.len(),
        trace_assessment
    ));
    if !ev.communities.is_empty() {
        md.push_str(&format!("| Communities | {} |\n", ev.communities.join(", ")));
    }
    md.push('\n');

    // Process Flow
    md.push_str("## Process Flow\n\n");
    md.push_str("<!-- GNX:INTRO:process-flow -->\n\n");

    let seq_diagram = generate_sequence_diagram(&ev.steps);
    md.push_str("```mermaid\n");
    md.push_str(&seq_diagram);
    md.push_str("\n```\n\n");

    // Step-by-step table
    md.push_str("### Step-by-Step Breakdown\n\n");
    md.push_str("| Step | Method | Class | File | Traced |\n");
    md.push_str("|------|--------|-------|------|--------|\n");
    for step in &ev.steps {
        let class = step.class_name.as_deref().unwrap_or("-");
        let traced = if step.is_traced { "Yes" } else { "No" };
        md.push_str(&format!(
            "| {} | `{}` | {} | `{}:L{}` | {} |\n",
            step.step_number,
            step.name,
            class,
            step.file_path,
            step.start_line.unwrap_or(0),
            traced
        ));
    }
    md.push('\n');

    // Detailed step evidence
    for (i, step) in ev.steps.iter().enumerate() {
        md.push_str(&format!(
            "#### Step {}: `{}`\n\n",
            step.step_number, step.name
        ));

        if let Some(class) = &step.class_name {
            md.push_str(&format!("**Component**: {} ({})\n\n", class, step.label.as_str()));
        }

        // Code evidence (highest priority)
        if let Some(step_ev) = ev.step_evidence.get(i) {
            for evidence in step_ev.iter().filter(|e| e.source == EvidenceSource::Code) {
                let lang = step
                    .language
                    .map(|l| l.as_str().to_lowercase())
                    .unwrap_or_else(|| detect_lang_from_path(&step.file_path));
                md.push_str(&format!("**Source Code** `[{}]`\n\n", evidence.id));
                md.push_str(&format!("```{}\n{}\n```\n\n", lang, evidence.content));
            }

            // Trace evidence
            for evidence in step_ev.iter().filter(|e| e.source == EvidenceSource::Trace) {
                md.push_str(&format!("**Execution Data** `[{}]`\n\n", evidence.id));
                md.push_str(&format!("{}\n\n", evidence.content));
            }
        }

        // RAG evidence for this step (from global, matching this step's node_id)
        let step_rag: Vec<_> = ev
            .global_rag_evidence
            .iter()
            .filter(|e| e.source == EvidenceSource::Rag)
            .take(2)
            .collect();

        if !step_rag.is_empty() && i == 0 {
            // Show RAG context once, at the first step
            md.push_str("> [!NOTE]\n");
            md.push_str("> This information comes from external documentation and may not reflect the current code.\n\n");
            for evidence in step_rag {
                // Truncate long RAG content
                let content = if evidence.content.len() > 500 {
                    format!("{}...", &evidence.content[..500])
                } else {
                    evidence.content.clone()
                };
                md.push_str(&format!("{} `[{}]`\n\n", content, evidence.id));
            }
        }
    }

    // Data Flow
    if !ev.entities.is_empty() {
        md.push_str("## Data Flow\n\n");
        md.push_str("<!-- GNX:INTRO:data-flow -->\n\n");

        let reads: Vec<_> = ev
            .entities
            .iter()
            .filter(|e| matches!(e.access_type, process_doc::EntityAccessType::Read | process_doc::EntityAccessType::ReadWrite))
            .collect();
        let writes: Vec<_> = ev
            .entities
            .iter()
            .filter(|e| matches!(e.access_type, process_doc::EntityAccessType::Write | process_doc::EntityAccessType::ReadWrite))
            .collect();

        if !reads.is_empty() {
            md.push_str("### Input Entities\n\n");
            md.push_str("| Entity | Table | Accessed By |\n");
            md.push_str("|--------|-------|-------------|\n");
            for entity in reads {
                let table = entity.db_table_name.as_deref().unwrap_or("-");
                let step = entity
                    .accessed_by_step
                    .map(|s| format!("Step {}", s))
                    .unwrap_or_else(|| "-".to_string());
                md.push_str(&format!("| {} | {} | {} |\n", entity.name, table, step));
            }
            md.push('\n');
        }

        if !writes.is_empty() {
            md.push_str("### Output Entities & Side Effects\n\n");
            md.push_str("| Entity | Table | Written By |\n");
            md.push_str("|--------|-------|------------|\n");
            for entity in writes {
                let table = entity.db_table_name.as_deref().unwrap_or("-");
                let step = entity
                    .accessed_by_step
                    .map(|s| format!("Step {}", s))
                    .unwrap_or_else(|| "-".to_string());
                md.push_str(&format!("| {} | {} | {} |\n", entity.name, table, step));
            }
            md.push('\n');
        }
    }

    // Component Architecture
    if !ev.components.is_empty() {
        md.push_str("## Component Architecture\n\n");
        md.push_str("<!-- GNX:INTRO:components -->\n\n");

        let comp_diagram = generate_component_diagram(&ev.components);
        md.push_str("```mermaid\n");
        md.push_str(&comp_diagram);
        md.push_str("\n```\n\n");

        md.push_str("| Component | Type | File | Steps |\n");
        md.push_str("|-----------|------|------|-------|\n");
        for comp in &ev.components {
            md.push_str(&format!(
                "| {} | {} | `{}` | {} |\n",
                comp.name,
                comp.label.as_str(),
                comp.file_path,
                comp.step_count
            ));
        }
        md.push('\n');
    }

    // Quality Metrics
    md.push_str("## Quality Metrics\n\n");
    md.push_str("<!-- GNX:INTRO:quality -->\n\n");
    md.push_str("| Metric | Value | Assessment |\n");
    md.push_str("|--------|-------|------------|\n");
    md.push_str(&format!(
        "| Trace Coverage | {:.0}% | {} |\n",
        ev.trace_coverage_pct, trace_assessment
    ));
    md.push_str(&format!(
        "| Dead Code Candidates | {} methods | {} |\n",
        ev.dead_code_candidates.len(),
        if ev.dead_code_candidates.is_empty() {
            "Clean"
        } else {
            "Review needed"
        }
    ));
    md.push_str(&format!("| Process Type | {} | |\n", process_type));
    md.push('\n');

    // References
    md.push_str("## References\n\n");
    md.push_str("<!-- GNX:INTRO:references -->\n\n");

    md.push_str("### Source Files\n\n");
    md.push_str("| File | Lines | Component |\n");
    md.push_str("|------|-------|-----------|\n");
    for step in &ev.steps {
        let class = step.class_name.as_deref().unwrap_or("-");
        let lines = match (step.start_line, step.end_line) {
            (Some(s), Some(e)) => format!("{}-{}", s, e),
            _ => "-".to_string(),
        };
        md.push_str(&format!(
            "| `{}` | {} | {} |\n",
            step.file_path, lines, class
        ));
    }
    md.push('\n');

    if !ev.global_rag_evidence.is_empty() {
        md.push_str("### Related Documentation\n\n");
        md.push_str("> [!NOTE]\n");
        md.push_str("> External documentation may not reflect the current codebase.\n\n");
        md.push_str("| Document | Source |\n");
        md.push_str("|----------|--------|\n");
        for rag in &ev.global_rag_evidence {
            let title = rag
                .content
                .lines()
                .next()
                .unwrap_or("Untitled")
                .chars()
                .take(60)
                .collect::<String>();
            md.push_str(&format!("| {} | `[{}]` |\n", title, rag.id));
        }
        md.push('\n');
    }

    md.push_str("<!-- GNX:CLOSING -->\n");

    md
}

fn render_process_overview(all_evidence: &[ProcessEvidence]) -> String {
    let mut md = String::with_capacity(4096);

    md.push_str("# Business Processes\n\n");
    md.push_str("<!-- GNX:LEAD -->\n\n");

    let total = all_evidence.len();
    let fully_traced = all_evidence
        .iter()
        .filter(|e| e.trace_coverage_pct >= 100.0)
        .count();
    let partially_traced = all_evidence
        .iter()
        .filter(|e| e.trace_coverage_pct > 0.0 && e.trace_coverage_pct < 100.0)
        .count();
    let untraced = total - fully_traced - partially_traced;

    md.push_str(&format!(
        "> This section documents the {} execution flows detected in the codebase.\n\n",
        total
    ));

    // Process table
    md.push_str("| # | Process | Steps | Type | Entry Point | Trace Coverage |\n");
    md.push_str("|---|---------|-------|------|-------------|----------------|\n");
    for (i, ev) in all_evidence.iter().enumerate() {
        let slug = sanitize_filename(&ev.process_name);
        let ptype = ev
            .process_type
            .map(|t| match t {
                ProcessType::IntraCommunity => "Intra",
                ProcessType::CrossCommunity => "Cross",
            })
            .unwrap_or("-");
        let entry = ev.entry_point_name.as_deref().unwrap_or("-");
        md.push_str(&format!(
            "| {} | [{}](./processes/process-{}.md) | {} | {} | `{}` | {:.0}% |\n",
            i + 1,
            ev.process_name,
            slug,
            ev.steps.len(),
            ptype,
            entry,
            ev.trace_coverage_pct
        ));
    }
    md.push('\n');

    // Trace coverage summary
    md.push_str("## Trace Coverage Summary\n\n");
    md.push_str("| Metric | Value |\n");
    md.push_str("|--------|-------|\n");
    md.push_str(&format!("| Total Processes | {} |\n", total));
    md.push_str(&format!("| Fully Traced (100%) | {} |\n", fully_traced));
    md.push_str(&format!("| Partially Traced | {} |\n", partially_traced));
    md.push_str(&format!("| Untraced | {} |\n", untraced));
    md.push('\n');

    md.push_str("<!-- GNX:CLOSING -->\n");

    md
}

fn detect_lang_from_path(path: &str) -> String {
    if path.ends_with(".cs") {
        "csharp".to_string()
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        "typescript".to_string()
    } else if path.ends_with(".js") || path.ends_with(".jsx") {
        "javascript".to_string()
    } else if path.ends_with(".rs") {
        "rust".to_string()
    } else if path.ends_with(".py") {
        "python".to_string()
    } else if path.ends_with(".java") || path.ends_with(".kt") {
        "java".to_string()
    } else if path.ends_with(".go") {
        "go".to_string()
    } else {
        String::new()
    }
}

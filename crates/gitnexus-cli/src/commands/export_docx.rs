//! DOCX export engine — converts Markdown documentation into Word documents.
//!
//! Generates professional .docx files using Open XML (OOXML) format by:
//! 1. Reading all `.md` files from `.gitnexus/docs/` (including ASP.NET pages)
//! 2. Reading `_index.json` to determine section order and hierarchy
//! 3. Parsing Markdown into a structured document tree
//! 4. Writing OOXML (ZIP-packaged XML) with proper styles, headers, tables, TOC
//!
//! The generated document includes:
//! - Title page with project name, stats summary, and GitNexus branding
//! - Table of contents (TOC field) — auto-updatable in Word
//! - All documentation pages in logical order with proper heading hierarchy
//! - Tables rendered as Word tables with styled headers and alternating rows
//! - Code blocks with monospace font and grey background
//! - Mermaid diagrams as labelled placeholder with full source code
//! - Bullet lists, numbered lists, blockquotes
//! - Inline formatting: **bold**, *italic*, `code`, [links](url)

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use serde_json::Value;
use zip::write::{FileOptions, SimpleFileOptions};
use zip::ZipWriter;

/// Export all documentation as a single DOCX file.
/// Reads `_index.json` to determine page order, then converts all Markdown files.
pub fn export_docs_as_docx(
    docs_dir: &Path,
    output_path: &Path,
    project_name: &str,
) -> Result<()> {
    // Read _index.json for ordered page list and stats
    let index_path = docs_dir.join("_index.json");
    let (ordered_files, stats) = if index_path.exists() {
        let index_str = std::fs::read_to_string(&index_path)?;
        let index: Value = serde_json::from_str(&index_str)?;
        let files = collect_pages_from_index(&index);
        let stats = extract_stats(&index);
        (files, stats)
    } else {
        // Fallback: read files in hardcoded order
        (fallback_file_order(), DocStats::default())
    };

    // Read all markdown files in order
    let mut md_files: Vec<(String, String, String)> = Vec::new(); // (id, title, content)
    for (id, title, filename) in &ordered_files {
        let path = docs_dir.join(filename);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            md_files.push((id.clone(), title.clone(), content));
        }
    }

    if md_files.is_empty() {
        anyhow::bail!("No documentation files found in {}", docs_dir.display());
    }

    // Generate DOCX
    let file = std::fs::File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options: SimpleFileOptions = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 1. [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(CONTENT_TYPES_XML.as_bytes())?;

    // 2. _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(RELS_XML.as_bytes())?;

    // 4. word/styles.xml
    zip.start_file("word/styles.xml", options)?;
    zip.write_all(generate_styles_xml().as_bytes())?;

    // 5. word/numbering.xml
    zip.start_file("word/numbering.xml", options)?;
    zip.write_all(NUMBERING_XML.as_bytes())?;

    // 6. word/document.xml (main content) and collect hyperlinks
    let (document_xml, links) = generate_document_xml(project_name, &md_files, &stats);
    zip.start_file("word/document.xml", options)?;
    zip.write_all(document_xml.as_bytes())?;

    // 3. word/_rels/document.xml.rels (with dynamic hyperlinks)
    let doc_rels_xml = generate_document_rels(&links);
    zip.start_file("word/_rels/document.xml.rels", options)?;
    zip.write_all(doc_rels_xml.as_bytes())?;

    zip.finish()?;
    Ok(())
}

// ─── Index.json Parsing ─────────────────────────────────────────────────

#[derive(Default)]
struct DocStats {
    files: usize,
    nodes: usize,
    edges: usize,
    modules: usize,
}

fn extract_stats(index: &Value) -> DocStats {
    let s = &index["stats"];
    DocStats {
        files: s["files"].as_u64().unwrap_or(0) as usize,
        nodes: s["nodes"].as_u64().unwrap_or(0) as usize,
        edges: s["edges"].as_u64().unwrap_or(0) as usize,
        modules: s["modules"].as_u64().unwrap_or(0) as usize,
    }
}

/// Walk the _index.json page tree and return a flat list of (id, title, path).
fn collect_pages_from_index(index: &Value) -> Vec<(String, String, String)> {
    let mut result = Vec::new();
    if let Some(pages) = index["pages"].as_array() {
        for page in pages {
            collect_page_recursive(page, &mut result);
        }
    }
    result
}

fn collect_page_recursive(page: &Value, out: &mut Vec<(String, String, String)>) {
    let id = page["id"].as_str().unwrap_or("").to_string();
    let title = page["title"].as_str().unwrap_or("").to_string();

    // If page has a path, it's a leaf page
    if let Some(path) = page["path"].as_str() {
        out.push((id, title, path.to_string()));
    }

    // If page has children, recurse into them
    if let Some(children) = page["children"].as_array() {
        for child in children {
            collect_page_recursive(child, out);
        }
    }
}

fn fallback_file_order() -> Vec<(String, String, String)> {
    let files = [
        ("overview", "Overview", "overview.md"),
        ("architecture", "Architecture", "architecture.md"),
        ("getting-started", "Getting Started", "getting-started.md"),
        ("aspnet-controllers", "Controllers & Actions", "aspnet-controllers.md"),
        ("aspnet-routes", "API & Route Table", "aspnet-routes.md"),
        ("aspnet-entities", "Entity Data Model", "aspnet-entities.md"),
        ("aspnet-data-model", "Entity Relationship Diagram", "aspnet-data-model.md"),
        ("aspnet-views", "Views & Templates", "aspnet-views.md"),
        ("aspnet-areas", "MVC Areas", "aspnet-areas.md"),
        ("aspnet-seq-http", "Sequence: HTTP Request Flow", "aspnet-seq-http.md"),
        ("aspnet-seq-data", "Sequence: Data Access Flow", "aspnet-seq-data.md"),
    ];
    files
        .iter()
        .map(|(id, title, path)| (id.to_string(), title.to_string(), path.to_string()))
        .collect()
}

// ─── Document XML Generation ────────────────────────────────────────────

fn generate_document_xml(
    project_name: &str,
    md_files: &[(String, String, String)],
    stats: &DocStats,
) -> (String, Vec<(String, String)>) {
    let mut body = String::new();
    let mut links = Vec::new();

    // ── Title page ──
    body.push_str(&title_page(project_name, stats));
    body.push_str(PAGE_BREAK);

    // ── Table of contents ──
    body.push_str(&toc_field());
    body.push_str(PAGE_BREAK);

    // ── Document body: each markdown file as a section ──
    for (i, (_id, _title, content)) in md_files.iter().enumerate() {
        let (ooxml, doc_links) = markdown_to_ooxml(content);
        body.push_str(&ooxml);
        links.extend(doc_links);
        // Page break between sections (but not after last)
        if i < md_files.len() - 1 {
            body.push_str(PAGE_BREAK);
        }
    }

    let doc_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            xmlns:o="urn:schemas-microsoft-com:office:office"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
            xmlns:v="urn:schemas-microsoft-com:vml"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:w10="urn:schemas-microsoft-com:office:word"
            xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml">
  <w:body>
{body}
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>"#
    );
    (doc_xml, links)
}

const PAGE_BREAK: &str = r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#;

fn title_page(project_name: &str, stats: &DocStats) -> String {
    let date = chrono::Local::now().format("%d/%m/%Y").to_string();
    let mut s = format!(
        r#"
    <w:p><w:pPr><w:spacing w:before="3000"/><w:jc w:val="center"/></w:pPr>
      <w:r><w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:b/><w:sz w:val="60"/><w:color w:val="1B3A6B"/></w:rPr>
        <w:t>{}</w:t>
      </w:r>
    </w:p>
    <w:p><w:pPr><w:spacing w:before="200"/><w:jc w:val="center"/></w:pPr>
      <w:r><w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:sz w:val="32"/><w:color w:val="4472C4"/></w:rPr>
        <w:t>Documentation Technique et Fonctionnelle</w:t>
      </w:r>
    </w:p>
    <w:p><w:pPr><w:spacing w:before="120"/><w:jc w:val="center"/></w:pPr>
      <w:r><w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:sz w:val="22"/><w:color w:val="888888"/></w:rPr>
        <w:t xml:space="preserve">Audit de code automatise — {date}</w:t>
      </w:r>
    </w:p>"#,
        xml_escape(project_name),
        date = date
    );

    // Stats summary table on title page
    if stats.files > 0 || stats.nodes > 0 {
        s.push_str(&format!(
            r#"
    <w:p><w:pPr><w:spacing w:before="600"/><w:jc w:val="center"/></w:pPr></w:p>
    <w:tbl>
      <w:tblPr><w:tblW w:w="7000" w:type="dxa"/><w:jc w:val="center"/>
        <w:tblBorders>
          <w:top w:val="single" w:sz="4" w:space="0" w:color="D0D0D0"/>
          <w:bottom w:val="single" w:sz="4" w:space="0" w:color="D0D0D0"/>
          <w:insideH w:val="single" w:sz="4" w:space="0" w:color="D0D0D0"/>
          <w:insideV w:val="single" w:sz="4" w:space="0" w:color="D0D0D0"/>
        </w:tblBorders>
      </w:tblPr>
      <w:tblGrid><w:gridCol w:w="1750"/><w:gridCol w:w="1750"/><w:gridCol w:w="1750"/><w:gridCol w:w="1750"/></w:tblGrid>
      <w:tr>
        {stat_cell("Fichiers", stats.files)}
        {stat_cell("Noeuds", stats.nodes)}
        {stat_cell("Relations", stats.edges)}
        {stat_cell("Modules", stats.modules)}
      </w:tr>
    </w:tbl>"#,
            stat_cell = |label: &str, value: usize| -> String {
                format!(
                    r#"<w:tc><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="F8F9FA"/><w:tcMar><w:top w:w="120" w:type="dxa"/><w:bottom w:w="120" w:type="dxa"/></w:tcMar></w:tcPr>
                    <w:p><w:pPr><w:jc w:val="center"/><w:spacing w:after="0"/></w:pPr>
                      <w:r><w:rPr><w:b/><w:sz w:val="28"/><w:color w:val="1B3A6B"/></w:rPr><w:t>{}</w:t></w:r>
                    </w:p>
                    <w:p><w:pPr><w:jc w:val="center"/><w:spacing w:after="0"/></w:pPr>
                      <w:r><w:rPr><w:sz w:val="18"/><w:color w:val="888888"/></w:rPr><w:t>{}</w:t></w:r>
                    </w:p></w:tc>"#,
                    value, label
                )
            },
        ));
    }

    // Generator branding
    s.push_str(
        r#"
    <w:p><w:pPr><w:spacing w:before="800"/><w:jc w:val="center"/></w:pPr>
      <w:r><w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:sz w:val="18"/><w:color w:val="AAAAAA"/></w:rPr>
        <w:t xml:space="preserve">Genere automatiquement par GitNexus — Code Intelligence Engine</w:t>
      </w:r>
    </w:p>"#,
    );

    s
}

fn toc_field() -> String {
    r#"
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Table des matieres</w:t></w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:fldChar w:fldCharType="begin"/>
      </w:r>
      <w:r>
        <w:instrText xml:space="preserve"> TOC \o "1-4" \h \z \u </w:instrText>
      </w:r>
      <w:r>
        <w:fldChar w:fldCharType="separate"/>
      </w:r>
      <w:r>
        <w:rPr><w:i/><w:color w:val="999999"/></w:rPr>
        <w:t>Ouvrez ce document dans Word et appuyez sur Ctrl+A, F9 pour actualiser la table des matieres.</w:t>
      </w:r>
      <w:r>
        <w:fldChar w:fldCharType="end"/>
      </w:r>
    </w:p>
"#
    .to_string()
}

// ─── Markdown to OOXML Conversion ────────────────────────────────────────

/// Convert Markdown content to OOXML paragraphs.
/// Returns (ooxml_string, vec_of_links) where links are (rId, url) pairs.
fn markdown_to_ooxml(markdown: &str) -> (String, Vec<(String, String)>) {
    let mut result = String::new();
    let mut links = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Empty line
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Headings (H1 to H6, check longest prefix first)
        if let Some(rest) = trimmed.strip_prefix("###### ") {
            let (ooxml, doc_links) = heading(rest, 6);
            result.push_str(&ooxml);
            links.extend(doc_links);
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("##### ") {
            let (ooxml, doc_links) = heading(rest, 5);
            result.push_str(&ooxml);
            links.extend(doc_links);
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#### ") {
            let (ooxml, doc_links) = heading(rest, 4);
            result.push_str(&ooxml);
            links.extend(doc_links);
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            let (ooxml, doc_links) = heading(rest, 3);
            result.push_str(&ooxml);
            links.extend(doc_links);
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            let (ooxml, doc_links) = heading(rest, 2);
            result.push_str(&ooxml);
            links.extend(doc_links);
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let (ooxml, doc_links) = heading(rest, 1);
            result.push_str(&ooxml);
            links.extend(doc_links);
            i += 1;
            continue;
        }

        // Code blocks (fenced)
        if trimmed.starts_with("```") {
            let lang = trimmed.strip_prefix("```").unwrap_or("").trim().to_string();
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            i += 1; // skip closing ```

            if lang == "mermaid" {
                result.push_str(&mermaid_placeholder(&code_lines.join("\n")));
            } else {
                result.push_str(&code_block(&code_lines.join("\n"), &lang));
            }
            continue;
        }

        // Tables: collect contiguous lines starting with |
        if trimmed.starts_with('|') && trimmed.contains('|') {
            let mut table_lines = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                table_lines.push(lines[i].trim());
                i += 1;
            }
            let (ooxml, table_links) = table_to_ooxml(&table_lines);
            result.push_str(&ooxml);
            links.extend(table_links);
            continue;
        }

        // Nested bullet list (indented 2+ spaces + -)
        if (trimmed.starts_with("- ") || trimmed.starts_with("* "))
            && (line.starts_with("  ") || line.starts_with('\t'))
        {
            let content = &trimmed[2..];
            let (ooxml, item_links) = bullet_item(content, 1);
            result.push_str(&ooxml);
            links.extend(item_links);
            i += 1;
            continue;
        }

        // Bullet list
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            let (ooxml, item_links) = bullet_item(content, 0);
            result.push_str(&ooxml);
            links.extend(item_links);
            i += 1;
            continue;
        }

        // Numbered list
        if trimmed.len() > 2
            && trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
            && trimmed.contains(". ")
        {
            let dot_pos = trimmed.find(". ").unwrap_or(0);
            let content = &trimmed[dot_pos + 2..];
            let (ooxml, item_links) = numbered_item(content);
            result.push_str(&ooxml);
            links.extend(item_links);
            i += 1;
            continue;
        }

        // Blockquote
        if let Some(rest) = trimmed.strip_prefix("> ") {
            let (ooxml, blockquote_links) = blockquote(rest);
            result.push_str(&ooxml);
            links.extend(blockquote_links);
            i += 1;
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            result.push_str(HORIZONTAL_RULE);
            i += 1;
            continue;
        }

        // Regular paragraph
        let (ooxml, para_links) = paragraph(trimmed);
        result.push_str(&ooxml);
        links.extend(para_links);
        i += 1;
    }

    (result, links)
}

// ─── OOXML Element Builders ──────────────────────────────────────────────

fn heading(text: &str, level: u32) -> (String, Vec<(String, String)>) {
    let style = format!("Heading{}", level);
    let (runs, links) = inline_runs(text);
    (
        format!(
            r#"<w:p><w:pPr><w:pStyle w:val="{style}"/></w:pPr>{}</w:p>"#,
            runs
        ),
        links,
    )
}

fn paragraph(text: &str) -> (String, Vec<(String, String)>) {
    let (runs, links) = inline_runs(text);
    (
        format!(
            r#"<w:p><w:pPr><w:spacing w:after="120"/></w:pPr>{}</w:p>"#,
            runs
        ),
        links,
    )
}

fn bullet_item(text: &str, level: u32) -> (String, Vec<(String, String)>) {
    let indent = (level + 1) * 360;
    let (runs, links) = inline_runs(text);
    (
        format!(
            r#"<w:p><w:pPr><w:pStyle w:val="ListBullet"/><w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="1"/></w:numPr><w:ind w:left="{indent}" w:hanging="360"/></w:pPr>{}</w:p>"#,
            runs
        ),
        links,
    )
}

fn numbered_item(text: &str) -> (String, Vec<(String, String)>) {
    let (runs, links) = inline_runs(text);
    (
        format!(
            r#"<w:p><w:pPr><w:pStyle w:val="ListNumber"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>{}</w:p>"#,
            runs
        ),
        links,
    )
}

fn blockquote(text: &str) -> (String, Vec<(String, String)>) {
    let (runs, links) = inline_runs(text);
    (
        format!(
            r#"<w:p><w:pPr><w:pBdr><w:left w:val="single" w:sz="18" w:space="8" w:color="4472C4"/></w:pBdr><w:ind w:left="360"/><w:shd w:val="clear" w:color="auto" w:fill="F0F4FA"/><w:spacing w:after="120"/></w:pPr>{}</w:p>"#,
            runs
        ),
        links,
    )
}

fn code_block(code: &str, lang: &str) -> String {
    let mut result = String::new();
    // Language label header
    if !lang.is_empty() {
        result.push_str(&format!(
            r#"<w:p><w:pPr><w:shd w:val="clear" w:color="auto" w:fill="E8E8E8"/><w:spacing w:after="0"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/><w:sz w:val="16"/><w:b/><w:color w:val="888888"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
            xml_escape(lang)
        ));
    }
    // Code lines with monospace and grey background
    for line in code.lines() {
        result.push_str(&format!(
            r#"<w:p><w:pPr><w:shd w:val="clear" w:color="auto" w:fill="F2F2F2"/><w:spacing w:after="0" w:line="276" w:lineRule="auto"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/><w:sz w:val="18"/><w:color w:val="333333"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
            xml_escape(line)
        ));
    }
    // Spacer after code block
    result.push_str(r#"<w:p><w:pPr><w:spacing w:after="80"/></w:pPr></w:p>"#);
    result
}

fn mermaid_placeholder(code: &str) -> String {
    let mut result = String::new();

    // Determine diagram type for a better label
    let diagram_type = if code.starts_with("sequenceDiagram") {
        "Diagramme de Sequence"
    } else if code.starts_with("erDiagram") {
        "Diagramme Entite-Relation"
    } else if code.starts_with("graph TD") || code.starts_with("graph LR") {
        "Diagramme de Dependances"
    } else if code.starts_with("classDiagram") {
        "Diagramme de Classes"
    } else if code.starts_with("flowchart") {
        "Diagramme de Flux"
    } else {
        "Diagramme Mermaid"
    };

    // Header with diagram type icon
    result.push_str(&format!(
        r#"<w:p><w:pPr><w:shd w:val="clear" w:color="auto" w:fill="E8F0FE"/><w:spacing w:after="60"/><w:pBdr><w:top w:val="single" w:sz="4" w:space="4" w:color="4472C4"/><w:bottom w:val="single" w:sz="4" w:space="4" w:color="4472C4"/></w:pBdr></w:pPr><w:r><w:rPr><w:b/><w:color w:val="1B3A6B"/><w:sz w:val="22"/></w:rPr><w:t xml:space="preserve">  {diagram_type}</w:t></w:r></w:p>"#,
    ));

    // Instruction
    result.push_str(
        r#"<w:p><w:pPr><w:shd w:val="clear" w:color="auto" w:fill="E8F0FE"/><w:spacing w:after="60"/></w:pPr><w:r><w:rPr><w:i/><w:sz w:val="18"/><w:color w:val="666666"/></w:rPr><w:t xml:space="preserve">Copiez le code ci-dessous dans mermaid.live ou un viewer Mermaid pour voir le rendu visuel.</w:t></w:r></w:p>"#,
    );

    // Source code with slightly different background
    for line in code.lines() {
        result.push_str(&format!(
            r#"<w:p><w:pPr><w:shd w:val="clear" w:color="auto" w:fill="E8F0FE"/><w:spacing w:after="0"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/><w:sz w:val="16"/><w:color w:val="3366AA"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
            xml_escape(line)
        ));
    }
    // Spacer
    result.push_str(r#"<w:p><w:pPr><w:spacing w:after="120"/></w:pPr></w:p>"#);
    result
}

const HORIZONTAL_RULE: &str = r#"<w:p><w:pPr><w:pBdr><w:bottom w:val="single" w:sz="4" w:space="4" w:color="CCCCCC"/></w:pBdr><w:spacing w:before="200" w:after="200"/></w:pPr></w:p>"#;

fn table_to_ooxml(lines: &[&str]) -> (String, Vec<(String, String)>) {
    if lines.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut result = String::new();
    let mut links = Vec::new();

    // Parse header row
    let header = parse_table_row(lines[0]);

    // Skip separator row (|---|---|)
    let data_start = if lines.len() > 1 && lines[1].contains("---") {
        2
    } else {
        1
    };

    let col_count = header.len();
    if col_count == 0 {
        return (String::new(), Vec::new());
    }

    // Calculate column widths (total page width ~9000 twips)
    let col_width = 9000 / col_count;

    // Table start
    result.push_str(r#"<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="0" w:type="auto"/><w:tblBorders>"#);
    result.push_str(r#"<w:top w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>"#);
    result.push_str(r#"<w:left w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>"#);
    result.push_str(r#"<w:bottom w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>"#);
    result.push_str(r#"<w:right w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>"#);
    result.push_str(r#"<w:insideH w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>"#);
    result.push_str(r#"<w:insideV w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>"#);
    result.push_str(r#"</w:tblBorders></w:tblPr>"#);

    // Grid columns
    result.push_str("<w:tblGrid>");
    for _ in 0..col_count {
        result.push_str(&format!(r#"<w:gridCol w:w="{col_width}"/>"#));
    }
    result.push_str("</w:tblGrid>");

    // Header row (bold, dark blue background, white text)
    result.push_str("<w:tr>");
    for cell in &header {
        result.push_str(&format!(
            r#"<w:tc><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="1B3A6B"/><w:tcMar><w:top w:w="40" w:type="dxa"/><w:bottom w:w="40" w:type="dxa"/><w:left w:w="80" w:type="dxa"/><w:right w:w="80" w:type="dxa"/></w:tcMar></w:tcPr><w:p><w:pPr><w:spacing w:after="0"/></w:pPr><w:r><w:rPr><w:b/><w:color w:val="FFFFFF"/><w:sz w:val="20"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:tc>"#,
            xml_escape(cell)
        ));
    }
    result.push_str("</w:tr>");

    // Data rows with alternating background
    for i in data_start..lines.len() {
        let cells = parse_table_row(lines[i]);
        let bg = if (i - data_start) % 2 == 0 {
            "FFFFFF"
        } else {
            "F5F7FA"
        };

        result.push_str("<w:tr>");
        for (j, cell) in cells.iter().enumerate() {
            if j < col_count {
                let (runs, cell_links) = inline_runs(cell);
                links.extend(cell_links);
                result.push_str(&format!(
                    r#"<w:tc><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="{bg}"/><w:tcMar><w:top w:w="30" w:type="dxa"/><w:bottom w:w="30" w:type="dxa"/><w:left w:w="80" w:type="dxa"/><w:right w:w="80" w:type="dxa"/></w:tcMar></w:tcPr><w:p><w:pPr><w:spacing w:after="0"/></w:pPr>{}</w:p></w:tc>"#,
                    runs
                ));
            }
        }
        // Fill missing cells
        for _ in cells.len()..col_count {
            result.push_str(&format!(
                r#"<w:tc><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="{bg}"/></w:tcPr><w:p></w:p></w:tc>"#
            ));
        }
        result.push_str("</w:tr>");
    }

    result.push_str("</w:tbl>");
    result.push_str(r#"<w:p><w:pPr><w:spacing w:after="160"/></w:pPr></w:p>"#);
    (result, links)
}

fn parse_table_row(line: &str) -> Vec<String> {
    line.split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Handle inline markdown formatting: **bold**, *italic*, `code`, [text](url)
/// Returns (ooxml_string, vec_of_links) where links are (rId, url) pairs.
fn inline_runs(text: &str) -> (String, Vec<(String, String)>) {
    let mut result = String::new();
    let mut links = Vec::new();
    let mut rid_counter = 10; // Start from rId10 (rId1-2 reserved for styles/numbering)
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Bold: **text**
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_closing_double(&chars, i + 2, '*') {
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&format!(
                    r#"<w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#,
                    xml_escape(&inner)
                ));
                i = end + 2;
                continue;
            }
        }

        // Inline code: `text`
        if chars[i] == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                result.push_str(&format!(
                    r#"<w:r><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/><w:sz w:val="20"/><w:shd w:val="clear" w:color="auto" w:fill="F0F0F0"/><w:color w:val="C7254E"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#,
                    xml_escape(&inner)
                ));
                i = i + 1 + end + 1;
                continue;
            }
        }

        // Link: [text](url)
        if chars[i] == '[' {
            if let Some(link) = parse_link(&chars, i) {
                let rid = format!("rId{}", rid_counter);
                rid_counter += 1;
                // Record the link
                links.push((rid.clone(), link.url.clone()));
                // Render link as clickable hyperlink element
                result.push_str(&format!(
                    r#"<w:hyperlink r:id="{rid}"><w:r><w:rPr><w:color w:val="2E5EA0"/><w:u w:val="single"/><w:rStyle w:val="Hyperlink"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:hyperlink>"#,
                    xml_escape(&link.text)
                ));
                i = link.end_pos;
                continue;
            }
        }

        // Italic: *text* (but not **)
        if chars[i] == '*' && (i + 1 >= len || chars[i + 1] != '*') {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '*') {
                // Make sure it's not part of ** bold
                if i + 1 + end + 1 >= len || chars[i + 1 + end + 1] != '*' {
                    let inner: String = chars[i + 1..i + 1 + end].iter().collect();
                    result.push_str(&format!(
                        r#"<w:r><w:rPr><w:i/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#,
                        xml_escape(&inner)
                    ));
                    i = i + 1 + end + 1;
                    continue;
                }
            }
        }

        // Regular text: accumulate until next special char
        let start = i;
        while i < len && chars[i] != '*' && chars[i] != '`' && chars[i] != '[' {
            i += 1;
        }
        if i > start {
            let span: String = chars[start..i].iter().collect();
            result.push_str(&format!(
                r#"<w:r><w:t xml:space="preserve">{}</w:t></w:r>"#,
                xml_escape(&span)
            ));
        } else if i < len {
            // Unmatched special char — emit as-is
            result.push_str(&format!(
                r#"<w:r><w:t xml:space="preserve">{}</w:t></w:r>"#,
                xml_escape(&chars[i].to_string())
            ));
            i += 1;
        }
    }

    (result, links)
}

struct ParsedLink {
    text: String,
    url: String,
    end_pos: usize,
}

/// Parse a Markdown link: [text](url) starting at position `start` where chars[start] == '['
fn parse_link(chars: &[char], start: usize) -> Option<ParsedLink> {
    // Find closing ]
    let mut j = start + 1;
    while j < chars.len() && chars[j] != ']' {
        j += 1;
    }
    if j >= chars.len() {
        return None;
    }
    let text: String = chars[start + 1..j].iter().collect();

    // Expect ( immediately after ]
    if j + 1 >= chars.len() || chars[j + 1] != '(' {
        return None;
    }

    // Find closing )
    let mut k = j + 2;
    while k < chars.len() && chars[k] != ')' {
        k += 1;
    }
    if k >= chars.len() {
        return None;
    }
    let url: String = chars[j + 2..k].iter().collect();

    Some(ParsedLink {
        text,
        url,
        end_pos: k + 1,
    })
}

fn find_closing_double(chars: &[char], start: usize, ch: char) -> Option<usize> {
    if chars.len() < 2 {
        return None;
    }
    for i in start..chars.len() - 1 {
        if chars[i] == ch && chars[i + 1] == ch {
            return Some(i);
        }
    }
    None
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ─── Static OOXML Templates ─────────────────────────────────────────────

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
</Types>"#;

const RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

fn generate_document_rels(links: &[(String, String)]) -> String {
    let mut rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>"#);

    // Add hyperlink relationships
    for (rid, url) in links {
        rels.push_str(&format!(
            r#"
  <Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{}" TargetMode="External"/>"#,
            xml_escape(url)
        ));
    }

    rels.push_str("\n</Relationships>");
    rels
}

fn generate_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI" w:cs="Segoe UI"/>
        <w:sz w:val="22"/>
        <w:szCs w:val="22"/>
        <w:lang w:val="fr-FR"/>
      </w:rPr>
    </w:rPrDefault>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:after="160" w:line="259" w:lineRule="auto"/>
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr><w:keepNext/><w:spacing w:before="480" w:after="200"/><w:outlineLvl w:val="0"/>
      <w:pBdr><w:bottom w:val="single" w:sz="4" w:space="4" w:color="1B3A6B"/></w:pBdr>
    </w:pPr>
    <w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:b/><w:sz w:val="36"/><w:color w:val="1B3A6B"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:pPr><w:keepNext/><w:spacing w:before="360" w:after="160"/><w:outlineLvl w:val="1"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:b/><w:sz w:val="30"/><w:color w:val="2E5EA0"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:pPr><w:keepNext/><w:spacing w:before="240" w:after="120"/><w:outlineLvl w:val="2"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:b/><w:sz w:val="26"/><w:color w:val="4472C4"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading4">
    <w:name w:val="heading 4"/>
    <w:pPr><w:keepNext/><w:spacing w:before="200" w:after="80"/><w:outlineLvl w:val="3"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:b/><w:i/><w:sz w:val="24"/><w:color w:val="4472C4"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading5">
    <w:name w:val="heading 5"/>
    <w:pPr><w:keepNext/><w:spacing w:before="160" w:after="60"/><w:outlineLvl w:val="4"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:b/><w:sz w:val="22"/><w:color w:val="5B9BD5"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading6">
    <w:name w:val="heading 6"/>
    <w:pPr><w:keepNext/><w:spacing w:before="120" w:after="40"/><w:outlineLvl w:val="5"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="Segoe UI" w:hAnsi="Segoe UI"/><w:b/><w:i/><w:sz w:val="20"/><w:color w:val="5B9BD5"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListBullet">
    <w:name w:val="List Bullet"/>
    <w:pPr><w:spacing w:after="80"/><w:ind w:left="720" w:hanging="360"/></w:pPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListNumber">
    <w:name w:val="List Number"/>
    <w:pPr><w:spacing w:after="80"/><w:ind w:left="720" w:hanging="360"/></w:pPr>
  </w:style>
  <w:style w:type="table" w:styleId="TableGrid">
    <w:name w:val="Table Grid"/>
    <w:tblPr>
      <w:tblBorders>
        <w:top w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>
        <w:left w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>
        <w:bottom w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>
        <w:right w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>
        <w:insideH w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>
        <w:insideV w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/>
      </w:tblBorders>
    </w:tblPr>
  </w:style>
</w:styles>"#
        .to_string()
}

const NUMBERING_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="&#x2022;"/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr>
      <w:rPr><w:rFonts w:ascii="Symbol" w:hAnsi="Symbol" w:hint="default"/></w:rPr>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:start w:val="1"/>
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="&#x25E6;"/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="1080" w:hanging="360"/></w:pPr>
      <w:rPr><w:rFonts w:ascii="Courier New" w:hAnsi="Courier New" w:hint="default"/></w:rPr>
    </w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
  <w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>
</w:numbering>"#;

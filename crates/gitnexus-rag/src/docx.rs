//! Extractor for Microsoft Word .docx (Office Open XML / OOXML) files.
//!
//! Reads `word/document.xml` from the zip container and emits a markdown-like
//! string: `w:pStyle` values starting with "Heading"/"Titre"/"Title" become
//! `#`-prefixed headings, everything else is flattened to paragraphs. Tables
//! are walked into and each cell is emitted as its own paragraph (a pragmatic
//! choice — we're feeding a GraphRAG chunker, not a Word viewer).
//!
//! The produced markdown is then handed to [`crate::chunker::chunk_markdown`]
//! so we reuse the existing header-driven splitter.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Extract the body of a `.docx` file as markdown.
pub fn docx_to_markdown(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("open docx: {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("not a valid .docx (zip) file: {}", path.display()))?;

    let mut doc_xml = String::new();
    {
        let mut entry = archive
            .by_name("word/document.xml")
            .with_context(|| format!("missing word/document.xml in {}", path.display()))?;
        entry.read_to_string(&mut doc_xml)?;
    }

    parse_document_xml(&doc_xml)
}

/// Parse an OOXML `word/document.xml` body into a markdown string.
///
/// Separated from [`docx_to_markdown`] so it can be unit-tested without
/// constructing a real zip file.
fn parse_document_xml(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut out = String::new();
    let mut buf: Vec<u8> = Vec::new();

    let mut para_text = String::new();
    let mut heading_level: Option<u8> = None;
    let mut in_t = false;
    let mut in_body = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "xml parse error at byte {}: {}",
                    reader.buffer_position(),
                    e
                ))
            }
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"w:body" => in_body = true,
                    b"w:t" if in_body => in_t = true,
                    b"w:pStyle" if in_body => {
                        if let Some(level) = heading_level_from_attrs(&e) {
                            heading_level = Some(level);
                        }
                    }
                    _ => {}
                }
            }

            Ok(Event::Empty(e)) => {
                // Self-closing tags: w:pStyle, w:br, etc.
                let name = e.name();
                match name.as_ref() {
                    b"w:pStyle" if in_body => {
                        if let Some(level) = heading_level_from_attrs(&e) {
                            heading_level = Some(level);
                        }
                    }
                    b"w:br" if in_body => {
                        // Soft line break within a paragraph
                        para_text.push('\n');
                    }
                    b"w:tab" if in_body => {
                        para_text.push('\t');
                    }
                    _ => {}
                }
            }

            Ok(Event::End(e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"w:body" => in_body = false,
                    b"w:t" => in_t = false,
                    b"w:p" if in_body => {
                        flush_paragraph(&mut out, &mut para_text, &mut heading_level);
                    }
                    // Also flush on cell boundary so a cell with no </w:p> doesn't leak.
                    // (In well-formed docx every cell has at least one w:p — but be defensive.)
                    _ => {}
                }
            }

            Ok(Event::Text(e)) => {
                if in_t && in_body {
                    if let Ok(s) = std::str::from_utf8(e.as_ref()) {
                        // Best-effort XML entity unescape (&amp; → &, &lt; → <, …).
                        // If quick_xml rejects the input, fall back to the raw text.
                        match quick_xml::escape::unescape(s) {
                            Ok(cow) => para_text.push_str(&cow),
                            Err(_) => para_text.push_str(s),
                        }
                    }
                }
            }

            Ok(Event::CData(e)) => {
                if in_t && in_body {
                    if let Ok(s) = std::str::from_utf8(e.as_ref()) {
                        para_text.push_str(s);
                    }
                }
            }

            Ok(_) => {}
        }
        buf.clear();
    }

    // Final flush in case the document ends without a closing body event
    // (shouldn't happen for valid OOXML, but be defensive).
    flush_paragraph(&mut out, &mut para_text, &mut heading_level);

    Ok(out)
}

fn flush_paragraph(out: &mut String, para_text: &mut String, heading_level: &mut Option<u8>) {
    let trimmed = para_text.trim();
    if !trimmed.is_empty() {
        if let Some(level) = *heading_level {
            for _ in 0..level {
                out.push('#');
            }
            out.push(' ');
        }
        out.push_str(trimmed);
        out.push_str("\n\n");
    }
    para_text.clear();
    *heading_level = None;
}

/// Extract the heading level from a `<w:pStyle w:val="...">` element, if its
/// style name matches a known heading convention.
fn heading_level_from_attrs(e: &quick_xml::events::BytesStart) -> Option<u8> {
    for attr_res in e.attributes() {
        let Ok(attr) = attr_res else { continue };
        if attr.key.as_ref() == b"w:val" {
            let val = std::str::from_utf8(attr.value.as_ref()).ok()?;
            return detect_heading_level(val);
        }
    }
    None
}

fn detect_heading_level(style: &str) -> Option<u8> {
    let s = style.trim();
    let lower = s.to_ascii_lowercase();
    // English (Heading1, Heading 1, heading1), French (Titre1, Titre 1).
    let is_heading =
        lower.starts_with("heading") || lower.starts_with("titre") || lower.starts_with("title");
    if !is_heading {
        return None;
    }
    // Extract the first run of digits from the style name.
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        // "Title" with no number → H1
        return Some(1);
    }
    digits.parse::<u8>().ok().map(|n| n.clamp(1, 6))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap_body(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
{body}
</w:body>
</w:document>"#
        )
    }

    #[test]
    fn parses_simple_paragraph() {
        let body = r#"
<w:p><w:r><w:t>Hello world</w:t></w:r></w:p>
"#;
        let md = parse_document_xml(&wrap_body(body)).unwrap();
        assert_eq!(md.trim(), "Hello world");
    }

    #[test]
    fn parses_heading_and_body() {
        let body = r#"
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Intro</w:t></w:r></w:p>
<w:p><w:r><w:t>Body text here.</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Sub</w:t></w:r></w:p>
<w:p><w:r><w:t>More content.</w:t></w:r></w:p>
"#;
        let md = parse_document_xml(&wrap_body(body)).unwrap();
        assert!(md.contains("# Intro"), "missing H1: {md}");
        assert!(md.contains("Body text here."));
        assert!(md.contains("## Sub"), "missing H2: {md}");
        assert!(md.contains("More content."));
    }

    #[test]
    fn parses_french_titre_style() {
        // French locale of Word uses "Titre1", "Titre 1", etc.
        let body = r#"
<w:p><w:pPr><w:pStyle w:val="Titre 2"/></w:pPr><w:r><w:t>Calcul des baremes</w:t></w:r></w:p>
"#;
        let md = parse_document_xml(&wrap_body(body)).unwrap();
        assert!(
            md.contains("## Calcul des baremes"),
            "expected French H2, got: {md}"
        );
    }

    #[test]
    fn walks_into_table_cells() {
        // First w:body element is a table (common pattern in Alise docs).
        // Each cell's paragraph must be emitted.
        let body = r#"
<w:tbl>
  <w:tr>
    <w:tc><w:p><w:r><w:t>Cell A</w:t></w:r></w:p></w:tc>
    <w:tc><w:p><w:r><w:t>Cell B</w:t></w:r></w:p></w:tc>
  </w:tr>
</w:tbl>
"#;
        let md = parse_document_xml(&wrap_body(body)).unwrap();
        assert!(md.contains("Cell A"), "missing cell A: {md}");
        assert!(md.contains("Cell B"), "missing cell B: {md}");
    }

    #[test]
    fn empty_paragraphs_dont_crash() {
        let body = r#"
<w:p></w:p>
<w:p><w:r></w:r></w:p>
<w:p><w:r><w:t>Not empty.</w:t></w:r></w:p>
"#;
        let md = parse_document_xml(&wrap_body(body)).unwrap();
        assert_eq!(md.trim(), "Not empty.");
    }

    #[test]
    fn concatenates_multiple_runs_in_paragraph() {
        // Word often splits a single logical sentence across many <w:r> runs
        // (e.g., when formatting changes mid-sentence).
        let body = r#"
<w:p>
  <w:r><w:t xml:space="preserve">The </w:t></w:r>
  <w:r><w:t xml:space="preserve">quick </w:t></w:r>
  <w:r><w:t>fox</w:t></w:r>
</w:p>
"#;
        let md = parse_document_xml(&wrap_body(body)).unwrap();
        assert!(md.contains("The quick fox"), "runs not concatenated: {md}");
    }

    #[test]
    fn self_closing_break_becomes_newline() {
        let body = r#"
<w:p><w:r><w:t>Line 1</w:t><w:br/><w:t>Line 2</w:t></w:r></w:p>
"#;
        let md = parse_document_xml(&wrap_body(body)).unwrap();
        assert!(md.contains("Line 1"));
        assert!(md.contains("Line 2"));
    }

    #[test]
    fn heading_level_detection() {
        assert_eq!(detect_heading_level("Heading1"), Some(1));
        assert_eq!(detect_heading_level("Heading 2"), Some(2));
        assert_eq!(detect_heading_level("heading3"), Some(3));
        assert_eq!(detect_heading_level("Titre 4"), Some(4));
        assert_eq!(detect_heading_level("Title"), Some(1));
        assert_eq!(detect_heading_level("BodyText"), None);
        assert_eq!(detect_heading_level("Normal"), None);
        // Clamped to 1..=6
        assert_eq!(detect_heading_level("Heading9"), Some(6));
    }
}

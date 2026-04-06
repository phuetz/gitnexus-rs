//! Text chunking strategies for different document types.

use crate::DocChunk;
use anyhow::Result;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};

/// Chunk a Markdown document intelligently based on Headers.
///
/// Each heading starts a new chunk. Content between headings is accumulated
/// into the current chunk. Code blocks and inline code are preserved.
pub fn chunk_markdown(source_path: &str, content: &str) -> Result<Vec<DocChunk>> {
    let parser = Parser::new(content);
    let mut chunks = Vec::new();

    let mut current_title = String::from("Document Start");
    let mut current_content = String::new();
    let mut index = 0;
    let mut in_heading = false;
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading(_, _, _)) => {
                // Save accumulated content as a chunk
                if !current_content.trim().is_empty() {
                    chunks.push(DocChunk {
                        source_path: source_path.to_string(),
                        title: current_title.clone(),
                        content: current_content.trim().to_string(),
                        index,
                    });
                    index += 1;
                    current_content.clear();
                }
                current_title.clear();
                in_heading = true;
            }
            Event::End(Tag::Heading(_, _, _)) => {
                in_heading = false;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                current_content.push_str("\n```");
                if let CodeBlockKind::Fenced(lang) = kind {
                    current_content.push_str(&lang);
                }
                current_content.push('\n');
            }
            Event::End(Tag::CodeBlock(_)) => {
                in_code_block = false;
                current_content.push_str("```\n");
            }
            Event::Text(text) => {
                if in_heading {
                    current_title.push_str(&text);
                } else if in_code_block {
                    // Preserve code block text verbatim (no trailing space)
                    current_content.push_str(&text);
                } else {
                    current_content.push_str(&text);
                }
            }
            Event::Code(text) => {
                current_content.push('`');
                current_content.push_str(&text);
                current_content.push_str("` ");
            }
            Event::SoftBreak | Event::HardBreak => {
                current_content.push('\n');
            }
            _ => {}
        }
    }

    // Push the last chunk
    if !current_content.trim().is_empty() {
        chunks.push(DocChunk {
            source_path: source_path.to_string(),
            title: current_title.clone(),
            content: current_content.trim().to_string(),
            index,
        });
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_chunking() {
        let md = "
# Intro
This is the intro.

## Details
Here are the details.
        ";

        let chunks = chunk_markdown("test.md", md).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].title, "Intro");
        assert!(chunks[0].content.contains("This is the intro."));
        assert_eq!(chunks[1].title, "Details");
        assert!(chunks[1].content.contains("Here are the details."));
    }

    #[test]
    fn test_heading_with_emphasis() {
        let md = "# Hello *World*\nSome content here.";
        let chunks = chunk_markdown("test.md", md).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].title, "Hello World");
        assert!(chunks[0].content.contains("Some content here."));
    }

    #[test]
    fn test_code_block_preserved() {
        let md = "# API\n\n```rust\nfn main() {}\n```\n";
        let chunks = chunk_markdown("test.md", md).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("fn main()"));
        assert!(chunks[0].content.contains("```rust"));
        assert!(chunks[0].content.contains("```"));
    }

    #[test]
    fn test_empty_section_skipped() {
        let md = "# First\nContent.\n\n# Empty\n\n# Third\nMore content.";
        let chunks = chunk_markdown("test.md", md).unwrap();
        // Empty section between First and Third should be skipped
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| !c.content.trim().is_empty()));
    }

    #[test]
    fn test_inline_code() {
        let md = "# Docs\nUse `foo()` to start.";
        let chunks = chunk_markdown("test.md", md).unwrap();
        assert!(chunks[0].content.contains("`foo()`"));
    }
}

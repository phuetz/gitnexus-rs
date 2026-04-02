//! Markdown to HTML conversion and related helpers.

/// Convert Markdown content to HTML (basic, no external dependencies).
pub(super) fn markdown_to_html(md: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();
    let mut in_table = false;
    let mut table_has_body = false;
    let mut in_list = false;
    let mut in_ordered_list = false;

    for line in md.lines() {
        // Strip GNX anchor comments (used by LLM enrichment, not for display)
        if line.trim().starts_with("<!-- GNX:") {
            continue;
        }

        // Handle HTML comments (pass through as invisible)
        if line.trim().starts_with("<!--") && line.trim().ends_with("-->") {
            html.push_str(line);
            html.push('\n');
            continue;
        }

        // Handle <details>/<summary> blocks (pass through as HTML)
        if line.trim_start().starts_with("<details>")
            || line.trim_start().starts_with("<details ")
            || line.trim_start().starts_with("</details>")
            || line.trim_start().starts_with("<summary>")
            || line.trim_start().starts_with("<summary ")
            || line.trim_start().starts_with("</summary>")
        {
            html.push_str(line);
            html.push('\n');
            continue;
        }

        // Code fences
        if line.starts_with("```") {
            if in_code_block {
                // Close code block
                if code_lang == "mermaid" {
                    html.push_str(&format!(
                        "<pre><code class=\"language-mermaid\">{}</code></pre>\n",
                        html_escape(&code_content)
                    ));
                } else {
                    html.push_str(&format!(
                        "<pre><code class=\"language-{}\">{}</code></pre>\n",
                        code_lang,
                        html_escape(&code_content)
                    ));
                }
                code_content.clear();
                in_code_block = false;
            } else {
                // Close any open list before a code block
                if in_list {
                    html.push_str("</ul>\n");
                    in_list = false;
                }
                if in_ordered_list {
                    html.push_str("</ol>\n");
                    in_ordered_list = false;
                }
                code_lang = line.trim_start_matches('`').trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_content.push_str(line);
            code_content.push('\n');
            continue;
        }

        // Tables
        if line.contains('|') && line.trim().starts_with('|') {
            // Separator row (e.g., |---|---|)
            if line.replace('|', "").replace('-', "").replace(' ', "").replace(':', "").is_empty() {
                // Mark that we should switch from thead to tbody
                if in_table {
                    html.push_str("</thead><tbody>\n");
                    table_has_body = true;
                }
                continue;
            }
            if !in_table {
                // Close any open list
                if in_list {
                    html.push_str("</ul>\n");
                    in_list = false;
                }
                if in_ordered_list {
                    html.push_str("</ol>\n");
                    in_ordered_list = false;
                }
                html.push_str("<table>\n<thead>\n");
                in_table = true;
                table_has_body = false;
            }
            let cells: Vec<&str> = line
                .split('|')
                .filter(|s| !s.trim().is_empty())
                .collect();
            let tag = if table_has_body { "td" } else { "th" };
            html.push_str("<tr>");
            for cell in cells {
                html.push_str(&format!(
                    "<{tag}>{}</{tag}>",
                    inline_md(cell.trim())
                ));
            }
            html.push_str("</tr>\n");
            continue;
        } else if in_table {
            if table_has_body {
                html.push_str("</tbody></table>\n");
            } else {
                html.push_str("</thead></table>\n");
            }
            in_table = false;
            table_has_body = false;
        }

        // Headings
        if line.starts_with("### ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!("<h3>{}</h3>\n", inline_md(&line[4..])));
            continue;
        }
        if line.starts_with("## ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!("<h2>{}</h2>\n", inline_md(&line[3..])));
            continue;
        }
        if line.starts_with("# ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!("<h1>{}</h1>\n", inline_md(&line[2..])));
            continue;
        }

        // Horizontal rule
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str("<hr>\n");
            continue;
        }

        // Unordered lists
        if line.starts_with("- ") || line.starts_with("* ") {
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>\n", inline_md(&line[2..])));
            continue;
        }
        // Indented sub-items (2 or 4 spaces + dash)
        if (line.starts_with("  - ") || line.starts_with("    - ")) && in_list {
            let content = line.trim_start().trim_start_matches("- ");
            html.push_str(&format!("<li style=\"margin-left:16px\">{}</li>\n", inline_md(content)));
            continue;
        }

        // Ordered lists
        if !line.is_empty() {
            let maybe_ol = trimmed.split_once(". ");
            if let Some((num_part, rest)) = maybe_ol {
                if num_part.chars().all(|c| c.is_ascii_digit()) {
                    if in_list { html.push_str("</ul>\n"); in_list = false; }
                    if !in_ordered_list {
                        html.push_str("<ol>\n");
                        in_ordered_list = true;
                    }
                    html.push_str(&format!("<li>{}</li>\n", inline_md(rest)));
                    continue;
                }
            }
        }

        // Callouts: > [!NOTE], > [!TIP], > [!WARNING], > [!DANGER]
        if line.starts_with("> [!") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            let callout_type = if line.contains("[!NOTE]") { "note" }
                else if line.contains("[!TIP]") { "tip" }
                else if line.contains("[!WARNING]") { "warning" }
                else if line.contains("[!DANGER]") { "danger" }
                else { "note" };
            let icon = match callout_type {
                "tip" => "\u{1f4a1}",
                "warning" => "\u{26a0}\u{fe0f}",
                "danger" => "\u{1f534}",
                _ => "\u{2139}\u{fe0f}",
            };
            let text = line.trim_start_matches("> ").trim_start_matches("[!NOTE]")
                .trim_start_matches("[!TIP]").trim_start_matches("[!WARNING]")
                .trim_start_matches("[!DANGER]").trim();
            html.push_str(&format!(
                "<div class=\"callout callout-{}\">\
                 <span class=\"callout-icon\">{}</span>\
                 <div class=\"callout-content\">{}</div>\
                 </div>\n",
                callout_type, icon, inline_md(text)
            ));
            continue;
        }

        // Blockquotes
        if line.starts_with("> ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!(
                "<blockquote>{}</blockquote>\n",
                inline_md(&line[2..])
            ));
            continue;
        }

        // Empty lines close lists
        if line.trim().is_empty() {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            continue;
        }

        // Paragraph (default)
        if in_list { html.push_str("</ul>\n"); in_list = false; }
        if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
        html.push_str(&format!("<p>{}</p>\n", inline_md(line)));
    }

    // Close any open blocks
    if in_table {
        if table_has_body {
            html.push_str("</tbody></table>\n");
        } else {
            html.push_str("</thead></table>\n");
        }
    }
    if in_list {
        html.push_str("</ul>\n");
    }
    if in_ordered_list {
        html.push_str("</ol>\n");
    }

    html
}

/// Process inline Markdown formatting: bold, italic, code, links.
pub(super) fn inline_md(text: &str) -> String {
    let mut s = html_escape(text);

    // Bold: **text**
    loop {
        if let Some(start) = s.find("**") {
            if let Some(end) = s[start + 2..].find("**") {
                let bold_text = s[start + 2..start + 2 + end].to_string();
                s = format!(
                    "{}<strong>{}</strong>{}",
                    &s[..start],
                    bold_text,
                    &s[start + 2 + end + 2..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Italic: *text* (but not inside <strong> tags already processed)
    // Simple approach: match single * not preceded/followed by *
    loop {
        // Find a lone * that is not part of **
        let bytes = s.as_bytes();
        let mut start_pos = None;
        for i in 0..bytes.len() {
            if bytes[i] == b'*' {
                let prev_star = i > 0 && bytes[i - 1] == b'*';
                let next_star = i + 1 < bytes.len() && bytes[i + 1] == b'*';
                if !prev_star && !next_star {
                    start_pos = Some(i);
                    break;
                }
            }
        }
        if let Some(start) = start_pos {
            // Find matching closing *
            let rest = &s[start + 1..];
            let mut end_pos = None;
            let rest_bytes = rest.as_bytes();
            for i in 0..rest_bytes.len() {
                if rest_bytes[i] == b'*' {
                    let prev_star = i > 0 && rest_bytes[i - 1] == b'*';
                    let next_star = i + 1 < rest_bytes.len() && rest_bytes[i + 1] == b'*';
                    if !prev_star && !next_star {
                        end_pos = Some(i);
                        break;
                    }
                }
            }
            if let Some(end) = end_pos {
                let italic_text = s[start + 1..start + 1 + end].to_string();
                s = format!(
                    "{}<em>{}</em>{}",
                    &s[..start],
                    italic_text,
                    &s[start + 1 + end + 1..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Inline code: `text`
    loop {
        if let Some(start) = s.find('`') {
            if let Some(end) = s[start + 1..].find('`') {
                let code_text = s[start + 1..start + 1 + end].to_string();
                s = format!(
                    "{}<code>{}</code>{}",
                    &s[..start],
                    code_text,
                    &s[start + 1 + end + 1..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Links: [text](url) - after HTML escaping, parens are still literal
    // We need to match the pattern carefully
    loop {
        if let Some(bracket_start) = s.find('[') {
            if let Some(bracket_end) = s[bracket_start..].find("](") {
                let abs_bracket_end = bracket_start + bracket_end;
                let link_text = &s[bracket_start + 1..abs_bracket_end];
                let after_paren = &s[abs_bracket_end + 2..];
                if let Some(paren_end) = after_paren.find(')') {
                    let url = &after_paren[..paren_end];
                    // Transform .md links to JavaScript page navigation for HTML site
                    let replacement = if url.contains(".md") {
                        // Handle anchors: ./modules/file.md#ENTITY → page='modules/file', anchor='ENTITY'
                        let (md_part, anchor) = if let Some(hash_idx) = url.find('#') {
                            (&url[..hash_idx], Some(&url[hash_idx + 1..]))
                        } else {
                            (url, None)
                        };
                        let page_id = md_part.trim_start_matches("./")
                            .trim_end_matches(".md");
                        if let Some(anchor_id) = anchor {
                            // Navigate to page AND scroll to + open the entity details
                            format!(
                                "<a href=\"#\" onclick=\"showPage('{}'); setTimeout(function(){{ var el=document.getElementById('{}'); if(el){{ el.open=true; el.scrollIntoView({{behavior:'smooth'}}); }} }}, 100); return false;\">{}</a>",
                                page_id, anchor_id, link_text
                            )
                        } else {
                            format!(
                                "<a href=\"#\" onclick=\"showPage('{}'); return false;\">{}</a>",
                                page_id, link_text
                            )
                        }
                    } else {
                        format!("<a href=\"{}\">{}</a>", url, link_text)
                    };
                    s = format!(
                        "{}{}{}",
                        &s[..bracket_start],
                        replacement,
                        &after_paren[paren_end + 1..]
                    );
                    continue;
                }
            }
        }
        break;
    }

    s
}

/// Escape HTML special characters.
pub(super) fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Extract the first `# Title` from Markdown content.
pub(super) fn extract_title_from_md(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with("# ") {
            return Some(line[2..].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_md_bold() {
        let result = inline_md("This is **bold** text");
        assert!(result.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_inline_md_code() {
        let result = inline_md("Use `code` here");
        assert!(result.contains("<code>code</code>"));
    }

    #[test]
    fn test_inline_md_link() {
        let result = inline_md("See [docs](./overview.md)");
        assert!(result.contains("showPage"));
        assert!(result.contains("overview"));
    }

    #[test]
    fn test_markdown_to_html_headings() {
        let md = "# Title\n## Section\n### Subsection\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>"));
        assert!(html.contains("<h2>"));
        assert!(html.contains("<h3>"));
    }

    #[test]
    fn test_markdown_to_html_code_block() {
        let md = "```csharp\npublic void Test() {}\n```\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<pre>"));
        assert!(html.contains("language-csharp"));
    }

    #[test]
    fn test_markdown_to_html_callout() {
        let md = "> [!WARNING]\n> This is a warning\n";
        let html = markdown_to_html(md);
        assert!(html.contains("callout-warning"));
        assert!(html.contains("\u{26a0}\u{fe0f}"));
    }

    #[test]
    fn test_markdown_to_html_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
        assert!(html.contains("<td>"));
    }

    #[test]
    fn test_markdown_to_html_mermaid() {
        let md = "```mermaid\ngraph TD\n  A-->B\n```\n";
        let html = markdown_to_html(md);
        assert!(html.contains("language-mermaid"));
    }

    #[test]
    fn test_markdown_to_html_details() {
        let md = "<details>\n<summary>Click me</summary>\nContent here\n</details>\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<details>"));
        assert!(html.contains("<summary>"));
    }
}

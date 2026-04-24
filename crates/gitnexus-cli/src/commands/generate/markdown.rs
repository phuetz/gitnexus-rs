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
    let mut in_callout = false;

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

        // Handle <details>/<summary>/<div>/<a>/<img> blocks (pass through as HTML)
        if line.trim_start().starts_with("<details>")
            || line.trim_start().starts_with("<details ")
            || line.trim_start().starts_with("</details>")
            || line.trim_start().starts_with("<summary>")
            || line.trim_start().starts_with("<summary ")
            || line.trim_start().starts_with("</summary>")
            || line.trim_start().starts_with("<div")
            || line.trim_start().starts_with("</div")
            || line.trim_start().starts_with("<a ")
            || line.trim_start().starts_with("</a>")
            || line.trim_start().starts_with("<img")
        {
            html.push_str(line);
            html.push('\n');
            continue;
        }

        // Markdown image syntax: ![alt](url) → <img src="url" alt="alt">
        if let Some(img_html) = parse_md_image(line) {
            html.push_str(&img_html);
            html.push('\n');
            continue;
        }

        // Multiline Callouts (:::note, :::warning, etc.)
        if line.starts_with(":::") {
            if in_callout {
                // Close callout
                html.push_str("</div></div>\n");
                in_callout = false;
            } else {
                // Open callout
                let callout_type = line.trim_start_matches(":::").trim();
                let (css_class, icon, default_title) = match callout_type {
                    t if t.starts_with("tip") => ("tip", "lightbulb", "Tip"),
                    t if t.starts_with("warning") => ("warning", "triangle-alert", "Warning"),
                    t if t.starts_with("danger") => ("danger", "octagon-alert", "Danger"),
                    t if t.starts_with("info") => ("note", "info", "Info"),
                    t if t.starts_with("note") => ("note", "info", "Note"),
                    _ => ("note", "info", "Note"),
                };

                // Allow custom titles: :::note Custom Title
                let parts: Vec<&str> = line.trim_start_matches(":::").splitn(2, ' ').collect();
                let title = if parts.len() > 1 && !parts[1].trim().is_empty() {
                    parts[1].trim()
                } else {
                    default_title
                };

                html.push_str(&format!(
                    "<div class=\"callout callout-{}\">\
                     <div class=\"callout-icon\"><i data-lucide=\"{}\"></i></div>\
                     <div class=\"callout-content\">\
                     <div class=\"callout-title\">{}</div>\n",
                    css_class,
                    icon,
                    inline_md(title)
                ));
                in_callout = true;
            }
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
                    let escaped_lang = html_escape(&code_lang);
                    let escaped_content = html_escape(&code_content);
                    let line_count = code_content.lines().count();
                    if line_count > 25 {
                        let summary_label = if code_lang.is_empty() {
                            format!("{} lignes", line_count)
                        } else {
                            format!("{} · {} lignes", escaped_lang, line_count)
                        };
                        html.push_str(&format!(
                            "<details class=\"code-collapse\">\
                             <summary><i data-lucide=\"code-2\" style=\"width:12px;height:12px;vertical-align:middle;margin-right:6px;\"></i>{}</summary>\
                             <pre><code class=\"language-{}\">{}</code></pre>\
                             </details>\n",
                            summary_label, escaped_lang, escaped_content
                        ));
                    } else {
                        // The fence info string is user-controlled (`code_lang` is taken
                        // verbatim from ```<lang> in the source). Escape it before
                        // injecting into the `class` attribute or attackers can break
                        // out via `">` and inject arbitrary HTML.
                        html.push_str(&format!(
                            "<pre><code class=\"language-{}\">{}</code></pre>\n",
                            escaped_lang, escaped_content
                        ));
                    }
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
            if line.replace(['|', '-', ' ', ':'], "").is_empty() {
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
            // Drop only the leading and trailing empty tokens that come from
            // the surrounding `|` sentinels. Naively filtering all empties
            // (the previous behaviour) collapses intentionally blank middle
            // cells like `| a |  | c |` into `[a, c]`, which shifts every
            // subsequent column one position left and corrupts the rendered
            // table — the resulting <tr> has fewer <td>s than the header's
            // <th>s and breaks alignment in browser rendering.
            let cells: Vec<&str> = parse_md_table_row(line);
            let tag = if table_has_body { "td" } else { "th" };
            html.push_str("<tr>");
            for cell in cells {
                html.push_str(&format!("<{tag}>{}</{tag}>", inline_md(cell.trim())));
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
        if let Some(rest) = line.strip_prefix("### ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            html.push_str(&format!("<h3>{}</h3>\n", inline_md(rest)));
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>\n", inline_md(rest)));
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            html.push_str(&format!("<h1>{}</h1>\n", inline_md(rest)));
            continue;
        }

        // Horizontal rule
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            html.push_str("<hr>\n");
            continue;
        }

        // Unordered lists
        if line.starts_with("- ") || line.starts_with("* ") {
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
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
            html.push_str(&format!(
                "<li style=\"margin-left:16px\">{}</li>\n",
                inline_md(content)
            ));
            continue;
        }

        // Ordered lists
        if !line.is_empty() {
            let maybe_ol = trimmed.split_once(". ");
            if let Some((num_part, rest)) = maybe_ol {
                if num_part.chars().all(|c| c.is_ascii_digit()) {
                    if in_list {
                        html.push_str("</ul>\n");
                        in_list = false;
                    }
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
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            let callout_type = if line.contains("[!NOTE]") {
                "note"
            } else if line.contains("[!TIP]") {
                "tip"
            } else if line.contains("[!WARNING]") {
                "warning"
            } else if line.contains("[!DANGER]") {
                "danger"
            } else {
                "note"
            };
            let icon = match callout_type {
                "tip" => "lightbulb",
                "warning" => "triangle-alert",
                "danger" => "octagon-alert",
                _ => "info",
            };
            let text = line
                .trim_start_matches("> ")
                .trim_start_matches("[!NOTE]")
                .trim_start_matches("[!TIP]")
                .trim_start_matches("[!WARNING]")
                .trim_start_matches("[!DANGER]")
                .trim();
            html.push_str(&format!(
                "<div class=\"callout callout-{}\">\
                 <div class=\"callout-icon\"><i data-lucide=\"{}\"></i></div>\
                 <div class=\"callout-content\">\
                 <p>{}</p>\
                 </div>\
                 </div>\n",
                callout_type,
                icon,
                inline_md(text)
            ));
            continue;
        }

        // Blockquotes
        if let Some(rest) = line.strip_prefix("> ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            html.push_str(&format!("<blockquote>{}</blockquote>\n", inline_md(rest)));
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
        if in_list {
            html.push_str("</ul>\n");
            in_list = false;
        }
        if in_ordered_list {
            html.push_str("</ol>\n");
            in_ordered_list = false;
        }
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

    // Extract inline code spans up-front and replace each with a sentinel
    // placeholder containing no markdown-special characters. Without this
    // step the bold/italic passes below walk the *entire* string and chew
    // through asterisks inside `\``-fenced segments — `\`**not bold**\``
    // would render as `<code><strong>not bold</strong></code>` instead of
    // a literal `**not bold**`. Sentinels use a Private Use Area glyph that
    // cannot legally appear in source markdown after escaping.
    let mut code_spans: Vec<String> = Vec::new();
    let mut after_code = String::new();
    {
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '`' {
                after_code.push(c);
                continue;
            }
            // Try to find a matching closing backtick. If none, emit the
            // opening backtick literally and continue scanning.
            let mut span_content = String::new();
            let mut closed = false;
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '`' {
                    closed = true;
                    break;
                }
                span_content.push(nc);
            }
            if closed {
                let idx = code_spans.len();
                after_code.push_str(&format!("\u{E000}C{}\u{E000}", idx));
                code_spans.push(span_content);
            } else {
                after_code.push('`');
                after_code.push_str(&span_content);
            }
        }
    }
    s = after_code;

    // Bold: **text**
    while let Some(start) = s.find("**") {
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

    // Restore the code-span placeholders extracted at the top of this
    // function. The contents are wrapped in `<code>...</code>` here so that
    // any `**`/`*` characters inside the original backticks survive the
    // bold/italic passes above as literal text.
    if !code_spans.is_empty() {
        for (idx, content) in code_spans.iter().enumerate() {
            let placeholder = format!("\u{E000}C{}\u{E000}", idx);
            let replacement = format!("<code>{}</code>", content);
            s = s.replace(&placeholder, &replacement);
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
                        let page_id = md_part.trim_start_matches("./").trim_end_matches(".md");
                        // page_id and anchor_id are interpolated into JS string
                        // literals delimited by single quotes, then into HTML
                        // attributes. They must be safe for both contexts. We
                        // restrict to a conservative whitelist of identifier
                        // characters and reject anything that contains quotes,
                        // backslashes, control chars, or HTML metacharacters.
                        let safe_page_id = sanitize_id(page_id);
                        if let Some(anchor_id) = anchor {
                            let safe_anchor_id = sanitize_id(anchor_id);
                            // Navigate to page AND scroll to + open the entity details
                            format!(
                                "<a href=\"#\" onclick=\"showPage('{}'); setTimeout(function(){{ var el=document.getElementById('{}'); if(el){{ el.open=true; el.scrollIntoView({{behavior:'smooth'}}); }} }}, 100); return false;\">{}</a>",
                                safe_page_id, safe_anchor_id, link_text
                            )
                        } else {
                            format!(
                                "<a href=\"#\" onclick=\"showPage('{}'); return false;\">{}</a>",
                                safe_page_id, link_text
                            )
                        }
                    } else if is_safe_link_url(url) {
                        // Only allow http(s)://, mailto:, anchor (#…), or
                        // root-relative (/…) URLs into href. Anything else
                        // (e.g. javascript:) renders as plain text instead.
                        format!("<a href=\"{}\">{}</a>", html_escape(url), link_text)
                    } else {
                        format!("[{}]({})", link_text, html_escape(url))
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

/// Split a markdown pipe row into cells, dropping only the leading and
/// trailing empty tokens that come from the surrounding `|` sentinels.
/// Interior empty cells (e.g. `| a |  | c |`) are preserved so that they map
/// 1:1 to the header columns.
fn parse_md_table_row(line: &str) -> Vec<&str> {
    let raw: Vec<&str> = line.split('|').collect();
    let start = if raw.first().is_some_and(|s| s.trim().is_empty()) {
        1
    } else {
        0
    };
    let end = if raw.last().is_some_and(|s| s.trim().is_empty()) {
        raw.len().saturating_sub(1)
    } else {
        raw.len()
    };
    if start >= end {
        return Vec::new();
    }
    raw[start..end].to_vec()
}

/// Escape HTML special characters.
pub(super) fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('`', "&#96;")
}

/// Parse a standalone Markdown image `![alt](url)` line → `<div><img/></div>`.
/// Returns `None` if the line is not a standalone image.
pub(super) fn parse_md_image(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("![") {
        return None;
    }
    let alt_end = trimmed.find("](")?;
    let alt = &trimmed[2..alt_end];
    let rest = &trimmed[alt_end + 2..];
    let paren_end = rest.rfind(')')?;
    let url_part = rest[..paren_end]
        .splitn(2, '"')
        .next()
        .unwrap_or(&rest[..paren_end])
        .trim();
    Some(format!(
        "<div class=\"gnx-img-wrapper\" style=\"margin:12px 0;text-align:center;\"><img src=\"{}\" alt=\"{}\" style=\"max-width:100%;border-radius:6px;\"/></div>",
        html_escape(url_part), html_escape(alt)
    ))
}

/// Restrict an identifier to ASCII letters, digits and the small set of
/// characters used in our generated page IDs (`/`, `_`, `-`, `.`, `:`).
/// Anything else is dropped. Used to make values safe for both JavaScript
/// string literal context and HTML attribute context.
pub(super) fn sanitize_id(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':'))
        .collect()
}

/// Allow only safe URL schemes for direct `href` injection.
pub(super) fn is_safe_link_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('#') || trimmed.starts_with('/') {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
    {
        // Reject control chars and angle brackets even in allowed schemes.
        return !trimmed
            .chars()
            .any(|c| c.is_control() || c == '<' || c == '>' || c == '"');
    }
    false
}

/// Extract the first `# Title` from Markdown content.
pub(super) fn extract_title_from_md(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            return Some(rest.trim().to_string());
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
    fn test_inline_md_code_protects_asterisks() {
        // Bold/italic must NOT be applied inside inline code spans.
        let result = inline_md("Show `**not bold**` here");
        assert!(
            result.contains("<code>**not bold**</code>"),
            "expected literal asterisks inside <code>, got: {}",
            result
        );
        assert!(
            !result.contains("<strong>not bold</strong>"),
            "bold tag must not appear inside an inline code span: {}",
            result
        );
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
        assert!(html.contains("triangle-alert"));
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

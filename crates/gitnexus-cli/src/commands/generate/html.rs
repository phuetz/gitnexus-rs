//! HTML site generator.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use serde_json::json;
use tracing::info;

use gitnexus_core::graph::KnowledgeGraph;

use super::markdown::{markdown_to_html, extract_title_from_md, html_escape};

pub(super) fn generate_html_site(
    graph: &KnowledgeGraph,
    repo_path: &Path,
    docs_dir: &Path,
) -> Result<()> {
    if !docs_dir.exists() {
        return Err(anyhow::anyhow!(
            "No docs found. Run 'generate docs' first."
        ));
    }

    // 1. Collect all .md files from docs/.
    // Titles are HTML-escaped at extraction time because they flow into three
    // sinks where they would otherwise allow script injection: the sidebar
    // HTML (interpolated into `<a>` text), the `pages_json` object consumed
    // via `innerHTML` in `buildBreadcrumb`/`buildPageNav`, and the
    // `search_index` rendered via `innerHTML` in the search overlay. All
    // three consumers render titles as HTML, so escaping once at the source
    // closes every downstream vector.
    let mut pages: BTreeMap<String, (String, String)> = BTreeMap::new(); // id -> (escaped title, html_content)

    for entry in std::fs::read_dir(docs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            let content = std::fs::read_to_string(&path)?;
            let filename = path.file_stem().unwrap().to_string_lossy().to_string();
            let raw_title = extract_title_from_md(&content).unwrap_or_else(|| filename.clone());
            let title = html_escape(&raw_title);
            let html = markdown_to_html(&content);
            pages.insert(filename, (title, html));
        }
    }

    // Also read modules/ subdirectory
    let modules_dir = docs_dir.join("modules");
    if modules_dir.exists() {
        for entry in std::fs::read_dir(&modules_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let content = std::fs::read_to_string(&path)?;
                let filename = path.file_stem().unwrap().to_string_lossy().to_string();
                let raw_title = extract_title_from_md(&content).unwrap_or_else(|| filename.clone());
                let title = html_escape(&raw_title);
                let html = markdown_to_html(&content);
                pages.insert(format!("modules/{}", filename), (title, html));
            }
        }
    }

    // Also read processes/ subdirectory
    let processes_dir = docs_dir.join("processes");
    if processes_dir.exists() {
        for entry in std::fs::read_dir(&processes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let content = std::fs::read_to_string(&path)?;
                let filename = path.file_stem().unwrap().to_string_lossy().to_string();
                let raw_title = extract_title_from_md(&content).unwrap_or_else(|| filename.clone());
                let title = html_escape(&raw_title);
                let html = markdown_to_html(&content);
                pages.insert(format!("processes/{}", filename), (title, html));
            }
        }
    }

    if pages.is_empty() {
        return Err(anyhow::anyhow!(
            "No .md pages found in {}",
            docs_dir.display()
        ));
    }

    // 2. Build sidebar HTML with numbered sections
    let mut sidebar_html = String::new();

    // Group pages by category — force overview first
    let preferred_order = [
        "overview", "functional-guide", "project-health", "architecture",
        "getting-started", "deployment",
        "hotspots", "coupling", "ownership",
        "aspnet-controllers", "aspnet-routes", "aspnet-entities", "aspnet-data-model",
        "aspnet-views", "aspnet-services", "aspnet-external", "aspnet-entities-detail",
        "aspnet-seq-http", "aspnet-seq-data",
    ];

    let mut overview_pages: Vec<_> = pages
        .iter()
        .filter(|(k, _)| !k.starts_with("modules/") && !k.starts_with("processes/"))
        .collect();
    // Sort by preferred order, then alphabetically for unlisted
    overview_pages.sort_by_key(|(k, _)| {
        preferred_order.iter().position(|&p| k.as_str() == p).unwrap_or(999)
    });

    let module_pages: Vec<_> = pages
        .iter()
        .filter(|(k, _)| k.starts_with("modules/"))
        .collect();

    let first_page_id = overview_pages
        .first()
        .map(|(k, _)| k.as_str())
        .unwrap_or("");

    let mut section_num: usize = 1;

    sidebar_html.push_str(&format!("<div class=\"section-title\">{}. OVERVIEW</div>\n", section_num));
    for (sub_idx, (id, (title, _))) in overview_pages.iter().enumerate() {
        let active = if id.as_str() == first_page_id {
            " active"
        } else {
            ""
        };
        sidebar_html.push_str(&format!(
            "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\" class=\"{active}\">{section_num}.{sub_num} {title}</a>\n",
            sub_num = sub_idx + 1
        ));
    }

    // Controllers
    let ctrl_pages: Vec<_> = module_pages
        .iter()
        .filter(|(k, _)| k.contains("ctrl-"))
        .collect();
    if !ctrl_pages.is_empty() {
        section_num += 1;
        sidebar_html.push_str(&format!("<div class=\"section-title\">{}. CONTROLLERS</div>\n", section_num));
        for (sub_idx, (id, (title, _))) in ctrl_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    // Data Model
    let data_pages: Vec<_> = module_pages
        .iter()
        .filter(|(k, _)| k.contains("data-"))
        .collect();
    if !data_pages.is_empty() {
        section_num += 1;
        sidebar_html.push_str(&format!("<div class=\"section-title\">{}. DATA MODEL</div>\n", section_num));
        for (sub_idx, (id, (title, _))) in data_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    // Remaining module pages (services, UI, AJAX, etc.)
    let other_pages: Vec<_> = module_pages
        .iter()
        .filter(|(k, _)| !k.contains("ctrl-") && !k.contains("data-"))
        .collect();
    if !other_pages.is_empty() {
        section_num += 1;
        sidebar_html.push_str(&format!("<div class=\"section-title\">{}. MODULES</div>\n", section_num));
        for (sub_idx, (id, (title, _))) in other_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    // Process pages
    let process_pages: Vec<_> = pages
        .iter()
        .filter(|(k, _)| k.starts_with("processes/"))
        .collect();
    if !process_pages.is_empty() {
        section_num += 1;
        sidebar_html.push_str(&format!("<div class=\"section-title\">{}. BUSINESS PROCESSES</div>\n", section_num));
        for (sub_idx, (id, (title, _))) in process_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    // 3. Build pages JSON
    let pages_json: BTreeMap<String, serde_json::Value> = pages
        .iter()
        .map(|(id, (title, html))| {
            (
                id.clone(),
                serde_json::json!({
                    "title": title,
                    "html": html
                }),
            )
        })
        .collect();

    // 3b. Build PAGE_ORDER (ordered list of page IDs for prev/next navigation)
    let page_order: Vec<&String> = pages.keys().collect();
    let page_order_json = serde_json::to_string(&page_order)?;

    // 3c. Build SEARCH_INDEX (stripped text for full-text search)
    let search_index: Vec<serde_json::Value> = pages
        .iter()
        .map(|(id, (title, html))| {
            json!({
                "id": id,
                "title": title,
                "text": strip_html_tags(html)
            })
        })
        .collect();
    let search_index_json = serde_json::to_string(&search_index)?;

    // 4. Get project stats
    let node_count = graph.node_count();
    let edge_count = graph.relationship_count();
    let project_name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repository".to_string());
    let stats_str = format!(
        "{} nodes &middot; {} relations &middot; {} pages",
        node_count,
        edge_count,
        pages.len()
    );

    // 5. Get first page content.
    // Use `first_page_id` (computed from the preferred overview order) so the
    // initially-rendered page matches the sidebar's `active` link. Previously
    // this took `pages.values().next()`, i.e. the BTreeMap's alphabetically
    // first entry — e.g. "architecture" — while the sidebar highlighted
    // "overview" as active. The DOMContentLoaded handler does NOT call
    // showPage() on startup, so that mismatch was visible until the user
    // clicked something.
    let first_page_html = pages
        .get(first_page_id)
        .map(|(_, html)| html.as_str())
        .or_else(|| pages.values().next().map(|(_, html)| html.as_str()))
        .unwrap_or("<h1>Documentation</h1><p>No pages generated yet.</p>");

    // 6. Assemble HTML from template
    let index_json_path = docs_dir.join("_index.json");
    let index_json = if index_json_path.exists() {
        std::fs::read_to_string(&index_json_path)?
    } else {
        "{}".to_string()
    };

    let pages_json_str = serde_json::to_string(&pages_json)?;
    let final_html = build_html_template(
        &project_name,
        &stats_str,
        &sidebar_html,
        first_page_html,
        &pages_json_str,
        &page_order_json,
        &search_index_json,
        &index_json,
    );

    // 7. Check for local mermaid.min.js (offline support)
    let mermaid_path = docs_dir.join("mermaid.min.js");
    if !mermaid_path.exists() {
        println!("  {} For offline diagrams, download mermaid.min.js to {}", "TIP".cyan(), docs_dir.display());
    }

    // 8. Write output
    let out_path = docs_dir.join("index.html");
    std::fs::write(&out_path, &final_html)?;
    info!("Generated HTML documentation at {}", out_path.display());
    println!(
        "{} Generated HTML documentation: {}",
        "OK".green(),
        out_path.display()
    );

    Ok(())
}

/// Strip HTML tags from content, returning plain text for search indexing.
pub(super) fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        if c == '<' { in_tag = true; continue; }
        if c == '>' { in_tag = false; result.push(' '); continue; }
        if !in_tag { result.push(c); }
    }
    // Collapse whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build the complete self-contained HTML template.
#[allow(clippy::too_many_arguments)]
fn build_html_template(
    project_name: &str,
    stats: &str,
    sidebar_nav: &str,
    first_page_content: &str,
    pages_json: &str,
    page_order_json: &str,
    search_index_json: &str,
    index_json: &str,
) -> String {
    // `project_name` is derived from the OS folder name (`repo_path.file_name()`)
    // and `stats` is built from internal counts, but the folder name is
    // attacker-controllable when indexing untrusted repositories. Both values
    // are interpolated directly into HTML element bodies and the <title> tag,
    // so escape them to prevent script injection in the generated index.html.
    let project_name = super::markdown::html_escape(project_name);
    let stats = super::markdown::html_escape(stats);
    let project_name = project_name.as_str();
    let stats = stats.as_str();
    format!(
        r##"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{project_name} — Documentation</title>
  <script src="https://unpkg.com/lucide@latest"></script>
  <script src="mermaid.min.js" onerror="this.onerror=null;var s=document.createElement('script');s.src='https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js';s.onload=function(){{if(typeof mermaid!=='undefined'){{mermaid.initialize({{theme:'dark',startOnLoad:false,securityLevel:'loose'}});renderMermaid();}}}};document.head.appendChild(s);"></script>
  <link rel="stylesheet" href="hljs-dark.css" onerror="this.onerror=null;this.href='https://cdn.jsdelivr.net/npm/highlight.js@11/styles/github-dark.min.css'">
  <script src="hljs.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/core.min.js'"></script>
  <script src="hljs-csharp.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/csharp.min.js'"></script>
  <script src="hljs-js.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/javascript.min.js'"></script>
  <script src="hljs-xml.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/xml.min.js'"></script>
  <script src="hljs-sql.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/sql.min.js'"></script>
  <style>
    [data-theme="light"] .hljs {{ background: var(--bg-surface); }}
    :root {{
      --bg: #0f1117; --bg-surface: #161822; --bg-sidebar: #12141e;
      --text: #e8ecf4; --text-muted: #8690a5; --accent: #6aa1f8;
      --border: rgba(255,255,255,0.08);
    }}
    [data-theme="light"] {{
      --bg: #f8f9fc; --bg-surface: #ffffff; --bg-sidebar: #f0f2f7;
      --text: #1a1d26; --text-muted: #5a6275; --accent: #4a85e0;
      --border: rgba(0,0,0,0.08);
    }}
    * {{ margin:0; padding:0; box-sizing:border-box; }}
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
           background: var(--bg); color: var(--text); display:flex; height:100vh; }}
    .header {{ position:fixed; top:0; left:0; right:0; height:48px; background:var(--bg-sidebar);
              border-bottom:1px solid var(--border); display:flex; align-items:center;
              padding:0 20px; z-index:50; }}
    .header h1 {{ font-size:15px; color:var(--accent); }}
    .header .stats {{ margin-left:auto; font-size:11px; color:var(--text-muted); margin-right:80px; }}
    body {{ padding-top:48px; }}
    .sidebar {{ width:280px; background:var(--bg-sidebar); border-right:1px solid var(--border);
               overflow-y:auto; padding:16px 0; flex-shrink:0; margin-top:48px; height:calc(100vh - 48px); }}
    .sidebar h2 {{ font-size:14px; padding:8px 20px; color:var(--accent); }}
    .sidebar a {{ display:block; padding:6px 20px; color:var(--text-muted); text-decoration:none;
                 font-size:13px; border-left:3px solid transparent; transition: all 0.15s; }}
    .sidebar a:hover {{ color:var(--text); background:rgba(255,255,255,0.03); }}
    .sidebar a.active {{ color:var(--accent); border-left-color:var(--accent);
                        background:rgba(106,161,248,0.08); }}
    .sidebar .section-title {{ font-size:10px; text-transform:uppercase; letter-spacing:0.05em;
                              color:var(--text-muted); padding:16px 20px 4px; }}
    .main {{ flex:1; overflow-y:auto; padding:40px 60px; max-width:900px;
            transition: opacity 0.12s ease-out; }}
    .main h1 {{ font-size:28px; margin-bottom:8px; }}
    .main h2 {{ font-size:20px; margin:32px 0 12px; padding-bottom:8px;
               border-bottom:1px solid var(--border); }}
    .main h3 {{ font-size:16px; margin:24px 0 8px; }}
    .main p {{ line-height:1.7; margin:8px 0; }}
    .main table {{ width:100%; border-collapse:collapse; margin:16px 0; font-size:13px; }}
    .main th, .main td {{ padding:8px 12px; border:1px solid var(--border); text-align:left; }}
    .main th {{ background:var(--bg-sidebar); font-weight:600; }}
    .main code {{ background:var(--bg-sidebar); padding:2px 6px; border-radius:4px; font-size:12px;
                 font-family:'JetBrains Mono',monospace; }}
    .main pre {{ background:var(--bg-sidebar); padding:16px; border-radius:8px; overflow-x:auto;
                margin:12px 0; border:1px solid var(--border); }}
    .main pre code {{ background:none; padding:0; }}
    .main ul, .main ol {{ padding-left:24px; margin:8px 0; }}
    .main li {{ line-height:1.7; }}
    .main blockquote {{ border-left:3px solid var(--accent); padding:8px 16px; margin:12px 0;
                       color:var(--text-muted); background:rgba(106,161,248,0.05); border-radius:0 8px 8px 0; }}
    .toc {{ width:220px; padding:20px 16px; border-left:1px solid var(--border);
           overflow-y:auto; flex-shrink:0; position:sticky; top:0; margin-top:48px; height:calc(100vh - 48px); }}
    .toc h3 {{ font-size:11px; text-transform:uppercase; letter-spacing:0.05em;
              color:var(--text-muted); margin-bottom:12px; }}
    .toc a {{ display:block; font-size:12px; color:var(--text-muted); text-decoration:none;
             padding:3px 0; border-left:2px solid transparent; padding-left:8px; }}
    .toc a:hover {{ color:var(--accent); }}
    .toc a.depth-3 {{ padding-left:20px; }}
    .toc a.toc-active {{
      color: var(--accent);
      border-left-color: var(--accent);
      font-weight: 600;
    }}
    .theme-toggle {{ position:fixed; top:12px; right:16px; background:var(--bg-surface);
                    border:1px solid var(--border); border-radius:8px; padding:6px 12px;
                    color:var(--text-muted); cursor:pointer; font-size:12px; z-index:100; }}
    .mermaid {{ background:var(--bg-surface); border-radius:8px; padding:16px; margin:16px 0;
               border:1px solid var(--border); text-align:center; cursor: zoom-in; position: relative; }}
    .mermaid:hover::after {{ content: 'Click to zoom'; position: absolute; top: 8px; right: 8px; font-size: 10px; color: var(--text-muted); background: var(--bg); padding: 2px 6px; border-radius: 4px; border: 1px solid var(--border); }}
    .mermaid-modal {{
      display: none; position: fixed; inset: 0; z-index: 200;
      background: rgba(0,0,0,0.9); align-items: center; justify-content: center;
      padding: 40px; cursor: zoom-out;
    }}
    .mermaid-modal.open {{ display: flex; }}
    .mermaid-modal svg {{ max-width: 100%; max-height: 100%; transform-origin: center; transition: transform 0.2s; }}
    .search {{ padding:8px 16px; }}
    .search input {{ width:100%; padding:6px 10px; background:var(--bg); border:1px solid var(--border);
                    border-radius:6px; color:var(--text); font-size:12px; outline:none; }}
    .search input:focus {{ border-color:var(--accent); }}
    .hidden {{ display:none !important; }}
    .main details {{ margin:12px 0; border:1px solid var(--border); border-radius:8px;
                    padding:4px 12px; background:var(--bg-surface); }}
    .main details summary {{ cursor:pointer; font-weight:600; font-size:13px; color:var(--text-muted);
                            padding:8px 0; user-select:none; }}
    .main details summary:hover {{ color:var(--accent); }}
    .main details[open] summary {{ margin-bottom:4px; border-bottom:1px solid var(--border); padding-bottom:8px; }}
    .hljs-keyword {{ color: #c678dd; font-weight: 600; }}
    .hljs-string {{ color: #98c379; }}
    .hljs-comment {{ color: #7f848e; font-style: italic; }}
    .hljs-number {{ color: #d19a66; }}
    .hljs-function .hljs-title {{ color: #61afef; }}
    .hljs-built_in {{ color: #e5c07b; }}
    .hljs-type {{ color: #e5c07b; }}
    [data-theme="light"] .hljs-keyword {{ color: #8b3dba; }}
    [data-theme="light"] .hljs-string {{ color: #2e7d32; }}
    [data-theme="light"] .hljs-comment {{ color: #9e9e9e; }}
    [data-theme="light"] .hljs-number {{ color: #b5651d; }}
    [data-theme="light"] .hljs-function .hljs-title {{ color: #1565c0; }}
    .code-wrapper {{ position: relative; }}
    .copy-btn {{
      position: absolute; top: 8px; right: 8px;
      background: var(--bg-surface); border: 1px solid var(--border);
      border-radius: 6px; padding: 4px 8px; cursor: pointer;
      font-size: 11px; color: var(--text-muted);
      opacity: 0; transition: opacity 0.15s;
    }}
    .code-wrapper:hover .copy-btn {{ opacity: 1; }}
    .copy-btn.copied {{ color: var(--accent); }}
    .callout {{
      border-radius: 8px; padding: 16px 20px; margin: 20px 0;
      border-left: 4px solid; display: flex; gap: 12px;
      font-size: 14px;
    }}
    .callout-icon {{ flex-shrink: 0; margin-top: 2px; color: inherit; opacity: 0.8; }}
    .callout-icon svg {{ width: 20px; height: 20px; }}
    .callout-content {{ flex: 1; }}
    .callout-title {{ font-weight: 600; font-size: 14px; margin-bottom: 6px; letter-spacing: 0.02em; color: var(--text); }}
    .callout-content p {{ margin: 0; line-height: 1.6; color: var(--text-muted); }}
    .callout-content p + p {{ margin-top: 12px; }}
    .callout-note {{ background: rgba(106,161,248,0.08); border-color: var(--accent); }}
    .callout-tip {{ background: rgba(74,222,128,0.08); border-color: #4ade80; }}
    .callout-warning {{ background: rgba(251,191,36,0.08); border-color: #fbbf24; }}
    .callout-danger {{ background: rgba(248,113,113,0.08); border-color: #f87171; }}
    
    /* Dashboard Cards */
    .dashboard-grid {{
      display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 16px; margin: 24px 0;
    }}
    .stat-card {{
      background: var(--bg-surface); border: 1px solid var(--border);
      border-radius: 12px; padding: 20px; text-align: center;
      transition: transform 0.2s, border-color 0.2s;
    }}
    .stat-card:hover {{ transform: translateY(-2px); border-color: var(--accent); }}
    .stat-card i {{ color: var(--accent); margin-bottom: 12px; display: block; }}
    .stat-card .value {{ font-size: 28px; font-weight: 700; color: var(--text); display: block; }}
    .stat-card .label {{ font-size: 12px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }}
    
    /* Chat Widget */
    .chat-widget {{ position: fixed; bottom: 24px; right: 24px; z-index: 1000; font-family: var(--font); }}
    .chat-toggle {{
      width: 56px; height: 56px; border-radius: 28px; background: var(--accent); color: white;
      border: none; cursor: pointer; box-shadow: 0 4px 12px rgba(0,0,0,0.3);
      display: flex; align-items: center; justify-content: center; transition: transform 0.2s;
    }}
    .chat-toggle:hover {{ transform: scale(1.05); }}
    .chat-panel {{
      position: absolute; bottom: 72px; right: 0; width: 380px; height: 500px;
      background: var(--bg-surface); border: 1px solid var(--border); border-radius: 12px;
      box-shadow: 0 8px 24px rgba(0,0,0,0.2); display: none; flex-direction: column; overflow: hidden;
    }}
    .chat-panel.open {{ display: flex; }}
    .chat-header {{
      padding: 12px 16px; background: var(--bg-surface); border-bottom: 1px solid var(--border);
      display: flex; justify-content: space-between; align-items: center;
    }}
    .chat-header-title {{ display: flex; align-items: center; gap: 8px; font-weight: 600; font-size: 14px; }}
    .chat-close {{ background: none; border: none; color: var(--text-muted); cursor: pointer; }}
    .chat-messages {{ flex: 1; overflow-y: auto; padding: 16px; display: flex; flexDirection: column; gap: 12px; }}
    .message {{ padding: 10px 14px; border-radius: 12px; font-size: 13px; line-height: 1.5; max-width: 85%; }}
    .message.user {{ background: var(--accent); color: white; align-self: flex-end; border-bottom-right-radius: 2px; }}
    .message.assistant {{ background: var(--bg-3); color: var(--text); align-self: flex-start; border-bottom-left-radius: 2px; }}
    .message.system {{ background: transparent; color: var(--text-muted); align-self: center; text-align: center; font-style: italic; font-size: 12px; }}
    .chat-input-container {{ padding: 12px; border-top: 1px solid var(--border); display: flex; gap: 8px; align-items: flex-end; }}
    #chat-input {{
      flex: 1; background: var(--bg); border: 1px solid var(--border); border-radius: 8px;
      padding: 8px 12px; color: var(--text); font-size: 13px; resize: none; min-height: 36px; max-height: 120px;
      outline: none;
    }}
    #chat-input:focus {{ border-color: var(--accent); }}
    #chat-send {{
      width: 36px; height: 36px; border-radius: 18px; background: var(--accent); color: white;
      border: none; cursor: pointer; display: flex; align-items: center; justify-content: center;
    }}
    #chat-send:disabled {{ opacity: 0.5; cursor: not-allowed; }}

    .breadcrumb {{
      font-size: 12px; color: var(--text-muted); margin-bottom: 16px;
      display: flex; gap: 6px; align-items: center;
    }}
    .breadcrumb a {{ color: var(--text-muted); text-decoration: none; }}
    .breadcrumb a:hover {{ color: var(--accent); }}
    .breadcrumb .sep {{ color: var(--border); }}
    .page-nav {{
      display: flex; justify-content: space-between; padding: 24px 0;
      margin-top: 32px; border-top: 1px solid var(--border);
    }}
    .page-nav a {{
      display: flex; flex-direction: column; gap: 4px;
      text-decoration: none; color: var(--text-muted); font-size: 13px;
      padding: 8px 12px; border-radius: 8px; transition: background 0.15s;
      max-width: 45%;
    }}
    .page-nav a:hover {{ background: rgba(106,161,248,0.06); color: var(--accent); }}
    .page-nav .nav-label {{ font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; }}
    .page-nav .nav-title {{ font-weight: 600; color: var(--text); }}
    .page-nav .nav-next {{ text-align: right; margin-left: auto; }}
    .hamburger {{
      display: none; position: fixed; top: 12px; left: 12px;
      background: var(--bg-surface); border: 1px solid var(--border);
      border-radius: 8px; padding: 6px 10px; cursor: pointer;
      color: var(--text-muted); z-index: 60; font-size: 18px;
    }}
    .search-result {{
      display: block; padding: 10px 12px; border-radius: 8px;
      text-decoration: none; color: var(--text); transition: background 0.1s;
    }}
    .search-result:hover {{ background: rgba(106,161,248,0.08); }}
    .search-result-title {{ font-weight: 600; font-size: 13px; }}
    .search-result-snippet {{ font-size: 12px; color: var(--text-muted); margin-top: 4px; }}
    .search-result-snippet mark {{ background: rgba(106,161,248,0.3); color: var(--text); border-radius: 2px; padding: 0 2px; }}
    .search-empty {{ padding: 20px; text-align: center; color: var(--text-muted); font-size: 13px; }}
    .code-wrapper pre {{ counter-reset: line; }}
    .code-wrapper pre code .line {{ counter-increment: line; }}
    .code-wrapper pre code .line::before {{
      content: counter(line); display: inline-block; width: 3em;
      margin-right: 1em; text-align: right; color: var(--text-muted);
      opacity: 0.4; font-size: 12px; user-select: none;
    }}
    @media (max-width:900px) {{
      .hamburger {{ display: block; }}
      .sidebar {{ transform: translateX(-100%); transition: transform 0.25s ease; z-index: 55; position: fixed; height: 100vh; }}
      .sidebar.open {{ transform: translateX(0); box-shadow: 4px 0 20px rgba(0,0,0,0.3); }}
      .toc {{ display:none; }}
      .main {{ padding:20px; }}
    }}
    @media print {{
      .sidebar, .toc, .header, .theme-toggle, .copy-btn, .hamburger, .page-nav, .search {{ display: none !important; }}
      .main {{ margin: 0; padding: 20px; max-width: 100%; }}
      body {{ font-family: Georgia, serif; font-size: 11pt; color: #000; background: #fff; }}
      pre {{ border: 1px solid #ccc; page-break-inside: avoid; font-size: 9pt; }}
      h1, h2, h3 {{ page-break-after: avoid; color: #000; }}
      a {{ color: #000; text-decoration: underline; }}
      .callout {{ border: 1px solid #ccc; break-inside: avoid; }}
    }}
  </style>
</head>
<body>
  <button class="hamburger" onclick="toggleSidebar()">&#9776;</button>
  <div id="search-overlay" class="hidden"
    style="position:fixed;inset:0;z-index:100;background:rgba(0,0,0,0.6);display:flex;align-items:flex-start;justify-content:center;padding-top:15vh;">
    <div style="width:560px;max-width:90vw;background:var(--bg-surface);border:1px solid var(--border);border-radius:12px;overflow:hidden;box-shadow:0 8px 32px rgba(0,0,0,0.3);">
      <div style="padding:12px 16px;border-bottom:1px solid var(--border);">
        <input id="search-input" type="text" placeholder="Rechercher dans la documentation... (Ctrl+K)"
          style="width:100%;padding:8px 12px;background:var(--bg);border:1px solid var(--border);border-radius:8px;color:var(--text);font-size:14px;outline:none;">
      </div>
      <div id="search-results" style="max-height:400px;overflow-y:auto;padding:8px;"></div>
    </div>
  </div>
  <div id="mermaid-modal" class="mermaid-modal" onclick="closeMermaidModal()"></div>
  <header class="header">
    <h1>{project_name}</h1>
    <span class="stats">{stats}</span>
    <button class="theme-toggle" onclick="toggleTheme()">Theme</button>
  </header>
  <nav class="sidebar">
    <div class="search">
      <input type="text" placeholder="Filter pages..." oninput="filterPages(this.value)">
    </div>
    <div id="dynamic-sidebar"></div>
  </nav>
  <main class="main" id="content">
    {first_page_content}
  </main>
  <aside class="toc" id="toc">
    <h3>On this page</h3>
    <div id="toc-links"></div>
  </aside>

  <!-- Integrated Chat UI -->
  <div id="chat-widget" class="chat-widget">
    <button id="chat-toggle" class="chat-toggle" onclick="toggleChat()">
      <i data-lucide="message-square"></i>
      <span class="chat-badge" id="chat-badge" style="display:none"></span>
    </button>
    <div id="chat-panel" class="chat-panel">
      <div class="chat-header">
        <div class="chat-header-title">
          <i data-lucide="bot" style="width:16px;height:16px;"></i>
          <span>GitNexus Assistant</span>
        </div>
        <button class="chat-close" onclick="toggleChat()"><i data-lucide="x" style="width:16px;height:16px;"></i></button>
      </div>
      <div id="chat-messages" class="chat-messages">
        <div class="message system">
          Bonjour ! Je suis l'assistant GitNexus. Je peux répondre à vos questions sur ce projet en utilisant le graphe de connaissances.
        </div>
      </div>
      <div class="chat-input-container">
        <textarea id="chat-input" placeholder="Posez une question sur le code..." onkeydown="handleChatKey(event)"></textarea>
        <button id="chat-send" onclick="sendChatMessage()"><i data-lucide="send" style="width:16px;height:16px;"></i></button>
      </div>
    </div>
  </div>

  <script>
    const PAGES = {pages_json};
    const PAGE_ORDER = {page_order_json};
    const SEARCH_INDEX = {search_index_json};
    const INDEX_JSON = {index_json};
    let currentPage = null;

    function buildDynamicSidebar() {{
      const container = document.getElementById('dynamic-sidebar');
      if (!INDEX_JSON || !Array.isArray(INDEX_JSON)) {{
        container.innerHTML = `{sidebar_nav}`; // Fallback
        return;
      }}

      let html = '';
      INDEX_JSON.forEach((section, i) => {{
        if (section.children && section.children.length > 0) {{
          html += `<div class="section-title"><i data-lucide="${{section.icon || 'folder'}}" style="width:14px;height:14px;vertical-align:middle;margin-right:6px;margin-top:-2px;"></i>${{section.title.toUpperCase()}}</div>`;
          section.children.forEach(child => {{
            // Use path or ID to construct the data-page ID.
            const pageId = child.path ? child.path.replace('.md', '') : child.id;
            html += `<a href="#" data-page="${{pageId}}" onclick="showPage('${{pageId}}'); return false;"><i data-lucide="${{child.icon || 'file-text'}}" style="width:14px;height:14px;vertical-align:middle;margin-right:6px;margin-top:-2px;"></i>${{child.title}}</a>`;
          }});
        }} else {{
            const pageId = section.path ? section.path.replace('.md', '') : section.id;
            html += `<a href="#" data-page="${{pageId}}" onclick="showPage('${{pageId}}'); return false;"><i data-lucide="${{section.icon || 'file-text'}}" style="width:14px;height:14px;vertical-align:middle;margin-right:6px;margin-top:-2px;"></i>${{section.title}}</a>`;
        }}
      }});
      container.innerHTML = html;
    }}

    function showPage(id, anchor, skipHistory = false) {{
      const page = PAGES[id];
      if (!page) return;
      currentPage = id;
      
      if (!skipHistory) {{
        const url = "#" + id + (anchor ? "%23" + anchor : "");
        history.pushState({{id: id, anchor: anchor}}, "", url);
      }}

      const content = document.getElementById('content');
      content.style.opacity = '0';
      setTimeout(() => {{
        content.innerHTML = page.html;
        const breadcrumb = buildBreadcrumb(id, page.title);
        content.insertAdjacentHTML('afterbegin', breadcrumb);
        
        // Estimated reading time
        const wordCount = content.textContent.split(/\\s+/).length;
        const readTime = Math.max(1, Math.ceil(wordCount / 200));
        content.insertAdjacentHTML('afterbegin', `<div class="reading-time" style="font-size:12px; color:var(--text-muted); margin-bottom:16px;"><i data-lucide="clock" style="width:12px;height:12px;vertical-align:middle;margin-right:4px;"></i>~${{readTime}} min de lecture</div>`);

        // Feedback buttons
        const feedbackHtml = `
          <div class="page-feedback" style="margin-top:40px; padding-top:20px; border-top:1px solid var(--border); text-align:center;">
            <p style="font-size:13px; color:var(--text-muted); margin-bottom:10px;">Cette page est-elle utile ?</p>
            <div style="display:flex; gap:10px; justify-content:center;">
              <button class="feedback-btn" onclick="submitFeedback('${{id}}', true)" style="background:var(--bg-surface); border:1px solid var(--border); border-radius:6px; padding:6px 12px; cursor:pointer; display:flex; align-items:center; gap:6px;"><i data-lucide="thumbs-up" style="width:14px;height:14px;"></i> Oui</button>
              <button class="feedback-btn" onclick="submitFeedback('${{id}}', false)" style="background:var(--bg-surface); border:1px solid var(--border); border-radius:6px; padding:6px 12px; cursor:pointer; display:flex; align-items:center; gap:6px;"><i data-lucide="thumbs-down" style="width:14px;height:14px;"></i> Non</button>
            </div>
            <div id="feedback-thanks" style="display:none; color:#4ade80; font-size:12px; margin-top:8px;">Merci pour votre retour !</div>
          </div>
        `;
        content.insertAdjacentHTML('beforeend', feedbackHtml);
        
        content.insertAdjacentHTML('beforeend', buildPageNav(id));
        document.querySelectorAll('.sidebar a[data-page]').forEach(a => a.classList.remove('active'));
        const link = document.querySelector('.sidebar a[data-page="' + id + '"]');
        if (link) {{ link.classList.add('active'); link.scrollIntoView({{block:'nearest'}}); }}
        
        // Make `<details>` list items clickable if they look like paths
        document.querySelectorAll('details li').forEach(li => {{
            const text = li.textContent.trim();
            if (text.includes('/') && text.includes('.')) {{
                // Create a basic GitHub-like file icon
                li.innerHTML = `<i data-lucide="file-code" style="width:12px;height:12px;vertical-align:middle;margin-right:6px;opacity:0.7;"></i><span style="font-family:monospace; font-size:12px; cursor:copy;" onclick="navigator.clipboard.writeText('${{text}}'); this.style.color='var(--accent)'; setTimeout(()=>this.style.color='', 1000);" title="Click to copy path">${{text}}</span>`;
            }}
        }});

        buildToc();
        addCopyButtons();
        renderMermaid();
        if (typeof hljs !== 'undefined') {{
          document.querySelectorAll('pre code').forEach(block => {{
            if (!block.classList.contains('language-mermaid')) {{
              hljs.highlightElement(block);
            }}
          }});
        }}
        if (typeof lucide !== 'undefined') {{
          lucide.createIcons({{
            attrs: {{
              class: ["lucide-icon"]
            }}
          }});
        }}
        initScrollSpy();
        
        // Restore feedback state
        const feedbackState = localStorage.getItem('feedback_' + id);
        if (feedbackState) {{
            document.getElementById('feedback-thanks').style.display = 'block';
            document.getElementById('feedback-thanks').textContent = 'Vous avez déjà évalué cette page.';
            document.querySelectorAll('.feedback-btn').forEach(btn => btn.style.display = 'none');
        }}

        content.style.opacity = '1';
        if (anchor) {{
          setTimeout(() => {{
            const el = document.getElementById(anchor);
            if (el) {{ if (el.tagName === 'DETAILS') el.open = true; el.scrollIntoView({{behavior:'smooth'}}); }}
          }}, 150);
        }} else {{
          content.scrollTop = 0;
        }}
      }}, 100);
    }}

    function submitFeedback(id, isPositive) {{
        localStorage.setItem('feedback_' + id, isPositive ? 'yes' : 'no');
        document.getElementById('feedback-thanks').style.display = 'block';
        document.querySelectorAll('.feedback-btn').forEach(btn => btn.style.display = 'none');
    }}

    window.onpopstate = function(event) {{
      if (event.state && event.state.id) {{
        showPage(event.state.id, event.state.anchor, true);
      }}
    }};
    function buildBreadcrumb(id, title) {{
      const parts = id.split('/');
      let html = '<div class="breadcrumb"><a href="#" onclick="showPage(PAGE_ORDER[0]); return false;">Documentation</a>';
      if (parts.length > 1) {{
        html += '<span class="sep">&#8250;</span><span>' + parts[0].charAt(0).toUpperCase() + parts[0].slice(1) + '</span>';
      }}
      html += '<span class="sep">&#8250;</span><span>' + title + '</span></div>';
      return html;
    }}
    function buildPageNav(id) {{
      const idx = PAGE_ORDER.indexOf(id);
      if (idx === -1) return '';
      let html = '<div class="page-nav">';
      if (idx > 0) {{
        const prev = PAGE_ORDER[idx - 1];
        html += '<a href="#" onclick="showPage(\'' + prev + '\'); return false;">' +
          '<span class="nav-label">&larr; Pr&eacute;c&eacute;dent</span>' +
          '<span class="nav-title">' + (PAGES[prev] ? PAGES[prev].title : prev) + '</span></a>';
      }}
      if (idx < PAGE_ORDER.length - 1) {{
        const next = PAGE_ORDER[idx + 1];
        html += '<a class="nav-next" href="#" onclick="showPage(\'' + next + '\'); return false;">' +
          '<span class="nav-label">Suivant &rarr;</span>' +
          '<span class="nav-title">' + (PAGES[next] ? PAGES[next].title : next) + '</span></a>';
      }}
      html += '</div>';
      return html;
    }}
    function buildToc() {{
      const headings = document.querySelectorAll('.main h2, .main h3');
      const tocDiv = document.getElementById('toc-links');
      tocDiv.innerHTML = '';
      headings.forEach((h, i) => {{
        h.id = 'heading-' + i;
        const a = document.createElement('a');
        a.textContent = h.textContent;
        a.href = '#heading-' + i;
        a.className = h.tagName === 'H3' ? 'depth-3' : '';
        a.setAttribute('data-target', 'heading-' + i);
        a.onclick = (e) => {{ e.preventDefault(); h.scrollIntoView({{behavior:'smooth'}}); }};
        tocDiv.appendChild(a);
      }});
    }}
    function initScrollSpy() {{
      const tocLinks = document.querySelectorAll('.toc a[data-target]');
      if (!tocLinks.length) return;
      const observer = new IntersectionObserver(entries => {{
        entries.forEach(e => {{
          const link = document.querySelector('.toc a[data-target="' + e.target.id + '"]');
          if (link) link.classList.toggle('toc-active', e.isIntersecting);
        }});
      }}, {{ threshold: 0.3, rootMargin: '-80px 0px -60% 0px' }});
      document.querySelectorAll('h2[id], h3[id]').forEach(h => observer.observe(h));
    }}
    function addCopyButtons() {{
      document.querySelectorAll('pre').forEach(pre => {{
        if (pre.parentElement.classList.contains('code-wrapper')) return;
        const wrapper = document.createElement('div');
        wrapper.className = 'code-wrapper';
        pre.parentNode.insertBefore(wrapper, pre);
        wrapper.appendChild(pre);
        const btn = document.createElement('button');
        btn.className = 'copy-btn';
        btn.textContent = 'Copier';
        btn.onclick = () => {{
          navigator.clipboard.writeText(pre.textContent).then(() => {{
            btn.textContent = '\u2713 Copi\u00e9';
            btn.classList.add('copied');
            setTimeout(() => {{ btn.textContent = 'Copier'; btn.classList.remove('copied'); }}, 1500);
          }});
        }};
        wrapper.appendChild(btn);
      }});
    }}
    function renderMermaid() {{
      const nodes = document.querySelectorAll('pre code.language-mermaid');
      if (nodes.length > 0) {{
        nodes.forEach(block => {{
          const div = document.createElement('div');
          div.className = 'mermaid';
          div.textContent = block.textContent;
          block.parentElement.replaceWith(div);
        }});
      }}
      if (typeof mermaid !== 'undefined') {{
        try {{
          mermaid.run();
          setTimeout(setupMermaidZoom, 200);
        }} catch(e) {{ console.warn('Mermaid render error:', e); }}
      }} else {{
        setTimeout(renderMermaid, 500);
      }}
    }}

    function setupMermaidZoom() {{
      document.querySelectorAll('.mermaid').forEach(div => {{
        div.onclick = (e) => {{
          const svg = div.querySelector('svg');
          if (svg) openMermaidModal(svg.cloneNode(true));
        }};
      }});
    }}

    function openMermaidModal(svg) {{
      const modal = document.getElementById('mermaid-modal');
      modal.innerHTML = '';
      modal.appendChild(svg);
      modal.classList.add('open');
      document.body.style.overflow = 'hidden';
      
      let scale = 1;
      svg.style.transform = `scale(${{scale}})`;
      modal.onwheel = (e) => {{
        e.preventDefault();
        scale += e.deltaY * -0.001;
        scale = Math.min(Math.max(.125, scale), 4);
        svg.style.transform = `scale(${{scale}})`;
      }};
    }}

    function closeMermaidModal() {{
      const modal = document.getElementById('mermaid-modal');
      modal.classList.remove('open');
      document.body.style.overflow = '';
    }}

    /* Chat Logic */
    function toggleChat() {{
      const panel = document.getElementById('chat-panel');
      panel.classList.toggle('open');
      if (panel.classList.contains('open')) {{
        document.getElementById('chat-input').focus();
        document.getElementById('chat-badge').style.display = 'none';
      }}
    }}

    function handleChatKey(e) {{
      if (e.key === 'Enter' && !e.shiftKey) {{
        e.preventDefault();
        sendChatMessage();
      }}
    }}

    async function sendChatMessage() {{
      const input = document.getElementById('chat-input');
      const text = input.value.trim();
      if (!text) return;

      addMessage(text, 'user');
      input.value = '';
      input.style.height = 'auto';

      const sendBtn = document.getElementById('chat-send');
      sendBtn.disabled = true;

      const loadingMsg = addMessage('...', 'assistant');
      loadingMsg.classList.add('pulse-subtle');

      try {{
        // We assume the server is running on the same host if served via 'gitnexus serve'
        const repoName = document.querySelector('header h1').textContent;
        const response = await fetch('/api/chat', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{
            question: text,
            repo: repoName,
            history: getChatHistory()
          }})
        }});

        if (!response.ok) throw new Error('Erreur serveur');

        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        loadingMsg.textContent = '';
        loadingMsg.classList.remove('pulse-subtle');

        while (true) {{
          const {{ done, value }} = await reader.read();
          if (done) break;
          const chunk = decoder.decode(value, {{ stream: true }});
          // Simple parsing of SSE or raw stream
          loadingMsg.textContent += chunk;
          const container = document.getElementById('chat-messages');
          container.scrollTop = container.scrollHeight;
        }}
      }} catch (err) {{
        loadingMsg.textContent = "Désolé, je ne peux pas répondre pour le moment. Assurez-vous que 'gitnexus serve' est en cours d'exécution.";
        loadingMsg.classList.add('error');
      }} finally {{
        sendBtn.disabled = false;
      }}
    }}

    function addMessage(text, role) {{
      const container = document.getElementById('chat-messages');
      const div = document.createElement('div');
      div.className = `message ${{role}}`;
      div.textContent = text;
      container.appendChild(div);
      container.scrollTop = container.scrollHeight;
      return div;
    }}

    function getChatHistory() {{
      const messages = [];
      document.querySelectorAll('.message.user, .message.assistant').forEach(msg => {{
        messages.push({{
          role: msg.classList.contains('user') ? 'user' : 'assistant',
          content: msg.textContent
        }});
      }});
      return messages.slice(-10);
    }}
    function initSearch() {{
      const searchInput = document.getElementById('search-input');
      const searchResults = document.getElementById('search-results');
      const searchOverlay = document.getElementById('search-overlay');
      document.addEventListener('keydown', e => {{
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {{
          e.preventDefault();
          searchOverlay.classList.toggle('hidden');
          if (!searchOverlay.classList.contains('hidden')) searchInput.focus();
        }}
        if (e.key === 'Escape') searchOverlay.classList.add('hidden');
      }});
      searchInput.addEventListener('input', () => {{
        const q = searchInput.value.toLowerCase().trim();
        if (q.length < 2) {{ searchResults.innerHTML = ''; return; }}
        const results = SEARCH_INDEX
          .filter(p => p.title.toLowerCase().includes(q) || p.text.toLowerCase().includes(q))
          .slice(0, 10);
        searchResults.innerHTML = results.map(r => {{
          const idx = r.text.toLowerCase().indexOf(q);
          const start = Math.max(0, idx - 40);
          const end = Math.min(r.text.length, idx + q.length + 40);
          const snippet = (start > 0 ? '...' : '') +
            r.text.slice(start, idx) +
            '<mark>' + r.text.slice(idx, idx + q.length) + '</mark>' +
            r.text.slice(idx + q.length, end) +
            (end < r.text.length ? '...' : '');
          return '<a class="search-result" href="#" onclick="showPage(\'' + r.id + '\'); document.getElementById(\'search-overlay\').classList.add(\'hidden\'); return false;">' +
            '<div class="search-result-title">' + r.title + '</div>' +
            '<div class="search-result-snippet">' + (idx >= 0 ? snippet : '') + '</div>' +
            '</a>';
        }}).join('');
        if (results.length === 0) {{
          searchResults.innerHTML = '<div class="search-empty">Aucun r&eacute;sultat pour "' + q + '"</div>';
        }}
      }});
    }}
    function filterPages(query) {{
      const q = query.toLowerCase();
      document.querySelectorAll('.sidebar a[data-page]').forEach(a => {{
        a.style.display = a.textContent.toLowerCase().includes(q) ? '' : 'none';
      }});
      document.querySelectorAll('.sidebar .section-title').forEach(title => {{
        let next = title.nextElementSibling;
        let hasVisible = false;
        while (next && !next.classList.contains('section-title')) {{
          if (next.style.display !== 'none') hasVisible = true;
          next = next.nextElementSibling;
        }}
        title.style.display = hasVisible || !q ? '' : 'none';
      }});
    }}
    function toggleTheme() {{
      const html = document.documentElement;
      const next = html.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
      html.setAttribute('data-theme', next);
      localStorage.setItem('theme', next);
      if (typeof mermaid !== 'undefined') {{
        mermaid.initialize({{ theme: next === 'dark' ? 'dark' : 'default', startOnLoad: false, securityLevel: 'loose' }});
        renderMermaid();
      }}
    }}
    function toggleSidebar() {{
      document.querySelector('.sidebar').classList.toggle('open');
    }}
    document.addEventListener('DOMContentLoaded', () => {{
      const saved = localStorage.getItem('theme');
      if (saved) document.documentElement.setAttribute('data-theme', saved);
      if (typeof mermaid !== 'undefined') {{
        const theme = document.documentElement.getAttribute('data-theme') === 'light' ? 'default' : 'dark';
        mermaid.initialize({{ theme, startOnLoad: false, securityLevel: 'loose' }});
      }}
      buildDynamicSidebar();
      buildToc();
      renderMermaid();
      addCopyButtons();
      initSearch();
      initScrollSpy();
      if (typeof hljs !== 'undefined') {{
        document.querySelectorAll('pre code').forEach(block => {{
          if (!block.classList.contains('language-mermaid')) {{
            hljs.highlightElement(block);
          }}
        }});
      }}
      if (typeof lucide !== 'undefined') {{
        lucide.createIcons({{
          attrs: {{
            class: ["lucide-icon"]
          }}
        }});
      }}
    }});
  </script>
</body>
</html>"##
    )
}

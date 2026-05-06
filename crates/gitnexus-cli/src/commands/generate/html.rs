//! HTML site generator.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use serde_json::{json, Value};
use tracing::{info, warn};

use gitnexus_core::graph::KnowledgeGraph;

use super::markdown::{extract_title_from_md, html_escape, markdown_to_html};

pub(super) fn generate_html_site(
    graph: &KnowledgeGraph,
    repo_path: &Path,
    docs_dir: &Path,
) -> Result<()> {
    if !docs_dir.exists() {
        return Err(anyhow::anyhow!("No docs found. Run 'generate docs' first."));
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
        "overview",
        "functional-guide",
        "project-health",
        "architecture",
        "getting-started",
        "deployment",
        "hotspots",
        "coupling",
        "ownership",
        "aspnet-controllers",
        "aspnet-routes",
        "aspnet-entities",
        "aspnet-data-model",
        "aspnet-views",
        "aspnet-services",
        "aspnet-external",
        "aspnet-entities-detail",
        "aspnet-seq-http",
        "aspnet-seq-data",
    ];

    let mut overview_pages: Vec<_> = pages
        .iter()
        .filter(|(k, _)| !k.starts_with("modules/") && !k.starts_with("processes/"))
        .collect();
    // Sort by preferred order, then alphabetically for unlisted
    overview_pages.sort_by_key(|(k, _)| {
        preferred_order
            .iter()
            .position(|&p| k.as_str() == p)
            .unwrap_or(999)
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

    sidebar_html.push_str(&format!(
        "<div class=\"section-title\">{}. OVERVIEW</div>\n",
        section_num
    ));
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
        sidebar_html.push_str(&format!(
            "<div class=\"section-title\">{}. CONTROLLERS</div>\n",
            section_num
        ));
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
        sidebar_html.push_str(&format!(
            "<div class=\"section-title\">{}. DATA MODEL</div>\n",
            section_num
        ));
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
        sidebar_html.push_str(&format!(
            "<div class=\"section-title\">{}. MODULES</div>\n",
            section_num
        ));
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
        sidebar_html.push_str(&format!(
            "<div class=\"section-title\">{}. BUSINESS PROCESSES</div>\n",
            section_num
        ));
        for (sub_idx, (id, (title, _))) in process_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    let index_json_path = docs_dir.join("_index.json");
    let index_value = load_json_file(&index_json_path, json!({ "pages": [] }))?;

    let provenance_path = docs_dir.join("_meta").join("provenance.json");
    let provenance_value = load_json_file(&provenance_path, json!([]))?;
    let provenance_ids = provenance_ids_from_value(&provenance_value);

    let backlinks_path = docs_dir.join("_meta").join("backlinks.json");
    let backlinks_value = load_json_file(&backlinks_path, json!({}))?;

    // 3. Build pages JSON
    let pages_json: BTreeMap<String, serde_json::Value> = pages
        .iter()
        .map(|(id, (title, html))| {
            let page_type = classify_page_from_id(id);
            let stem = id.split('/').next_back().unwrap_or(id);
            let enriched = provenance_ids.contains(id) || provenance_ids.contains(stem);
            (
                id.clone(),
                serde_json::json!({
                    "title": title,
                    "html": html,
                    "page_type": page_type,
                    "enriched": enriched
                }),
            )
        })
        .collect();

    // 3b. Build PAGE_ORDER from the docs index so previous/next follows the
    // generated wiki navigation instead of BTreeMap alphabetical order.
    let page_order = page_order_from_index(&index_value, &pages);
    let page_order_json = script_safe_json(&serde_json::to_string(&page_order)?);

    // 3c. Build SEARCH_INDEX (stripped text for full-text search)
    let search_index: Vec<serde_json::Value> = pages
        .iter()
        .map(|(id, (title, html))| {
            let page_type = classify_page_from_id(id);
            let stem = id.split('/').next_back().unwrap_or(id);
            let enriched = provenance_ids.contains(id) || provenance_ids.contains(stem);
            json!({
                "id": id,
                "title": title,
                "text": strip_html_tags(html),
                "page_type": page_type,
                "enriched": enriched
            })
        })
        .collect();
    let search_index_json = script_safe_json(&serde_json::to_string(&search_index)?);

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
    let index_json = script_safe_json(&serde_json::to_string(&index_value)?);
    let provenance_json = script_safe_json(&serde_json::to_string(&provenance_value)?);
    let backlinks_json = script_safe_json(&serde_json::to_string(&backlinks_value)?);
    let pages_json_str = script_safe_json(&serde_json::to_string(&pages_json)?);
    let final_html = build_html_template(
        &project_name,
        &stats_str,
        &sidebar_html,
        first_page_html,
        &pages_json_str,
        &page_order_json,
        &search_index_json,
        &index_json,
        &provenance_json,
        &backlinks_json,
    );

    // 7. Check for local mermaid.min.js (offline support)
    let mermaid_path = docs_dir.join("mermaid.min.js");
    if !mermaid_path.exists() {
        println!(
            "  {} For offline diagrams, download mermaid.min.js to {}",
            "TIP".cyan(),
            docs_dir.display()
        );
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
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' {
            in_tag = false;
            result.push(' ');
            continue;
        }
        if !in_tag {
            result.push(c);
        }
    }
    // Collapse whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn load_json_file(path: &Path, fallback: Value) -> Result<Value> {
    if !path.exists() {
        return Ok(fallback);
    }

    let raw = std::fs::read_to_string(path)?;
    match serde_json::from_str::<Value>(&raw) {
        Ok(value) => Ok(value),
        Err(err) => {
            warn!(
                "Could not parse generated docs JSON {}: {}. Falling back to an empty value.",
                path.display(),
                err
            );
            Ok(fallback)
        }
    }
}

fn script_safe_json(json: &str) -> String {
    json.replace("</", "<\\/")
}

fn provenance_ids_from_value(value: &Value) -> HashSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            entry
                .get("page_id")
                .and_then(|id| id.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn page_order_from_index(index: &Value, pages: &BTreeMap<String, (String, String)>) -> Vec<String> {
    fn page_id_from_nav_item(item: &Value) -> Option<String> {
        let raw = item
            .get("path")
            .or_else(|| item.get("id"))
            .and_then(|v| v.as_str())?;
        let page_id = raw
            .trim_start_matches("./")
            .strip_suffix(".md")
            .unwrap_or(raw)
            .to_string();
        if page_id.is_empty() {
            None
        } else {
            Some(page_id)
        }
    }

    fn push_nav_item(
        item: &Value,
        pages: &BTreeMap<String, (String, String)>,
        seen: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) {
        if let Some(children) = item.get("children").and_then(|v| v.as_array()) {
            for child in children {
                push_nav_item(child, pages, seen, out);
            }
            return;
        }

        if let Some(page_id) = page_id_from_nav_item(item) {
            if pages.contains_key(&page_id) && seen.insert(page_id.clone()) {
                out.push(page_id);
            }
        }
    }

    let nav_pages = index
        .get("pages")
        .and_then(|v| v.as_array())
        .or_else(|| index.as_array());
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    if let Some(nav_pages) = nav_pages {
        for item in nav_pages {
            push_nav_item(item, pages, &mut seen, &mut out);
        }
    }

    for id in pages.keys() {
        if seen.insert(id.clone()) {
            out.push(id.clone());
        }
    }

    out
}

/// Classify a page ID into a display type for search filtering.
fn classify_page_from_id(id: &str) -> &'static str {
    let name = id.split('/').next_back().unwrap_or(id);
    if name.starts_with("ctrl-") {
        "Controller"
    } else if name.starts_with("data-") || name.contains("model") {
        "DataModel"
    } else if name.contains("service") || name.contains("repository") {
        "Service"
    } else if name.contains("external") {
        "ExternalService"
    } else if name == "overview" {
        "Overview"
    } else if name == "architecture" {
        "Architecture"
    } else if name.contains("view") || name.contains("ui") {
        "UI"
    } else {
        "Misc"
    }
}

/// Build the complete self-contained HTML template.
#[allow(clippy::too_many_arguments)]
fn build_html_template(
    project_name: &str,
    stats: &str,
    _sidebar_nav: &str,
    first_page_content: &str,
    pages_json: &str,
    page_order_json: &str,
    search_index_json: &str,
    index_json: &str,
    provenance_json: &str,
    backlinks_json: &str,
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
    .sidebar a:focus-visible,
    .top-bar button:focus-visible,
    .theme-toggle:focus-visible,
    .hamburger:focus-visible,
    .chat-toggle:focus-visible,
    .search input:focus-visible,
    button:focus-visible {{ outline:2px solid var(--accent); outline-offset:2px; border-radius:3px; }}
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
    .search-result-snippet mark {{ background: var(--accent); color: #fff; border-radius: 2px; padding: 0 2px; font-weight: 600; }}
    .search-type-badge {{
      font-size: 10px; padding: 1px 5px; border-radius: 3px;
      background: var(--bg-surface); border: 1px solid var(--border);
      vertical-align: middle; margin-left: 6px; color: var(--text-muted);
    }}
    .page-type-badge {{ font-size: 11px; vertical-align: middle; margin-left: 8px; }}
    .search-filter {{
      font-size: 11px; padding: 3px 8px; border: 1px solid var(--border);
      border-radius: 12px; background: var(--bg-surface); cursor: pointer; color: var(--text);
    }}
    .search-filter.active {{ background: var(--accent); color: #fff; border-color: var(--accent); }}
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
      .main table {{ overflow-x: auto; display: block; }}
      .main th, .main td {{ font-size: 11px; padding: 4px 6px; }}
    }}
    #back-to-top {{
      position:fixed; bottom:24px; right:24px; display:none;
      padding:8px 14px; background:var(--accent); color:#fff;
      border:none; border-radius:6px; cursor:pointer;
      font-size:13px; z-index:100;
      box-shadow:0 2px 8px rgba(0,0,0,.3); transition:opacity .2s;
    }}
    @media print {{
      .sidebar, .toc, .header, .theme-toggle, .copy-btn, .hamburger, .page-nav, .search, #back-to-top {{ display: none !important; }}
      .main {{ margin: 0; padding: 20px; max-width: 100%; }}
      body {{ font-family: Georgia, serif; font-size: 11pt; color: #000; background: #fff; }}
      pre {{ border: 1px solid #ccc; page-break-inside: avoid; font-size: 9pt; }}
      h1, h2, h3 {{ page-break-after: avoid; color: #000; }}
      a {{ color: #000; text-decoration: underline; }}
      .callout {{ border: 1px solid #ccc; break-inside: avoid; }}
      .mermaid, .mermaid svg {{ page-break-inside: avoid; max-width: 100% !important; }}
      .code-wrapper {{ page-break-inside: avoid; }}
    }}
    /* Provenance badge */
    .provenance-badge {{
      display: inline-flex; align-items: center; gap: 6px;
      font-size: 11px; color: var(--text-muted);
      background: var(--bg-surface); border: 1px solid var(--border);
      border-radius: 4px; padding: 3px 8px; margin-bottom: 10px;
      cursor: pointer; user-select: none;
    }}
    .provenance-badge:hover {{ border-color: var(--accent); color: var(--accent); }}
    .provenance-detail {{
      border-left: 1px solid var(--border); padding-left: 6px; margin-left: 2px;
    }}
    /* Evidence sources panel */
    .ev-panel {{
      background: var(--bg-surface); border: 1px solid var(--border);
      border-radius: 6px; padding: 12px; margin-bottom: 12px;
      max-height: 280px; overflow-y: auto;
    }}
    .ev-panel-title {{
      font-size: 11px; font-weight: 600; color: var(--text-muted);
      text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;
    }}
    .ev-source-item {{
      display: flex; align-items: center; gap: 8px;
      padding: 4px 0; border-bottom: 1px solid rgba(255,255,255,0.05);
    }}
    .ev-source-path {{
      font-family: monospace; font-size: 11px; cursor: pointer;
      color: var(--text-muted);
    }}
    .ev-source-path:hover {{ color: var(--accent); }}
    .ev-source-path.copied {{ color: #4ade80; }}
    /* Staleness warning */
    .stale-warning {{
      display: flex; align-items: center; gap: 8px;
      background: rgba(234,179,8,0.1); border: 1px solid rgba(234,179,8,0.4);
      border-radius: 6px; padding: 8px 12px; margin-bottom: 12px;
      font-size: 12px; color: #ca8a04;
    }}
    /* Auto-collapse long code blocks */
    .code-collapse {{ margin: 12px 0; }}
    .code-collapse > summary {{
      cursor: pointer; font-size: 12px; color: var(--text-muted);
      padding: 4px 8px; background: var(--bg-surface);
      border: 1px solid var(--border); border-radius: 4px;
      list-style: none; user-select: none;
    }}
    .code-collapse > summary::-webkit-details-marker {{ display: none; }}
    .code-collapse[open] > summary {{
      border-bottom-left-radius: 0; border-bottom-right-radius: 0; border-bottom: none;
    }}
    .code-collapse[open] > pre {{
      margin-top: 0; border-top-left-radius: 0; border-top-right-radius: 0;
    }}
    /* Related pages cards */
    .related-pages {{
      margin: 24px 0; padding: 16px;
      background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px;
    }}
    .related-pages-title {{
      font-size: 11px; font-weight: 600; color: var(--text-muted);
      text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;
    }}
    .related-page-card {{
      display: inline-block;
      background: var(--bg); border: 1px solid var(--border);
      border-radius: 6px; padding: 5px 12px; margin: 4px;
      font-size: 13px; text-decoration: none; color: var(--accent);
      transition: background 0.15s;
    }}
    .related-page-card:hover {{ background: rgba(106,161,248,0.08); }}
    /* Backlinks */
    .backlinks-section {{ margin: 24px 0; padding: 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; }}
    .backlinks-title {{ font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px; }}
    /* Mobile TOC */
    .toc-mobile-details {{ margin-bottom: 1rem; background: var(--bg-secondary, var(--bg-surface)); border-radius: 6px; padding: 0.5rem 1rem; border: 1px solid var(--border); }}
    .toc-mobile-details summary {{ cursor: pointer; font-size: 13px; color: var(--text-muted); }}
  </style>
</head>
<body>
  <a href="#content" id="skip-nav" style="position:absolute;top:-44px;left:6px;padding:8px 16px;background:var(--accent);color:#fff;border-radius:0 0 6px 6px;font-size:13px;font-weight:600;text-decoration:none;z-index:9999;transition:top .15s;" onfocus="this.style.top='0'" onblur="this.style.top='-44px'">Passer au contenu</a>
  <button class="hamburger" onclick="toggleSidebar()" aria-label="Ouvrir la navigation">&#9776;</button>
  <div id="search-overlay" class="hidden"
    style="position:fixed;inset:0;z-index:100;background:rgba(0,0,0,0.6);display:flex;align-items:flex-start;justify-content:center;padding-top:15vh;">
    <div style="width:560px;max-width:90vw;background:var(--bg-surface);border:1px solid var(--border);border-radius:12px;overflow:hidden;box-shadow:0 8px 32px rgba(0,0,0,0.3);">
      <div style="padding:12px 16px;border-bottom:1px solid var(--border);">
        <input id="search-input" type="text" aria-label="Rechercher dans la documentation" placeholder="Rechercher dans la documentation... (Ctrl+K)"
          style="width:100%;padding:8px 12px;background:var(--bg);border:1px solid var(--border);border-radius:8px;color:var(--text);font-size:14px;outline:none;">
        <div id="search-filters" style="display:flex;gap:6px;margin-top:8px;flex-wrap:wrap;">
          <button class="search-filter active" data-filter="all" onclick="setSearchFilter('all',this)">Tout</button>
          <button class="search-filter" data-filter="Controller" onclick="setSearchFilter('Controller',this)">Controllers</button>
          <button class="search-filter" data-filter="Service" onclick="setSearchFilter('Service',this)">Services</button>
          <button class="search-filter" data-filter="DataModel" onclick="setSearchFilter('DataModel',this)">Data</button>
          <button class="search-filter" data-filter="enriched" onclick="setSearchFilter('enriched',this)">Enrichi ✓</button>
        </div>
      </div>
      <div id="search-results" style="max-height:380px;overflow-y:auto;padding:8px;"></div>
    </div>
  </div>
  <div id="mermaid-modal" class="mermaid-modal" onclick="closeMermaidModal()"></div>
  <header class="header">
    <h1>{project_name}</h1>
    <span class="stats">{stats}</span>
    <button class="theme-toggle" onclick="toggleTheme()" aria-label="Basculer le thème">Theme</button>
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
    <button id="chat-toggle" class="chat-toggle" onclick="toggleChat()" aria-label="Ouvrir le chat">
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
    const PROVENANCE = {provenance_json};
    const BACKLINKS = {backlinks_json};
    let currentPage = null;
    let mermaidRetryCount = 0;

    const ICON_ALLOWLIST = new Set([
      'activity','arrow-right-left','book-open','cloud','cog','component','database',
      'file-text','flame','folder','git-branch','git-commit','globe','hard-drive',
      'home','layers','layout','link','route','server','table-2','users','workflow'
    ]);

    function navPages() {{
      if (INDEX_JSON && Array.isArray(INDEX_JSON.pages)) return INDEX_JSON.pages;
      if (Array.isArray(INDEX_JSON)) return INDEX_JSON;
      return PAGE_ORDER.map(id => ({{
        id,
        path: id + '.md',
        title: PAGES[id] ? PAGES[id].title : id,
        icon: 'file-text'
      }}));
    }}

    function decodeTitle(value) {{
      const textarea = document.createElement('textarea');
      textarea.innerHTML = String(value || '');
      return textarea.value;
    }}

    function safeIcon(name, fallback) {{
      const candidate = String(name || fallback || 'file-text');
      return ICON_ALLOWLIST.has(candidate) ? candidate : (fallback || 'file-text');
    }}

    function appendIcon(parent, name, fallback) {{
      const icon = document.createElement('i');
      icon.setAttribute('data-lucide', safeIcon(name, fallback));
      icon.style.cssText = 'width:14px;height:14px;vertical-align:middle;margin-right:6px;margin-top:-2px;';
      parent.appendChild(icon);
    }}

    function navPageId(item) {{
      const raw = item && (item.path || item.id) ? String(item.path || item.id) : '';
      return raw.replace(/\.md$/, '').replace(/^\.\//, '');
    }}

    function appendSidebarLink(parent, item) {{
      const pageId = navPageId(item);
      if (!pageId || !PAGES[pageId]) return;
      const link = document.createElement('a');
      link.href = '#';
      link.dataset.page = pageId;
      link.onclick = function(e) {{
        e.preventDefault();
        showPage(pageId);
        return false;
      }};
      appendIcon(link, item.icon, 'file-text');
      link.appendChild(document.createTextNode(
        decodeTitle(item.title || (PAGES[pageId] && PAGES[pageId].title) || pageId)
      ));
      parent.appendChild(link);
    }}

    function buildDynamicSidebar() {{
      const container = document.getElementById('dynamic-sidebar');
      container.replaceChildren();
      const sections = navPages();
      if (!sections.length) return;

      sections.forEach((section, i) => {{
        if (section.children && section.children.length > 0) {{
          var secCollapsed = sessionStorage.getItem('gnx_sec_' + i) === '1';
          const title = document.createElement('div');
          title.className = 'section-title';
          title.style.cursor = 'pointer';
          title.style.userSelect = 'none';
          title.onclick = function() {{ toggleSection(title, i); }};
          appendIcon(title, section.icon, 'folder');
          title.appendChild(document.createTextNode(decodeTitle(section.title || 'Section').toUpperCase()));
          const arrow = document.createElement('span');
          arrow.style.cssText = 'float:right;font-size:10px;margin-right:4px;';
          arrow.textContent = secCollapsed ? '\u25b8' : '\u25be';
          title.appendChild(arrow);
          container.appendChild(title);

          const childContainer = document.createElement('div');
          childContainer.id = 'gnx-sec-' + i;
          if (secCollapsed) childContainer.style.display = 'none';
          section.children.forEach(child => appendSidebarLink(childContainer, child));
          container.appendChild(childContainer);
        }} else {{
          appendSidebarLink(container, section);
        }}
      }});
    }}
    function toggleSection(el, i) {{
      var ch = document.getElementById('gnx-sec-' + i);
      if (!ch) return;
      var collapsed = ch.style.display === 'none';
      ch.style.display = collapsed ? '' : 'none';
      var arrow = el.querySelector('span');
      if (arrow) arrow.textContent = collapsed ? '\u25be' : '\u25b8';
      sessionStorage.setItem('gnx_sec_' + i, collapsed ? '0' : '1');
    }}

    function showToast(msg, ms) {{
      var t = document.createElement('div');
      t.style.cssText = 'position:fixed;bottom:64px;left:50%;transform:translateX(-50%);background:#333;color:#fff;padding:8px 16px;border-radius:6px;font-size:13px;z-index:999;transition:opacity .4s;pointer-events:none;';
      t.textContent = msg;
      document.body.appendChild(t);
      setTimeout(function() {{ t.style.opacity = '0'; setTimeout(function() {{ t.remove(); }}, 400); }}, ms || 2500);
    }}
    function showPage(id, anchor, skipHistory = false) {{
      if (window.innerWidth < 900) {{
        var sb = document.querySelector('.sidebar');
        if (sb) sb.classList.remove('open');
      }}
      const page = PAGES[id];
      if (!page) {{ showToast('Page introuvable\u00a0: ' + id); return; }}
      if (id === currentPage) {{
        if (anchor) {{ var aEl = document.getElementById(anchor); if (aEl) aEl.scrollIntoView({{behavior:'smooth'}}); }}
        return;
      }}
      const prevPage = currentPage;
      currentPage = id;

      if (!skipHistory) {{
        const url = "#" + id + (anchor ? "%23" + anchor : "");
        history.pushState({{id: id, anchor: anchor}}, "", url);
      }}

      const content = document.getElementById('content');
      if (prevPage && prevPage !== id) {{
        sessionStorage.setItem('scroll_' + prevPage, content.scrollTop.toString());
      }}
      content.style.opacity = '0';
      setTimeout(() => {{
        content.innerHTML = page.html;
        // Inject page-type badge after the first H1
        const ptype = page.page_type;
        if (ptype && ptype !== 'Misc') {{
          const h1 = content.querySelector('h1');
          if (h1) {{
            const typeBadge = document.createElement('span');
            typeBadge.className = 'search-type-badge page-type-badge';
            typeBadge.setAttribute('data-type', ptype);
            typeBadge.textContent = ptype;
            h1.appendChild(typeBadge);
          }}
        }}
        content.prepend(buildBreadcrumb(id, page.title));
        
        // Estimated reading time
        const wordCount = content.textContent.split(/\\s+/).length;
        const readTime = Math.max(1, Math.ceil(wordCount / 200));
        content.insertAdjacentHTML('afterbegin', `<div class="reading-time" style="font-size:12px; color:var(--text-muted); margin-bottom:16px;"><i data-lucide="clock" style="width:12px;height:12px;vertical-align:middle;margin-right:4px;"></i>~${{readTime}} min de lecture</div>`);

        // Provenance badge
        if (Array.isArray(PROVENANCE)) {{
          const provEntry = PROVENANCE.find(p =>
            p.page_id === id || p.page_id === id.split('/').pop()
          );
          if (provEntry) {{
            const model = (provEntry.model || '').split('/').pop();
            const enrichedAt = new Date(provEntry.enriched_at);
            const ageDays = (Date.now() - enrichedAt) / 86400000;
            const ago = timeSince(enrichedAt);
            const sourcesCount = (provEntry.evidence_refs || []).length;
            const panelId = 'ev-panel-' + id.replace(/\//g, '-');
            // Build badge
            const badge = document.createElement('div');
            badge.className = 'provenance-badge';
            badge.title = 'Cliquer pour voir les sources analysées';
            badge.onclick = function() {{ toggleEvPanel(panelId); }};
            const badgeIcon = document.createElement('i');
            badgeIcon.setAttribute('data-lucide', 'cpu');
            badgeIcon.style.cssText = 'width:12px;height:12px;';
            const modelCode = document.createElement('code');
            modelCode.textContent = model;
            const detail = document.createElement('span');
            detail.className = 'provenance-detail';
            detail.textContent = sourcesCount + ' source' + (sourcesCount !== 1 ? 's' : '');
            badge.appendChild(badgeIcon);
            badge.appendChild(document.createTextNode(' Enrichi par '));
            badge.appendChild(modelCode);
            badge.appendChild(document.createTextNode(' · ' + ago + ' '));
            badge.appendChild(detail);
            content.insertBefore(badge, content.firstChild);
            // Build evidence panel
            const panel = document.createElement('div');
            panel.className = 'ev-panel';
            panel.id = panelId;
            panel.style.display = 'none';
            const panelTitle = document.createElement('div');
            panelTitle.className = 'ev-panel-title';
            panelTitle.textContent = 'Sources analysées (' + sourcesCount + ')';
            panel.appendChild(panelTitle);
            (provEntry.evidence_refs || []).forEach(function(r) {{
              const item = document.createElement('div');
              item.className = 'ev-source-item';
              const loc = r.start_line ? ':L' + r.start_line : '';
              const span = document.createElement('span');
              span.className = 'ev-source-path';
              span.textContent = (r.file_path || '') + loc;
              span.title = 'Cliquer pour copier';
              span.dataset.path = (r.file_path || '') + loc;
              span.onclick = function() {{ copyPath(this.dataset.path); }};
              item.appendChild(span);
              panel.appendChild(item);
            }});
            content.insertBefore(panel, badge.nextSibling);
            if (ageDays > 30) {{
              content.insertAdjacentHTML('afterbegin', `<div class="stale-warning"><i data-lucide="alert-triangle" style="width:14px;height:14px;"></i> Documentation enrichie il y a ${{Math.floor(ageDays)}} jours. Relancer <code>gitnexus generate html --enrich</code> pour actualiser.</div>`);
            }}
          }}
        }}

        // Backlinks "Citée dans"
        const bl = BACKLINKS[id] || BACKLINKS[id.split('/').pop()] || [];
        if (bl.length > 0) {{
          const blDiv = document.createElement('div');
          blDiv.className = 'backlinks-section';
          const blTitle = document.createElement('div');
          blTitle.className = 'backlinks-title';
          blTitle.textContent = 'Citée dans';
          blDiv.appendChild(blTitle);
          bl.forEach(function(p) {{
            const pageId = String(p || '');
            if (!PAGES[pageId]) return;
            const a = document.createElement('a');
            a.className = 'related-page-card';
            a.href = '#';
            a.textContent = pageId;
            a.onclick = function(e) {{
              e.preventDefault();
              showPage(pageId);
              return false;
            }};
            blDiv.appendChild(a);
          }});
          content.appendChild(blDiv);
        }}

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
        
        content.appendChild(buildPageNav(id));
        document.querySelectorAll('.sidebar a[data-page]').forEach(a => a.classList.remove('active'));
        const link = Array.from(document.querySelectorAll('.sidebar a[data-page]')).find(a => a.dataset.page === id);
        if (link) {{ link.classList.add('active'); link.scrollIntoView({{block:'nearest'}}); }}
        
        // Make `<details>` list items clickable if they look like paths
        document.querySelectorAll('details li').forEach(li => {{
            const text = li.textContent.trim();
            if (text.includes('/') && text.includes('.')) {{
                li.replaceChildren();
                const icon = document.createElement('i');
                icon.setAttribute('data-lucide', 'file-code');
                icon.style.cssText = 'width:12px;height:12px;vertical-align:middle;margin-right:6px;opacity:0.7;';
                const span = document.createElement('span');
                span.style.cssText = 'font-family:monospace;font-size:12px;cursor:copy;';
                span.title = 'Click to copy path';
                span.textContent = text;
                span.onclick = function() {{
                  navigator.clipboard.writeText(text);
                  span.style.color = 'var(--accent)';
                  setTimeout(function() {{ span.style.color = ''; }}, 1000);
                }};
                li.appendChild(icon);
                li.appendChild(span);
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
          var savedScroll = sessionStorage.getItem('scroll_' + id);
          content.scrollTop = savedScroll ? parseInt(savedScroll, 10) : 0;
        }}
      }}, 100);
    }}

    function submitFeedback(id, isPositive) {{
        localStorage.setItem('feedback_' + id, isPositive ? 'yes' : 'no');
        document.getElementById('feedback-thanks').style.display = 'block';
        document.querySelectorAll('.feedback-btn').forEach(btn => btn.style.display = 'none');
    }}

    function timeSince(date) {{
        const secs = Math.floor((Date.now() - date) / 1000);
        if (secs < 3600) return Math.floor(secs / 60) + ' min';
        if (secs < 86400) return Math.floor(secs / 3600) + 'h';
        const d = Math.floor(secs / 86400);
        return d + ' jour' + (d > 1 ? 's' : '');
    }}

    function toggleEvPanel(panelId) {{
        const panel = document.getElementById(panelId);
        if (panel) panel.style.display = panel.style.display === 'none' ? 'block' : 'none';
    }}

    function copyPath(path) {{
        navigator.clipboard.writeText(path).catch(() => {{}});
        document.querySelectorAll('.ev-source-path').forEach(function(s) {{
            if (s.dataset.path === path) {{
                s.classList.add('copied');
                setTimeout(function() {{ s.classList.remove('copied'); }}, 1200);
            }}
        }});
    }}

    window.onpopstate = function(event) {{
      if (event.state && event.state.id) {{
        showPage(event.state.id, event.state.anchor, true);
      }}
    }};
    function buildBreadcrumb(id, title) {{
      const parts = id.split('/');
      const breadcrumb = document.createElement('div');
      breadcrumb.className = 'breadcrumb';
      const home = document.createElement('a');
      home.href = '#';
      home.textContent = 'Documentation';
      home.onclick = function(e) {{
        e.preventDefault();
        showPage(PAGE_ORDER[0]);
        return false;
      }};
      breadcrumb.appendChild(home);
      if (parts.length > 1) {{
        const sep = document.createElement('span');
        sep.className = 'sep';
        sep.textContent = '\u203a';
        const group = document.createElement('span');
        group.textContent = parts[0].charAt(0).toUpperCase() + parts[0].slice(1);
        breadcrumb.appendChild(sep);
        breadcrumb.appendChild(group);
      }}
      const sep = document.createElement('span');
      sep.className = 'sep';
      sep.textContent = '\u203a';
      const current = document.createElement('span');
      current.textContent = decodeTitle(title);
      breadcrumb.appendChild(sep);
      breadcrumb.appendChild(current);
      return breadcrumb;
    }}
    function buildPageNav(id) {{
      const idx = PAGE_ORDER.indexOf(id);
      const nav = document.createElement('div');
      nav.className = 'page-nav';
      if (idx === -1) return nav;
      const appendNavLink = function(pageId, label, isNext) {{
        const a = document.createElement('a');
        a.href = '#';
        if (isNext) a.className = 'nav-next';
        a.onclick = function(e) {{
          e.preventDefault();
          showPage(pageId);
          return false;
        }};
        const labelEl = document.createElement('span');
        labelEl.className = 'nav-label';
        labelEl.textContent = label;
        const titleEl = document.createElement('span');
        titleEl.className = 'nav-title';
        titleEl.textContent = decodeTitle(PAGES[pageId] ? PAGES[pageId].title : pageId);
        a.appendChild(labelEl);
        a.appendChild(titleEl);
        nav.appendChild(a);
      }};
      if (idx > 0) {{
        appendNavLink(PAGE_ORDER[idx - 1], '\u2190 Précédent', false);
      }}
      if (idx < PAGE_ORDER.length - 1) {{
        appendNavLink(PAGE_ORDER[idx + 1], 'Suivant \u2192', true);
      }}
      return nav;
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
        a.onclick = (e) => {{ e.preventDefault(); h.scrollIntoView({{behavior:'smooth'}}); var td = document.querySelector('.toc details'); if (td) td.open = false; }};
        tocDiv.appendChild(a);
      }});
      // Mobile: inject a collapsible <details> at the top of the content
      document.querySelector('.toc-mobile-details')?.remove();
      if (window.innerWidth < 900 && headings.length > 0) {{
        const content = document.getElementById('content');
        const details = document.createElement('details');
        details.className = 'toc-mobile-details';
        const summary = document.createElement('summary');
        summary.textContent = 'Dans cette page (' + headings.length + ' sections)';
        details.appendChild(summary);
        const cloned = tocDiv.cloneNode(true);
        details.appendChild(cloned);
        content.prepend(details);
      }}
    }}
    function initScrollSpy() {{
      const tocLinks = document.querySelectorAll('.toc a[data-target]');
      if (!tocLinks.length) return;
      const observer = new IntersectionObserver(entries => {{
        entries.forEach(e => {{
          const link = document.querySelector('.toc a[data-target="' + e.target.id + '"]');
          if (link) {{
            link.classList.toggle('toc-active', e.isIntersecting);
            if (e.isIntersecting) link.scrollIntoView({{block:'nearest'}});
          }}
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
        var codeEl = pre.querySelector('code');
        if (codeEl) {{
          var lm = codeEl.className.match(/language-(\w+)/);
          if (lm && lm[1] !== 'mermaid') {{
            var lbl = document.createElement('div');
            lbl.style.cssText = 'position:absolute;top:8px;right:80px;font-size:10px;color:var(--text-muted);opacity:0.6;font-family:monospace;pointer-events:none;';
            lbl.textContent = lm[1].toUpperCase();
            wrapper.appendChild(lbl);
          }}
        }}
      }});
    }}
    function renderMermaid() {{
      const nodes = document.querySelectorAll('pre code.language-mermaid');
      if (nodes.length > 0) {{
        nodes.forEach(block => {{
          const div = document.createElement('div');
          div.className = 'mermaid';
          div.textContent = block.textContent;
          div.setAttribute('data-source', block.textContent);
          block.parentElement.replaceWith(div);
        }});
      }}
      if (typeof mermaid !== 'undefined') {{
        try {{
          mermaidRetryCount = 0;
          mermaid.run();
          setTimeout(setupMermaidZoom, 200);
        }} catch(e) {{
          document.querySelectorAll('.mermaid').forEach(function(el) {{
            var errDiv = document.createElement('div');
            errDiv.style.cssText = 'padding:12px 16px;background:var(--bg-surface);border:1px dashed var(--border);border-radius:6px;color:var(--text-muted);font-size:12px;';
            errDiv.textContent = 'Diagram error: ' + e.message;
            el.replaceWith(errDiv);
          }});
        }}
      }} else {{
        mermaidRetryCount += 1;
        if (mermaidRetryCount < 10) {{
          setTimeout(renderMermaid, 500);
        }} else {{
          document.querySelectorAll('.mermaid').forEach(function(el) {{
            var errDiv = document.createElement('div');
            errDiv.style.cssText = 'padding:12px 16px;background:var(--bg-surface);border:1px dashed var(--border);border-radius:6px;color:var(--text-muted);font-size:12px;white-space:pre-wrap;';
            errDiv.textContent = 'Mermaid indisponible. Le diagramme reste visible en source :\n\n' + (el.getAttribute('data-source') || el.textContent || '');
            el.replaceWith(errDiv);
          }});
        }}
      }}
    }}

    function setupMermaidZoom() {{
      document.querySelectorAll('.mermaid').forEach(div => {{
        div.style.position = 'relative';
        div.onclick = (e) => {{
          if (e.target.tagName === 'BUTTON') return;
          const svg = div.querySelector('svg');
          if (svg) openMermaidModal(svg.cloneNode(true));
        }};
        var srcBtn = document.createElement('button');
        srcBtn.className = 'copy-btn';
        srcBtn.style.cssText = 'position:absolute;top:8px;right:8px;font-size:11px;z-index:2;';
        srcBtn.textContent = 'Copier source';
        srcBtn.onclick = function(e) {{
          e.stopPropagation();
          var src = div.getAttribute('data-source') || '';
          navigator.clipboard.writeText(src).then(function() {{
            srcBtn.textContent = '\u2713 Copi\u00e9';
            setTimeout(function() {{ srcBtn.textContent = 'Copier source'; }}, 1500);
          }});
        }};
        div.appendChild(srcBtn);
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
        const headers = {{ 'Content-Type': 'application/json' }};
        const token = window.localStorage.getItem('GITNEXUS_HTTP_TOKEN') || window.localStorage.getItem('gitnexus.httpToken');
        if (token) headers.Authorization = `Bearer ${{token}}`;
        const response = await fetch('/api/chat', {{
          method: 'POST',
          headers,
          body: JSON.stringify({{
            question: text,
            repo: repoName,
            history: getChatHistory()
          }})
        }});

        if (response.status === 401) throw new Error('Token HTTP GitNexus manquant ou invalide.');
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
        loadingMsg.textContent = err?.message || "Désolé, je ne peux pas répondre pour le moment. Assurez-vous que 'gitnexus serve' est en cours d'exécution.";
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
    let _searchActiveFilter = 'all';
    function setSearchFilter(filter, btn) {{
      _searchActiveFilter = filter;
      sessionStorage.setItem('gnx_search_filter', filter);
      document.querySelectorAll('.search-filter').forEach(b => b.classList.remove('active'));
      if (btn) btn.classList.add('active');
      document.getElementById('search-input').dispatchEvent(new Event('input'));
    }}
    function initSearch() {{
      const searchInput = document.getElementById('search-input');
      const searchResults = document.getElementById('search-results');
      const searchOverlay = document.getElementById('search-overlay');
      var savedFilter = sessionStorage.getItem('gnx_search_filter');
      if (savedFilter) {{
        _searchActiveFilter = savedFilter;
        document.querySelectorAll('.search-filter').forEach(function(b) {{
          b.classList.toggle('active', b.getAttribute('data-filter') === savedFilter);
        }});
      }}
      document.addEventListener('keydown', e => {{
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {{
          e.preventDefault();
          searchOverlay.classList.toggle('hidden');
          if (!searchOverlay.classList.contains('hidden')) searchInput.focus();
        }}
        if (e.key === 'Escape') searchOverlay.classList.add('hidden');
      }});
      function normSearch(s) {{ return s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase(); }}
      function searchScore(page, q) {{
        let s = 0;
        const t = normSearch(page.title), x = normSearch(page.text);
        if (q.indexOf(' ') !== -1) {{
          if (t.indexOf(q) !== -1) s += 80;
          if (x.indexOf(q) !== -1) s += 40;
        }}
        if (t.startsWith(q)) s += 20;
        else if (t.indexOf(q) !== -1) s += 10;
        if (x.indexOf(q) !== -1) s += 1 + Math.min(4, (x.split(q).length - 1));
        return s;
      }}
      var _searchTimer = null;
      searchInput.addEventListener('input', () => {{
        clearTimeout(_searchTimer);
        _searchTimer = setTimeout(runSearch, 250);
      }});
      function runSearch() {{
        const q = normSearch(searchInput.value.trim());
        if (q.length < 2) {{ searchResults.innerHTML = ''; return; }}
        const results = SEARCH_INDEX
          .filter(p => {{
            if (_searchActiveFilter === 'enriched') return p.enriched;
            if (_searchActiveFilter !== 'all') return p.page_type === _searchActiveFilter;
            return true;
          }})
          .filter(p => normSearch(p.title).includes(q) || normSearch(p.text).includes(q))
          .map(p => ({{ ...p, _score: searchScore(p, q) }}))
          .sort((a, b) => b._score - a._score)
          .slice(0, 15);
        if (results.length === 0) {{
          searchResults.innerHTML = '<div class="search-empty">Aucun r&eacute;sultat</div>';
          return;
        }}
        searchResults.innerHTML = '';
        results.forEach(r => {{
          const a = document.createElement('a');
          a.className = 'search-result';
          a.href = '#';
          a.onclick = function() {{
            showPage(r.id);
            document.getElementById('search-overlay').classList.add('hidden');
            return false;
          }};
          const titleDiv = document.createElement('div');
          titleDiv.className = 'search-result-title';
          titleDiv.textContent = r.title;
          if (r.page_type && r.page_type !== 'Misc') {{
            const badge = document.createElement('span');
            badge.className = 'search-type-badge';
            badge.textContent = r.page_type;
            titleDiv.appendChild(badge);
          }}
          if (r.enriched) {{
            const icon = document.createElement('i');
            icon.setAttribute('data-lucide', 'cpu');
            icon.style.cssText = 'width:10px;height:10px;color:var(--accent);vertical-align:middle;margin-left:4px;';
            titleDiv.appendChild(icon);
          }}
          a.appendChild(titleDiv);
          const idx = r.text.toLowerCase().indexOf(q);
          if (idx >= 0) {{
            const start = Math.max(0, idx - 40);
            const end = Math.min(r.text.length, idx + q.length + 160);
            const snippet = (start > 0 ? '\u2026' : '') + r.text.slice(start, end) + (end < r.text.length ? '\u2026' : '');
            const snippetDiv = document.createElement('div');
            snippetDiv.className = 'search-result-snippet';
            var lsnip = snippet.toLowerCase(), sp = 0, lp = 0;
            while ((sp = lsnip.indexOf(q, lp)) !== -1) {{
              if (sp > lp) snippetDiv.appendChild(document.createTextNode(snippet.slice(lp, sp)));
              var mk = document.createElement('mark');
              mk.textContent = snippet.slice(sp, sp + q.length);
              snippetDiv.appendChild(mk);
              lp = sp + q.length;
            }}
            if (lp < snippet.length) snippetDiv.appendChild(document.createTextNode(snippet.slice(lp)));
            a.appendChild(snippetDiv);
          }}
          searchResults.appendChild(a);
        }});
        if (typeof lucide !== 'undefined') lucide.createIcons();
      }}
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
      // Alt+← / Alt+→ : navigate between pages
      document.addEventListener('keydown', function(e) {{
        if (!e.altKey || e.ctrlKey || e.metaKey) return;
        var idx = PAGE_ORDER.indexOf(currentPage);
        if (e.key === 'ArrowLeft'  && idx > 0)                              {{ e.preventDefault(); showPage(PAGE_ORDER[idx - 1]); }}
        if (e.key === 'ArrowRight' && idx >= 0 && idx < PAGE_ORDER.length - 1) {{ e.preventDefault(); showPage(PAGE_ORDER[idx + 1]); }}
      }});
      // Back-to-top button visibility
      document.getElementById('content').addEventListener('scroll', function() {{
        var btn = document.getElementById('back-to-top');
        if (btn) btn.style.display = this.scrollTop > 300 ? 'block' : 'none';
      }});
    }});
  </script>
  <button id="back-to-top" onclick="document.getElementById('content').scrollTop=0"
          aria-label="Retour en haut de page">&#8593; Haut</button>
</body>
</html>"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_json_escapes_script_end_tags() {
        let json = r#"{"title":"</script><script>alert(1)</script>"}"#;
        let safe = script_safe_json(json);
        assert!(!safe.contains("</script>"));
        assert!(safe.contains("<\\/script>"));
    }

    #[test]
    fn page_order_follows_index_pages_and_appends_missing_pages() {
        let index = json!({
            "pages": [
                {"id": "overview", "path": "overview.md"},
                {"id": "modules", "children": [
                    {"id": "service", "path": "modules/service.md"}
                ]}
            ]
        });
        let mut pages = BTreeMap::new();
        pages.insert(
            "overview".to_string(),
            ("Overview".to_string(), String::new()),
        );
        pages.insert(
            "modules/service".to_string(),
            ("Service".to_string(), String::new()),
        );
        pages.insert(
            "architecture".to_string(),
            ("Architecture".to_string(), String::new()),
        );

        assert_eq!(
            page_order_from_index(&index, &pages),
            vec![
                "overview".to_string(),
                "modules/service".to_string(),
                "architecture".to_string()
            ]
        );
    }
}

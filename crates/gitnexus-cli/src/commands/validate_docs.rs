//! Pre-delivery linter for the generated documentation.
//!
//! Walks `.gitnexus/docs/**/*.md` and reports issues that would embarrass us
//! in front of a client. Three severity levels:
//!
//!   RED   — must fix before delivery (TODO/TBD residuals, broken markdown
//!           links, unfilled GNX:* anchors that leaked through enrichment).
//!   YELLOW— should fix if time allows (sections under 50 words, missing
//!           §4 Algorithmes on Service / Controller pages).
//!   GREEN — clean, no issues found.
//!
//! Reports both to the console (with colored summary) and to
//! `.gitnexus/docs/_meta/validation.json` for CI / scripted post-processing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Red,
    Yellow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub kind: String,
    pub line: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PageReport {
    pub path: String,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub repo: String,
    pub generated_at: String,
    pub total_pages: usize,
    pub pages_with_issues: usize,
    pub red_count: usize,
    pub yellow_count: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub pages: Vec<PageReport>,
}

pub fn run(repo_path: Option<&str>, docs_dir_override: Option<&str>, json: bool) -> Result<()> {
    let repo = match repo_path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("cwd")?,
    };
    // `--docs-dir` lets us validate the LIVRABLES output dir or any other
    // generated docs tree, not just the canonical `.gitnexus/docs/` location.
    // Useful right after `gitnexus generate docx --output-dir <livrables>` to
    // gate delivery on the actual artefact, not the source dir.
    let docs_dir = match docs_dir_override {
        Some(d) => PathBuf::from(d),
        None => repo.join(".gitnexus").join("docs"),
    };
    if !docs_dir.exists() {
        anyhow::bail!(
            "No documentation found at {}. Run `gitnexus generate docs` first \
             (or pass --docs-dir to point at an explicit location).",
            docs_dir.display()
        );
    }

    let md_files = collect_md_files(&docs_dir)?;
    let mut pages: Vec<PageReport> = Vec::new();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut red_count = 0usize;
    let mut yellow_count = 0usize;

    let known_files: Vec<PathBuf> = md_files.clone();

    for path in &md_files {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut report = PageReport {
            path: relativize(&docs_dir, path),
            issues: Vec::new(),
        };
        check_residual_todos(&content, &mut report.issues);
        check_unfilled_anchors(&content, &mut report.issues);
        check_short_sections(&content, &mut report.issues);
        check_broken_internal_links(path, &content, &known_files, &mut report.issues);
        check_methodo_alise_section_4(path, &content, &mut report.issues);

        for iss in &report.issues {
            *by_kind.entry(iss.kind.clone()).or_default() += 1;
            match iss.severity {
                Severity::Red => red_count += 1,
                Severity::Yellow => yellow_count += 1,
            }
        }
        if !report.issues.is_empty() {
            pages.push(report);
        }
    }

    let report = ValidationReport {
        repo: repo.display().to_string(),
        generated_at: chrono::Utc::now()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        total_pages: md_files.len(),
        pages_with_issues: pages.len(),
        red_count,
        yellow_count,
        by_kind,
        pages,
    };

    // Persist a JSON sidecar for CI / scripted consumption.
    let meta_dir = docs_dir.join("_meta");
    std::fs::create_dir_all(&meta_dir).ok();
    let json_path = meta_dir.join("validation.json");
    let json_str = serde_json::to_string_pretty(&report)?;
    std::fs::write(&json_path, &json_str)?;

    if json {
        println!("{}", json_str);
        return Ok(());
    }

    print_console_report(&report, &json_path);

    // Exit non-zero if any RED issue: lets CI / shell pipelines fail-fast.
    if report.red_count > 0 {
        std::process::exit(2);
    }
    Ok(())
}

// ─── Walking the docs tree ─────────────────────────────────────────────

fn collect_md_files(docs_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk(docs_dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        // Skip the meta directory — we don't validate cache or queue payloads.
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "_meta")
            .unwrap_or(false)
        {
            continue;
        }
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

fn relativize(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .map(|r| r.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| p.display().to_string())
}

// ─── Individual checks ─────────────────────────────────────────────────

/// RED — TODO / TBD / FIXME / XXX in the body. Symptom of an LLM that
/// echoed the prompt or a hand-written stub that was never finished.
fn check_residual_todos(content: &str, issues: &mut Vec<Issue>) {
    for (idx, line) in content.lines().enumerate() {
        // Skip code fences — TODO inside <code> is a real-world artifact, not
        // a prose problem.
        if line.trim_start().starts_with("```") {
            continue;
        }
        for marker in ["TODO", "TBD", "FIXME", "XXX"] {
            if line.contains(marker) {
                issues.push(Issue {
                    severity: Severity::Red,
                    kind: "residual_todo".to_string(),
                    line: Some(idx + 1),
                    detail: format!("contains marker `{}`", marker),
                });
                break; // one report per line is enough
            }
        }
    }
}

/// RED — `<!-- GNX:TIP:foo -->` comments still in the file mean enrichment
/// silently failed for that section. Word renders the comment as visible
/// HTML, so this leaks straight to the client.
fn check_unfilled_anchors(content: &str, issues: &mut Vec<Issue>) {
    for (idx, line) in content.lines().enumerate() {
        if let Some(start) = line.find("<!-- GNX:") {
            let after = &line[start..];
            if let Some(end) = after.find("-->") {
                let anchor = &after[..end + 3];
                issues.push(Issue {
                    severity: Severity::Red,
                    kind: "unfilled_anchor".to_string(),
                    line: Some(idx + 1),
                    detail: format!("anchor `{}` was never replaced by enrichment", anchor),
                });
            }
        }
    }
}

/// YELLOW — H1/H2 sections shorter than 50 words read as stubs in a Word
/// doc. Body text is counted between the heading and the next heading or
/// EOF, excluding code fences and tables.
fn check_short_sections(content: &str, issues: &mut Vec<Issue>) {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let level = heading_level(line);
        if level == 1 || level == 2 {
            let title = line.trim_start_matches('#').trim().to_string();
            let start = i + 1;
            let mut end = start;
            let mut in_code = false;
            while end < lines.len() {
                let l = lines[end];
                if l.trim_start().starts_with("```") {
                    in_code = !in_code;
                    end += 1;
                    continue;
                }
                if !in_code && heading_level(l) > 0 && heading_level(l) <= level {
                    break;
                }
                end += 1;
            }
            let words = lines[start..end]
                .iter()
                .filter(|l| !l.trim_start().starts_with("```") && !l.trim_start().starts_with('|'))
                .flat_map(|l| l.split_whitespace())
                .count();
            if words < 50 && !title.is_empty() {
                issues.push(Issue {
                    severity: Severity::Yellow,
                    kind: "short_section".to_string(),
                    line: Some(i + 1),
                    detail: format!("section `{}` has only {} words", title, words),
                });
            }
            i = end;
            continue;
        }
        i += 1;
    }
}

fn heading_level(line: &str) -> usize {
    let trimmed = line.trim_start();
    let mut n = 0;
    for c in trimmed.chars() {
        if c == '#' {
            n += 1;
        } else {
            break;
        }
    }
    if n > 0 && trimmed[n..].starts_with(' ') {
        n
    } else {
        0
    }
}

/// RED — relative `[label](file.md)` links that don't resolve against the
/// docs tree. External http/https links are skipped (out of scope), as are
/// in-page anchors (`#section`).
fn check_broken_internal_links(
    page_path: &Path,
    content: &str,
    known_files: &[PathBuf],
    issues: &mut Vec<Issue>,
) {
    let parent = page_path.parent().unwrap_or_else(|| Path::new("."));
    for (idx, line) in content.lines().enumerate() {
        // Crude single-link-per-line scan — markdown link parsing is not
        // worth a full crate dep for a linter.
        let bytes = line.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b']' && bytes[i + 1] == b'(' {
                // Find closing paren
                let url_start = i + 2;
                if let Some(rel_close) = line[url_start..].find(')') {
                    let url = &line[url_start..url_start + rel_close];
                    // Strip in-page anchor and title attribute
                    let clean = url.split('#').next().unwrap_or(url);
                    let clean = clean.split_whitespace().next().unwrap_or(clean);
                    if !clean.is_empty()
                        && !clean.starts_with("http://")
                        && !clean.starts_with("https://")
                        && !clean.starts_with("mailto:")
                        && clean.ends_with(".md")
                    {
                        let target = parent.join(clean);
                        let canonical = target.canonicalize().ok();
                        let resolves = canonical
                            .map(|c| known_files.iter().any(|k| k.canonicalize().ok() == Some(c.clone())))
                            .unwrap_or(false);
                        if !resolves {
                            issues.push(Issue {
                                severity: Severity::Red,
                                kind: "broken_link".to_string(),
                                line: Some(idx + 1),
                                detail: format!("link target `{}` does not exist", clean),
                            });
                        }
                    }
                    i = url_start + rel_close + 1;
                    continue;
                }
            }
            i += 1;
        }
    }
}

/// YELLOW — Alise v1.1 méthodo requires a `§4 Algorithmes` section on
/// service and controller pages. We detect those pages by filename
/// (the generator names them `aspnet-services.md`, `aspnet-controllers.md`,
/// or anything matching `services`/`controllers` substring).
fn check_methodo_alise_section_4(page_path: &Path, content: &str, issues: &mut Vec<Issue>) {
    let stem = page_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !(stem.contains("service") || stem.contains("controller")) {
        return;
    }
    // Accept several spellings: "§4 Algorithmes", "Section 4 — Algorithmes",
    // or any heading containing "algorithm". Case-insensitive on the body
    // because LLMs sometimes drop the §.
    let lower = content.to_lowercase();
    let has_section_4 = lower.contains("§4")
        || lower.contains("section 4")
        || lower.contains("algorithm");
    if !has_section_4 {
        issues.push(Issue {
            severity: Severity::Yellow,
            kind: "missing_section_4".to_string(),
            line: None,
            detail: "Alise v1.1 méthodo requires §4 Algorithmes on service / controller pages"
                .to_string(),
        });
    }
}

// ─── Console report ────────────────────────────────────────────────────

fn print_console_report(report: &ValidationReport, json_path: &Path) {
    println!();
    println!(
        "{} Validation report for {}",
        "==".bold().blue(),
        report.repo.bold()
    );
    println!(
        "{} pages scanned, {} with issues  ({} red, {} yellow)",
        report.total_pages,
        report.pages_with_issues,
        report.red_count.to_string().red().bold(),
        report.yellow_count.to_string().yellow().bold(),
    );

    if report.pages.is_empty() {
        println!("{} No issues found — ready to ship.", "OK".green().bold());
        println!("Report written to: {}", json_path.display());
        return;
    }

    // Show top 10 worst pages with their issues, then summary by kind.
    let mut sorted = report.pages.clone();
    sorted.sort_by(|a, b| b.issues.len().cmp(&a.issues.len()));
    println!();
    for page in sorted.iter().take(10) {
        println!("  {} — {} issue(s)", page.path.bold(), page.issues.len());
        for iss in page.issues.iter().take(5) {
            let sev = match iss.severity {
                Severity::Red => "RED   ".red(),
                Severity::Yellow => "YELLOW".yellow(),
            };
            let line = iss
                .line
                .map(|n| format!("L{}", n))
                .unwrap_or_else(|| "-".to_string());
            println!("    [{}] {:>4} {}: {}", sev, line, iss.kind, iss.detail);
        }
        if page.issues.len() > 5 {
            println!("    ... +{} more", page.issues.len() - 5);
        }
    }
    if sorted.len() > 10 {
        println!();
        println!("  ... +{} more pages with issues", sorted.len() - 10);
    }

    println!();
    println!("Issue counts by kind:");
    for (kind, count) in &report.by_kind {
        println!("  {:>3}  {}", count, kind);
    }
    println!();
    println!("Full report: {}", json_path.display());

    if report.red_count > 0 {
        println!(
            "{} {} red-severity issues block delivery — fix or accept and re-run.",
            "BLOCK".red().bold(),
            report.red_count
        );
    }
}

# `gitnexus inject` Architecture

## Purpose

`gitnexus inject` adds small, generated fragments to existing enriched HTML pages
without regenerating the page and without overwriting hand edits. The command is
for review notes, trace excerpts, screenshots, and LLM output that should be
attached to generated documentation after `gitnexus generate html --enrich` has
already produced the page.

The command writes only inside GitNexus injection anchors and only inside
GitNexus-managed fragment blocks. Markup outside those regions is treated as
user-owned content.

## CLI

```bash
gitnexus inject <page>
  --anchor <section>
  --fragment <path|->
  --type <markdown|screenshot|llm-output>
  [--placement <append|prepend|before|after>]
  [--id <fragment-id>]
  [--title <title>]
  [--assets-dir <path>]
  [--manifest <path>]
  [--dry-run]
  [--replace-existing]
```

Arguments:

- `<page>`: Existing enriched HTML page to update.
- `--anchor`: Section slug matching a `GNX:inject:<section>` HTML comment.
- `--fragment`: Fragment file path, or `-` to read UTF-8 content from stdin.
- `--type`: Fragment renderer. Supported values are `markdown`, `screenshot`,
  and `llm-output`.
- `--placement`: Where to place a new fragment inside the anchor region.
  Defaults to `append`.
- `--id`: Stable fragment ID. Defaults to a slug built from anchor, type, and
  fragment source.
- `--title`: Optional heading, screenshot caption, or LLM result title.
- `--assets-dir`: Directory for copied screenshot assets. Defaults to an
  `assets/` directory next to the page.
- `--manifest`: Optional JSON manifest path. Defaults to
  `<output-root>/.gitnexus/injections.json` when the output root is known.
- `--dry-run`: Validate and print the planned operation without writing.
- `--replace-existing`: Replace a matching managed fragment block. Without this
  flag, duplicate fragment IDs are rejected.

Exit codes:

- `0`: Injection completed, or dry-run validation passed.
- `1`: Invalid CLI arguments or unsupported fragment type.
- `2`: Page, fragment, or anchor could not be found.
- `3`: Duplicate fragment ID found without `--replace-existing`.
- `4`: HTML parsing, asset copying, or atomic write failed.

## Anchor Format

Enriched pages expose injection points with stable HTML comment anchors:

```html
<!-- GNX:inject:summary -->
<section id="summary" class="gitnexus-section">
  ...
</section>
<!-- /GNX:inject:summary -->

<!-- GNX:inject:impact -->
<section id="impact" class="gitnexus-section">
  ...
</section>
<!-- /GNX:inject:impact -->
```

The `section` portion is a lowercase slug. Built-in sections should use the same
slugs as the generated page structure, for example `summary`, `impact`,
`callers`, `callees`, `trace`, `coverage`, `diagram`, `source`, and
`related-symbols`.

The user-facing `--anchor impact` argument resolves to:

```html
<!-- GNX:inject:impact -->
```

For documentation and templates, the generic marker can be described as:

```html
<!-- GNX:inject:section -->
```

where `section` is replaced by the concrete slug. Closing comments are required
so the command can bound the editable region:

```html
<!-- /GNX:inject:section -->
```

Generated fragments are wrapped in managed blocks:

```html
<!-- GNX:fragment:order-risk type="llm-output" hash="sha256:..." -->
<aside class="gnx-injection gnx-injection--llm-output" data-gnx-fragment-id="order-risk">
  ...
</aside>
<!-- /GNX:fragment:order-risk -->
```

Only bytes between matching `GNX:fragment:<id>` comments may be replaced. Anchor
contents that are not inside a managed fragment block are preserved.

## Fragment Types

### Markdown

`markdown` fragments read UTF-8 Markdown and render sanitized HTML.

Rules:

- Render headings, paragraphs, lists, tables, links, and fenced code blocks.
- Add `gnx-injection gnx-injection--markdown` wrapper classes.
- Prefix generated heading IDs with the fragment ID to avoid collisions.
- Strip scripts, inline event handlers, and unsafe URL schemes.
- Preserve code block language labels for syntax highlighting.

### Screenshot

`screenshot` fragments read an image path and inject a figure.

Rules:

- Accept `.png`, `.jpg`, `.jpeg`, and `.webp`.
- Copy the image to `--assets-dir` using a content-hash filename.
- Emit `<figure>`, `<img>`, and optional `<figcaption>`.
- Use `--title` as caption and alt text when supplied.
- Record original path, copied path, size, and hash in the manifest.

### LLM Output

`llm-output` fragments read Markdown plus optional provenance front matter.

Rules:

- Render with the Markdown renderer and
  `gnx-injection gnx-injection--llm-output` wrapper classes.
- Sanitize with the same policy as `markdown`.
- Preserve fenced code blocks and citations.
- Render compact provenance metadata when supplied.

Supported front matter:

```markdown
---
model: gpt-5.4
prompt_id: impact-review-2026-04-23
source: gitnexus ask "What changes if OrderService changes?"
---

Changing `OrderService` affects billing orchestration and the order summary page.
```

## Non-Overwrite Semantics

The command is append-safe by default:

1. Read the page as text.
2. Locate the requested `GNX:inject:<section>` start and closing comments.
3. Generate the fragment HTML and managed `GNX:fragment:<id>` wrapper.
4. Search only inside the anchor region for an existing fragment with the same ID.
5. If no matching fragment exists, insert the new block according to
   `--placement`.
6. If a matching fragment exists and `--replace-existing` is not set, fail with
   exit code `3`.
7. If a matching fragment exists and `--replace-existing` is set, replace only
   that managed fragment block.
8. Write through a temporary file in the page directory, then atomically rename.

The implementation should avoid whole-document pretty-printing. It should keep
unrelated whitespace, comments, generated markup, and user edits byte-identical
whenever possible.

## Rust Module Structure

CLI command:

```text
crates/gitnexus-cli/src/commands/inject.rs
crates/gitnexus-cli/src/commands/mod.rs
```

Library support:

```text
crates/gitnexus-output/src/inject/mod.rs
crates/gitnexus-output/src/inject/anchor.rs
crates/gitnexus-output/src/inject/assets.rs
crates/gitnexus-output/src/inject/fragment.rs
crates/gitnexus-output/src/inject/html.rs
crates/gitnexus-output/src/inject/manifest.rs
crates/gitnexus-output/src/inject/renderers/markdown.rs
crates/gitnexus-output/src/inject/renderers/screenshot.rs
crates/gitnexus-output/src/inject/renderers/llm_output.rs
```

Responsibilities:

- `commands/inject.rs`: Clap argument parsing, stdin handling, user diagnostics,
  and exit-code mapping.
- `anchor.rs`: Section slug validation and `GNX:inject:<section>` region
  discovery.
- `assets.rs`: Screenshot hash naming, asset copying, and relative URL
  generation.
- `fragment.rs`: Fragment IDs, placement rules, duplicate detection, and managed
  wrapper comments.
- `html.rs`: Text-preserving insertion, managed block replacement, temp-file
  writes, and atomic rename.
- `manifest.rs`: Read/write injection metadata, source hashes, copied assets,
  timestamps, and provenance.
- `renderers/*`: Type-specific conversion into sanitized HTML.

Suggested API:

```rust
pub struct InjectRequest {
    pub page: PathBuf,
    pub anchor: SectionAnchor,
    pub fragment: FragmentSource,
    pub fragment_type: FragmentType,
    pub placement: Placement,
    pub id: Option<String>,
    pub title: Option<String>,
    pub assets_dir: Option<PathBuf>,
    pub manifest: Option<PathBuf>,
    pub dry_run: bool,
    pub replace_existing: bool,
}

pub struct InjectResult {
    pub page: PathBuf,
    pub anchor: SectionAnchor,
    pub fragment_id: String,
    pub action: InjectAction,
    pub assets: Vec<PathBuf>,
    pub manifest: Option<PathBuf>,
}

pub enum InjectAction {
    Inserted,
    Replaced,
    DryRunInsert,
    DryRunReplace,
}

pub fn inject_fragment(request: InjectRequest) -> anyhow::Result<InjectResult>;
```

## Examples

Append a Markdown review note to the impact section:

```bash
gitnexus inject docs/out/context/OrderService.html \
  --anchor impact \
  --fragment notes/order-service-impact.md \
  --type markdown \
  --title "Review notes"
```

Inject a screenshot into a trace section:

```bash
gitnexus inject docs/out/controllers/HomeController.html \
  --anchor trace \
  --fragment screenshots/home-trace.png \
  --type screenshot \
  --title "HomeController trace"
```

Pipe LLM output into an existing page:

```bash
gitnexus ask "What breaks if OrderService changes?" | \
  gitnexus inject docs/out/context/OrderService.html \
    --anchor impact \
    --fragment - \
    --type llm-output \
    --id order-service-impact-answer
```

Replace a previous managed fragment explicitly:

```bash
gitnexus inject docs/out/context/OrderService.html \
  --anchor impact \
  --fragment notes/order-service-impact-v2.md \
  --type markdown \
  --id order-service-impact-answer \
  --replace-existing
```

## Verification Plan

- Unit-test anchor slug parsing and `GNX:inject:<section>` region discovery.
- Unit-test duplicate fragment detection with and without
  `--replace-existing`.
- Snapshot-test generated wrappers for all fragment types.
- Snapshot-test text preservation around unrelated markup.
- Integration-test insertion into enriched HTML containing user edits before,
  inside, and after an anchor region.
- Integration-test screenshot asset copying and manifest updates.
- Dry-run tests must prove page content, asset directories, and manifest files
  are unchanged.

# RAG Integration Plan: The "GraphRAG" Hybrid

## 1. Goal & Vision
Transform GitNexus from a pure structural code analysis tool into a **Context-Aware Enterprise Architecture Assistant**. 

Currently, GitNexus understands the *"How"* (how components interact, call graphs, dependencies). By integrating a Retrieval-Augmented Generation (RAG) system targeting external documentation (Specs, Jira, Architecture Decisions), we provide the *"Why"*.

The core innovation is **GraphRAG**: We will not just build a semantic search engine next to the code graph; we will **merge them**. External documents will be chunked, embedded, and physically linked (via edges) to the actual code symbols they describe in the KuzuDB graph.

## 2. Architecture & Backend Changes

### 2.1 New Ingestion Phase (`gitnexus-ingest`)
We will introduce a new pipeline phase dedicated to external knowledge ingestion.

*   **Supported Sources:** Local directories containing Markdown (`.md`), PDF (`.pdf`), and Word (`.docx`) files.
*   **Text Extraction & Chunking:**
    *   Use Rust crates (e.g., `pdf-extract`, `docx-rs`) to extract raw text.
    *   Implement smart chunking (e.g., splitting Markdown by Headers `##`, or by paragraphs for PDFs) to keep context concise for the LLM.
*   **Vectorization (Embeddings):** Reuse the existing ONNX-based semantic search engine (`gitnexus-search/embeddings`) to generate vector embeddings for each document chunk.
*   **Semantic Anchoring (The Graph Link):** 
    *   Run a Named Entity Recognition (NER) or simple keyword matching pass on the document chunks against the known symbols in the codebase (Class names, Method names, domain terms).
    *   Create new nodes in the KuzuDB graph: `(d:Document)` and `(c:DocChunk)`.
    *   Create relationships: `(c)-[:MENTIONS]->(s:Symbol)`.

### 2.2 Database Schema Updates (`gitnexus-db`)
Add new entity types to the graph schema:
*   `Document`: Represents a physical file (e.g., `Specs_V2.pdf`). Properties: `path`, `title`, `type`.
*   `DocChunk`: Represents a semantic block of text. Properties: `content`, `page_number`, `embedding` (vector).
*   Edges: `BELONGS_TO` (Chunk -> Document), `MENTIONS` (Chunk -> Symbol).

### 2.3 New MCP Tools (`gitnexus-mcp`)
To allow the AI agent to utilize this new knowledge, we need new tools:
*   `search_knowledge_base`: Performs a semantic vector search across all `DocChunk` nodes based on a natural language query.
*   `get_docs_for_symbol`: Given a specific code `symbol_id`, traverses the `MENTIONS` edges backwards to retrieve all relevant documentation chunks.

## 3. Desktop Application Updates (Frontend)

The Tauri/React desktop application (`gitnexus-desktop/ui`) will be updated to surface this new knowledge:

*   **Manage Mode (⚙️):**
    *   Add a new "External Knowledge" or "Documentation Sources" tab.
    *   Allow users to specify local folder paths containing their specs/documentation.
    *   Trigger an incremental re-index of these documents.
*   **Analyze Mode (📊):**
    *   Introduce a "Documentation Coverage" metric: *What percentage of our critical code (e.g., highly coupled files) is actually described in the external documentation?*
*   **Explorer Mode (🌐):**
    *   When clicking a node (Class/Method) in the 3D graph, the right-hand details panel will feature a new "Linked Docs" section, automatically surfacing the specific paragraphs from PDFs/Word docs that mention this code.
*   **Chat Mode (💬):**
    *   The AI will seamlessly use the new MCP tools. When asked *"Why does the billing service exclude suppliers?"*, it will search the RAG, find the business rule in a PDF chunk, traverse the graph to find the implementing method, and provide a verified answer with citations to both the code and the external document.

## 4. Implementation Phases

*   **Phase 1: Foundation (Backend)**
    *   Define the new graph schema (`Document`, `DocChunk`).
    *   Implement basic Markdown/text file ingestion and chunking in `gitnexus-ingest`.
    *   Generate embeddings and store them in the graph.
*   **Phase 2: Semantic Anchoring & Tools**
    *   Implement the logic to link `DocChunk` to `Symbol` based on name matching.
    *   Develop the `search_knowledge_base` and `get_docs_for_symbol` MCP tools.
*   **Phase 3: Rich Formats & UI**
    *   Add support for PDF and DOCX extraction.
    *   Build the frontend UI in the Desktop app (Settings tab, Linked Docs panel in the Explorer).

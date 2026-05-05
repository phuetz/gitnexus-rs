const MCP_BASE_URL = import.meta.env.VITE_MCP_URL ?? '';

export interface RepoInfo {
  name: string;
  path?: string;
  indexedAt?: string;
  lastCommit?: string;
  stats?: Record<string, number>;
}

export interface ChatHistoryMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export class ChatStreamError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ChatStreamError';
  }
}

interface JsonRpcResponse<T> {
  jsonrpc: '2.0';
  id: number | string | null;
  result?: T;
  error?: { code: number; message: string; data?: unknown };
}

/**
 * MCP tool envelope. Tools return a Markdown summary in `content` for the LLM
 * and may attach typed structured data in `_meta` for direct UI consumption.
 */
export interface McpToolResult {
  content: Array<{ type: string; text: string }>;
  _meta?: Record<string, unknown>;
}

/**
 * `_meta` payload for `list_sfd_pages`.
 */
export interface SfdPagesMeta {
  pages: string[];
  drafts: string[];
  docsDir: string;
  missing: boolean;
}

/**
 * `_meta` payload for `write_sfd_draft`.
 */
export interface SfdDraftWrittenMeta {
  path: string;
  bytes: number;
}

/**
 * Severity tag from `gitnexus-rag::validator::Severity`.
 */
export type SfdValidationSeverity = 'red' | 'yellow';

/**
 * Per-issue diagnostic, mirrors `gitnexus-rag::validator::Issue`.
 */
export interface SfdValidationIssue {
  severity: SfdValidationSeverity;
  kind: string;
  line?: number;
  detail: string;
}

/**
 * Per-page report, mirrors `gitnexus-rag::validator::PageReport`.
 */
export interface SfdValidationPageReport {
  path: string;
  issues: SfdValidationIssue[];
}

/**
 * Full validation report, mirrors `gitnexus-rag::validator::ValidationReport`.
 */
export interface SfdValidationReport {
  repo: string;
  generated_at: string;
  total_pages: number;
  pages_with_issues: number;
  red_count: number;
  yellow_count: number;
  by_kind: Record<string, number>;
  pages: SfdValidationPageReport[];
}

/**
 * `_meta` payload for `validate_sfd`.
 */
export interface SfdValidateMeta {
  report: SfdValidationReport;
  status: 'green' | 'yellow' | 'red';
}

/**
 * Tool-call lifecycle event surfaced by the chat stream. The Rust side emits
 * one `phase: "start"` per tool dispatch followed by a `phase: "end"` once
 * the backend returned (or failed). The chat-ui collects them and renders a
 * "🔍 Exécute search_code…" / "✓ search_code" badge on the assistant
 * bubble.
 */
export type ToolCallStreamEvent =
  | { phase: 'start'; id: string; name: string; args: string }
  | { phase: 'end'; id: string; name: string; success: boolean };

export class MCPClient {
  readonly baseUrl: string;
  readonly token?: string;

  constructor(baseUrl: string = MCP_BASE_URL, token?: string) {
    this.baseUrl = baseUrl;
    this.token = token ?? import.meta.env.VITE_MCP_TOKEN;
  }

  private headers(extra: Record<string, string> = {}): HeadersInit {
    const h: Record<string, string> = { 'Content-Type': 'application/json', ...extra };
    if (this.token) h['Authorization'] = `Bearer ${this.token}`;
    return h;
  }

  async health(): Promise<{ status: string; service: string; version: string }> {
    const res = await fetch(`${this.baseUrl}/health`, { headers: this.headers() });
    if (!res.ok) throw new Error(`Health check failed: ${res.status}`);
    return res.json();
  }

  async listRepos(): Promise<RepoInfo[]> {
    const res = await fetch(`${this.baseUrl}/api/repos`, { headers: this.headers() });
    if (!res.ok) throw new Error(`list_repos failed: ${res.status}`);
    const data = await res.json();
    return Array.isArray(data?.repos) ? data.repos : [];
  }

  /**
   * Invoke an MCP tool by name through the JSON-RPC `tools/call` method.
   * Returns the parsed `result` envelope: `{ content: [...], _meta?: {...} }`.
   * Throws on transport / auth / JSON-RPC error so callers don't need to
   * inspect status fields.
   */
  async callTool<T = McpToolResult>(name: string, args: Record<string, unknown> = {}): Promise<T> {
    const res = await fetch(`${this.baseUrl}/mcp`, {
      method: 'POST',
      headers: this.headers(),
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: Date.now(),
        method: 'tools/call',
        params: { name, arguments: args },
      }),
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      throw new Error(`callTool ${name}: ${res.status} ${res.statusText} ${body}`);
    }

    const envelope = (await res.json()) as JsonRpcResponse<T>;
    if (envelope.error) {
      throw new Error(`callTool ${name}: ${envelope.error.message ?? 'unknown error'}`);
    }
    if (!envelope.result) {
      throw new Error(`callTool ${name}: empty result`);
    }
    return envelope.result;
  }

  async chatStream(
    repo: string,
    question: string,
    history: ChatHistoryMessage[],
    onDelta: (text: string) => void,
    signal?: AbortSignal,
    onToolCall?: (event: ToolCallStreamEvent) => void
  ): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/chat`, {
      method: 'POST',
      headers: this.headers({ Accept: 'text/event-stream' }),
      body: JSON.stringify({ question, repo, history }),
      signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      throw new Error(`chat failed: ${res.status} ${res.statusText} ${body}`);
    }
    if (!res.body) throw new Error('chat: no response body');

    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    let eventType: string | null = null;
    let dataLines: string[] = [];

    const flush = () => {
      if (dataLines.length === 0) {
        eventType = null;
        return;
      }
      const payload = dataLines.join('\n');
      dataLines = [];
      const ev = eventType;
      eventType = null;

      if (payload === '[DONE]') {
        throw new SseDone();
      }
      if (ev === 'error') {
        throw new ChatStreamError(payload.replace(/^Error:\s*/i, ''));
      }
      if (ev === 'tool_call' && onToolCall) {
        try {
          const evt = JSON.parse(payload) as ToolCallStreamEvent;
          onToolCall(evt);
        } catch {
          // Ignore malformed tool_call events — they're decorative.
        }
        return;
      }

      try {
        const obj = JSON.parse(payload);
        const text =
          (typeof obj === 'string' && obj) ||
          obj?.delta ||
          obj?.text ||
          obj?.content ||
          obj?.choices?.[0]?.delta?.content ||
          '';
        if (text) onDelta(String(text));
      } catch {
        onDelta(payload);
      }
    };

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) {
          buffer += '\n\n';
        } else {
          buffer += decoder.decode(value, { stream: true });
        }

        let nl: number;
        while ((nl = buffer.indexOf('\n')) !== -1) {
          const rawLine = buffer.slice(0, nl).replace(/\r$/, '');
          buffer = buffer.slice(nl + 1);

          if (rawLine === '') {
            flush();
            continue;
          }

          if (rawLine.startsWith(':')) continue;

          const colon = rawLine.indexOf(':');
          const field = colon === -1 ? rawLine : rawLine.slice(0, colon);
          let val = colon === -1 ? '' : rawLine.slice(colon + 1);
          if (val.startsWith(' ')) val = val.slice(1);

          if (field === 'data') dataLines.push(val);
          else if (field === 'event') eventType = val;
        }

        if (done) return;
      }
    } catch (e) {
      if (e instanceof SseDone) return;
      throw e;
    }
  }
}

class SseDone extends Error {
  constructor() {
    super('done');
    this.name = 'SseDone';
  }
}

export const mcpClient = new MCPClient();

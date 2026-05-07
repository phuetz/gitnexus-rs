const MCP_BASE_URL = import.meta.env.VITE_MCP_URL ?? '';

export interface RepoInfo {
  id?: string;
  name: string;
  path?: string;
  indexedAt?: string;
  lastCommit?: string;
  stats?: Record<string, number>;
}

export interface LlmConfigInfo {
  configured: boolean;
  provider?: string;
  model?: string;
  reasoningEffort?: string;
  maxTokens?: number;
  bigContextModel?: string;
}

export interface DiagnosticsRepoInfo {
  id: string;
  name: string;
  indexedAt?: string;
  pathExposed: boolean;
}

export interface ChatGptOAuthDiagnostics {
  loggedIn: boolean;
  status: 'logged_in' | 'missing' | 'incomplete' | 'invalid' | 'unreadable' | string;
  tokenFilePresent: boolean;
  tokenFileReadable: boolean;
  refreshTokenPresent: boolean;
  lastRefresh?: string | null;
  storage: string;
  errorKind?: string;
}

export interface DiagnosticsInfo {
  service: string;
  version: string;
  generatedAtUnixMs: number;
  httpAuthRequired: boolean;
  repoPathsExposed: boolean;
  repos: {
    count: number;
    names: DiagnosticsRepoInfo[];
  };
  llm: LlmConfigInfo;
  auth?: {
    chatgptOAuth?: ChatGptOAuthDiagnostics;
  };
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

  private backendLabel(): string {
    return this.baseUrl || 'le proxy Vite courant';
  }

  private async request(path: string, init: RequestInit, action: string): Promise<Response> {
    let res: Response;
    try {
      res = await fetch(`${this.baseUrl}${path}`, init);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') {
        throw e;
      }
      const reason = e instanceof Error ? e.message : String(e);
      throw new Error(
        `${action}: serveur GitNexus injoignable via ${this.backendLabel()}. Vérifie que le backend tourne et que VITE_MCP_URL pointe vers lui. Lance aussi \`.\\gitnexus.cmd doctor\` pour contrôler les ports. (${reason})`,
        { cause: e }
      );
    }

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      const hint =
        res.status >= 500
          ? ` Vérifie ${this.backendLabel()} avec \`.\\gitnexus.cmd doctor\` et relance le chat avec -RestartBackend si besoin.`
          : '';
      throw new Error(
        `${action}: HTTP ${res.status}${res.statusText ? ` ${res.statusText}` : ''}.${hint}${formatErrorBody(body)}`
      );
    }

    return res;
  }

  async health(): Promise<{ status: string; service: string; version: string }> {
    const res = await this.request('/health', { headers: this.headers() }, 'health');
    return res.json();
  }

  async listRepos(): Promise<RepoInfo[]> {
    const res = await this.request('/api/repos', { headers: this.headers() }, 'list_repos');
    const data = await res.json();
    return Array.isArray(data?.repos) ? data.repos : [];
  }

  async llmConfig(): Promise<LlmConfigInfo> {
    const res = await this.request('/api/llm-config', { headers: this.headers() }, 'llm_config');
    return res.json();
  }

  async diagnostics(): Promise<DiagnosticsInfo> {
    const res = await this.request(
      '/api/diagnostics',
      { headers: this.headers() },
      'diagnostics'
    );
    return res.json();
  }

  /**
   * Invoke an MCP tool by name through the JSON-RPC `tools/call` method.
   * Returns the parsed `result` envelope: `{ content: [...], _meta?: {...} }`.
   * Throws on transport / auth / JSON-RPC error so callers don't need to
   * inspect status fields.
   */
  async callTool<T = McpToolResult>(name: string, args: Record<string, unknown> = {}): Promise<T> {
    const res = await this.request(
      '/mcp',
      {
        method: 'POST',
        headers: this.headers(),
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: Date.now(),
          method: 'tools/call',
          params: { name, arguments: args },
        }),
      },
      `callTool ${name}`
    );

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
    const res = await this.request(
      '/api/chat',
      {
        method: 'POST',
        headers: this.headers({ Accept: 'text/event-stream' }),
        body: JSON.stringify({ question, repo, history }),
        signal,
      },
      'chat'
    );
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

function formatErrorBody(body: string): string {
  const trimmed = sanitizeErrorBody(body.trim());
  if (!trimmed) return '';
  const singleLine = trimmed.replace(/\s+/g, ' ');
  const truncated =
    singleLine.length > 300 ? `${singleLine.slice(0, 300)}...` : singleLine;
  return ` Réponse: ${truncated}`;
}

function sanitizeErrorBody(body: string): string {
  return body
    .replace(/\bBearer\s+[A-Za-z0-9._~+/=-]{8,}/gi, 'Bearer [redacted]')
    .replace(/\bsk-[A-Za-z0-9_-]{12,}\b/g, '[redacted-openai-key]')
    .replace(/\bAIza[A-Za-z0-9_-]{20,}\b/g, '[redacted-google-key]')
    .replace(/\bya29\.[A-Za-z0-9._-]{12,}\b/g, '[redacted-google-token]')
    .replace(
      /\b(api[_-]?key|access[_-]?token|refresh[_-]?token|authorization)(["'\s:=]+)([A-Za-z0-9._~+/=-]{8,})/gi,
      '$1$2[redacted]'
    );
}

export const mcpClient = new MCPClient();

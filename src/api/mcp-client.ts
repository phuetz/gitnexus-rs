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

  async chatStream(
    repo: string,
    question: string,
    history: ChatHistoryMessage[],
    onDelta: (text: string) => void,
    signal?: AbortSignal
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

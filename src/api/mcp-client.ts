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

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() ?? '';

      for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line.startsWith('data:')) continue;
        const payload = line.slice(5).trim();
        if (payload === '[DONE]') return;
        if (!payload) continue;

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
          if (payload && payload !== '[DONE]') onDelta(payload);
        }
      }
    }
  }
}

export const mcpClient = new MCPClient();

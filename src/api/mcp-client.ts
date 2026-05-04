import type { MCPTool } from '../types/chat';

const MCP_BASE_URL = import.meta.env.VITE_MCP_URL ?? 'http://localhost:8080';

export class MCPClient {
  readonly baseUrl: string;

  constructor(baseUrl: string = MCP_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  async listTools(): Promise<MCPTool[]> {
    return MOCK_TOOLS;
  }

  async callTool(name: string, args: Record<string, unknown>): Promise<unknown> {
    await delay(300);
    return { mock: true, tool: name, args };
  }

  async chat(userMessage: string): Promise<string> {
    await delay(800);
    return `**Mock response** — tu as dit :\n\n> ${userMessage}\n\nEn V1 ce sera relié au backend \`gitnexus-mcp\` (\`gitnexus serve --http 8080\`) avec streaming SSE et appels d'outils réels.`;
  }
}

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

const MOCK_TOOLS: MCPTool[] = [
  { name: 'search_code', description: 'Hybrid BM25 + semantic search across the indexed code graph', inputSchema: {} },
  { name: 'read_file', description: 'Read a file slice (max 50 lines)', inputSchema: {} },
  { name: 'impact', description: 'Blast-radius analysis on a symbol', inputSchema: {} },
  { name: 'cypher', description: 'Run a read-only Cypher query', inputSchema: {} },
];

export const mcpClient = new MCPClient();

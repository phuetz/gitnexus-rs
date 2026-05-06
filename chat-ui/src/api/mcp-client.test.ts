import { afterEach, describe, expect, it, vi } from 'vitest';
import { MCPClient } from './mcp-client';

describe('MCPClient errors', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('adds a backend hint for HTTP 5xx responses', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('Bad Gateway', { status: 502, statusText: 'Bad Gateway' }))
    );

    await expect(new MCPClient('').listRepos()).rejects.toThrow(
      /list_repos: HTTP 502 Bad Gateway.*gitnexus serve --port 3010.*Bad Gateway/
    );
  });

  it('turns fetch failures into actionable backend messages', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new TypeError('Failed to fetch');
      })
    );

    await expect(new MCPClient('').llmConfig()).rejects.toThrow(
      /llm_config: serveur GitNexus injoignable.*VITE_MCP_URL.*Failed to fetch/
    );
  });

  it('preserves AbortError so chat cancellation stays distinct', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new DOMException('The operation was aborted.', 'AbortError');
      })
    );

    await expect(new MCPClient('').llmConfig()).rejects.toMatchObject({
      name: 'AbortError',
    });
  });
});

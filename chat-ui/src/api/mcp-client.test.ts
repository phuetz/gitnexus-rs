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

  it('loads safe runtime diagnostics from the backend', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(
          JSON.stringify({
            service: 'gitnexus',
            version: '0.1.0',
            generatedAtUnixMs: 1778000000000,
            httpAuthRequired: false,
            repoPathsExposed: false,
            repos: {
              count: 1,
              names: [
                {
                  id: 'repo_alise',
                  name: 'Alise_v2',
                  indexedAt: '2026-05-06T05:00:00Z',
                  pathExposed: false,
                },
              ],
            },
            llm: {
              configured: true,
              provider: 'chatgpt',
              model: 'gpt-5.5',
              reasoningEffort: 'high',
              maxTokens: 8192,
            },
            auth: {
              chatgptOAuth: {
                loggedIn: true,
                status: 'logged_in',
                tokenFilePresent: true,
                tokenFileReadable: true,
                refreshTokenPresent: true,
                lastRefresh: '2026-05-06T20:00:00Z',
                storage: 'dpapi',
              },
            },
          }),
          { status: 200, headers: { 'content-type': 'application/json' } }
        )
      )
    );

    const diagnostics = await new MCPClient('').diagnostics();

    expect(diagnostics.service).toBe('gitnexus');
    expect(diagnostics.repoPathsExposed).toBe(false);
    expect(diagnostics.repos.count).toBe(1);
    expect(diagnostics.llm.provider).toBe('chatgpt');
    expect(diagnostics.llm.model).toBe('gpt-5.5');
    expect(diagnostics.auth?.chatgptOAuth?.loggedIn).toBe(true);
  });
});

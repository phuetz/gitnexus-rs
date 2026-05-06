import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SystemDiagnostics } from './SystemDiagnostics';
import { useChatStore } from '../../stores/chat-store';

function jsonResponse(body: unknown, init?: ResponseInit) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
    ...init,
  });
}

describe('SystemDiagnostics', () => {
  beforeEach(() => {
    localStorage.clear();
    useChatStore.setState({
      selectedRepo: 'repo_alise',
      selectedRepoName: 'Alise_v2',
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('shows backend, project and LLM diagnostics on demand', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith('/api/diagnostics')) {
        return jsonResponse({
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
        });
      }
      return jsonResponse({}, { status: 404 });
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<SystemDiagnostics />);
    fireEvent.click(screen.getByRole('button', { name: /ouvrir le diagnostic système/i }));

    await waitFor(() => {
      expect(screen.getByText('Diagnostic GitNexus')).toBeTruthy();
      expect(screen.getByText('gitnexus 0.1.0')).toBeTruthy();
      expect(screen.getByText('Alise_v2')).toBeTruthy();
      expect(screen.getByText('chatgpt')).toBeTruthy();
      expect(screen.getByText('gpt-5.5')).toBeTruthy();
      expect(screen.getByText('masqués')).toBeTruthy();
      expect(screen.getByText('connecté')).toBeTruthy();
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('keeps backend failures actionable inside the panel', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('Bad Gateway', { status: 502, statusText: 'Bad Gateway' }))
    );

    render(<SystemDiagnostics />);
    fireEvent.click(screen.getByRole('button', { name: /ouvrir le diagnostic système/i }));

    await waitFor(() => {
      expect(screen.getByText('Diagnostic indisponible')).toBeTruthy();
      expect(screen.getByText(/HTTP 502 Bad Gateway/)).toBeTruthy();
    });
  });
});

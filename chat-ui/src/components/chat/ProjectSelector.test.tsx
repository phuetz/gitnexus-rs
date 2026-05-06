import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ProjectSelector } from './ProjectSelector';
import { useChatStore } from '../../stores/chat-store';

function jsonResponse(body: unknown, init?: ResponseInit) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
    ...init,
  });
}

describe('ProjectSelector', () => {
  beforeEach(() => {
    localStorage.clear();
    useChatStore.setState({
      selectedRepo: null,
      selectedRepoName: null,
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('auto-selects the first repo without refetching because selection changed', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith('/api/repos')) {
        return jsonResponse({
          repos: [
            {
              name: 'Alise_v2',
              path: 'D:/Repos/Alise_v2',
              indexedAt: '2026-05-06T05:00:00Z',
            },
          ],
        });
      }
      return jsonResponse({}, { status: 404 });
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<ProjectSelector />);

    await waitFor(() => {
      expect(useChatStore.getState().selectedRepo).toBe('Alise_v2');
      expect(useChatStore.getState().selectedRepoName).toBe('Alise_v2');
      expect(screen.getByText('Alise_v2')).toBeTruthy();
    });

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('uses the backend repo id when duplicate names exist', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith('/api/repos')) {
          return jsonResponse({
            repos: [
              {
                id: 'repo_first',
                name: 'gitnexus-rs',
                indexedAt: '2026-05-06T05:00:00Z',
              },
              {
                id: 'repo_second',
                name: 'gitnexus-rs',
                indexedAt: '2026-05-06T06:00:00Z',
              },
            ],
          });
        }
        return jsonResponse({}, { status: 404 });
      })
    );

    render(<ProjectSelector />);

    await waitFor(() => {
      expect(useChatStore.getState().selectedRepo).toBe('repo_first');
      expect(useChatStore.getState().selectedRepoName).toBe('gitnexus-rs · first');
      expect(screen.getByText('gitnexus-rs · first')).toBeTruthy();
    });
  });

  it('migrates a persisted repo name to its stable backend id', async () => {
    useChatStore.setState({ selectedRepo: 'Alise_v2' });
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith('/api/repos')) {
          return jsonResponse({
            repos: [
              {
                id: 'repo_alise',
                name: 'Alise_v2',
                indexedAt: '2026-05-06T05:00:00Z',
              },
            ],
          });
        }
        return jsonResponse({}, { status: 404 });
      })
    );

    render(<ProjectSelector />);

    await waitFor(() => {
      expect(useChatStore.getState().selectedRepo).toBe('repo_alise');
      expect(useChatStore.getState().selectedRepoName).toBe('Alise_v2');
      expect(screen.getByText('Alise_v2')).toBeTruthy();
    });
  });
});

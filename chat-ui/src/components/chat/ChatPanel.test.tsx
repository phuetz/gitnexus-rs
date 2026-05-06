import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ChatPanel } from './ChatPanel';
import { useChatStore } from '../../stores/chat-store';

function jsonResponse(body: unknown, init?: ResponseInit) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
    ...init,
  });
}

describe('ChatPanel', () => {
  beforeEach(() => {
    localStorage.clear();
    useChatStore.setState({
      sessions: [],
      currentSessionId: null,
      isStreaming: false,
      selectedRepo: null,
      selectedRepoName: null,
      inputDraft: '',
      isSfdPanelOpen: false,
    });

    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith('/health')) {
          return jsonResponse({ status: 'ok', service: 'gitnexus', version: 'test' });
        }
        if (url.endsWith('/api/repos')) {
          return jsonResponse({ repos: [] });
        }
        if (url.endsWith('/api/llm-config')) {
          return jsonResponse({
            configured: true,
            provider: 'chatgpt',
            model: 'gpt-5.5',
            reasoningEffort: 'high',
            maxTokens: 8192,
          });
        }
        return jsonResponse({}, { status: 404 });
      })
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('renders without entering a recursive store update loop', () => {
    render(<ChatPanel />);

    expect(screen.getAllByText('GitNexus Chat').length).toBeGreaterThan(0);
    expect(screen.getByText('Aucune conversation. Crée-en une pour démarrer.')).toBeTruthy();
  });
});

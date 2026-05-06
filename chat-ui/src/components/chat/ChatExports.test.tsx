import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { LlmConfigState } from '../../hooks/use-llm-config';
import { useChatStore } from '../../stores/chat-store';
import { ChatExports } from './ChatExports';

function readyLlm(): LlmConfigState {
  return {
    status: 'ready',
    config: {
      configured: true,
      provider: 'chatgpt',
      model: 'gpt-5.5',
      reasoningEffort: 'high',
      maxTokens: 8192,
    },
    message: 'chatgpt / gpt-5.5',
    refresh: vi.fn(async () => undefined),
  };
}

describe('ChatExports', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    localStorage.clear();
    useChatStore.setState({
      sessions: [
        {
          id: 's1',
          title: 'Trace courrier',
          createdAt: 1774507049000,
          updatedAt: 1774507059000,
          messages: [
            {
              id: 'm1',
              role: 'user',
              content: 'Trace le flux',
              createdAt: 1774507049000,
            },
            {
              id: 'm2',
              role: 'assistant',
              content: '```mermaid\nflowchart TD\nA --> B\n```',
              createdAt: 1774507059000,
              toolCalls: [
                { id: 't1', name: 'search_code', args: { query: 'courrier' }, status: 'done' },
              ],
            },
          ],
        },
      ],
      currentSessionId: 's1',
      isStreaming: false,
      selectedRepo: 'repo_alise',
      selectedRepoName: 'Alise_v2',
      inputDraft: '',
      isSfdPanelOpen: false,
    });
  });

  it('copies the full Markdown transcript with repo, model, tools, and diagrams', async () => {
    const writeText = vi.fn(async (text: string) => {
      void text;
    });
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });

    render(<ChatExports llm={readyLlm()} />);

    fireEvent.click(screen.getByRole('button', { name: /copier la conversation en markdown/i }));

    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1));
    const markdown = writeText.mock.calls[0][0];
    expect(markdown).toContain('# Trace courrier');
    expect(markdown).toContain('- Projet: Alise_v2');
    expect(markdown).toContain('- LLM: chatgpt / gpt-5.5, raisonnement high, max 8192 tokens');
    expect(markdown).toContain('_Outils: search_code (done)_');
    expect(markdown).toContain('```mermaid');
    expect(screen.getByRole('button', { name: /conversation markdown copiée/i })).toBeTruthy();
  });

  it('shows a non-blocking error when Markdown cannot be copied', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error('denied')) },
    });
    Object.defineProperty(document, 'execCommand', {
      configurable: true,
      value: undefined,
    });
    const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => undefined);

    render(<ChatExports llm={readyLlm()} />);

    fireEvent.click(screen.getByRole('button', { name: /copier la conversation en markdown/i }));

    expect(await screen.findByText('Copie impossible')).toBeTruthy();
    expect(alertSpy).not.toHaveBeenCalled();
  });

  it('shows a non-blocking error when PDF export is blocked', async () => {
    const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => undefined);
    vi.spyOn(window, 'open').mockReturnValue(null);

    render(<ChatExports llm={readyLlm()} />);

    fireEvent.click(screen.getByRole('button', { name: /exporter la conversation en pdf/i }));

    expect(await screen.findByText(/fenêtre d’export pdf/i)).toBeTruthy();
    expect(alertSpy).not.toHaveBeenCalled();
  });
});

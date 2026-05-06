import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mcpClient } from '../api/mcp-client';
import { useChatStore } from '../stores/chat-store';
import { useChat } from './use-chat';

function SendProbe() {
  const { sendMessage } = useChat();
  return (
    <button type="button" onClick={() => void sendMessage('Trace le flux courrier')}>
      Envoyer
    </button>
  );
}

describe('useChat', () => {
  beforeEach(() => {
    localStorage.clear();
    useChatStore.setState({
      sessions: [],
      currentSessionId: null,
      isStreaming: false,
      selectedRepo: 'repo_alise',
      selectedRepoName: 'Alise_v2',
      inputDraft: '',
      isSfdPanelOpen: false,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('keeps transport errors actionable for the local chat launcher', async () => {
    vi.spyOn(mcpClient, 'chatStream').mockRejectedValue(new Error('backend down'));

    render(<SendProbe />);
    fireEvent.click(screen.getByRole('button', { name: 'Envoyer' }));

    await waitFor(() => {
      const session = useChatStore.getState().sessions[0];
      const assistant = session?.messages.find((message) => message.role === 'assistant');
      expect(assistant?.content).toContain('.\\gitnexus.cmd doctor');
      expect(assistant?.content).toContain('.\\gitnexus.cmd chat -RestartBackend');
      expect(useChatStore.getState().isStreaming).toBe(false);
    });
  });

  it('renames an empty default session from the first user message', async () => {
    useChatStore.setState({
      sessions: [
        {
          id: 's1',
          title: 'Nouvelle conversation',
          createdAt: 1778000000000,
          updatedAt: 1778000000000,
          messages: [],
        },
      ],
      currentSessionId: 's1',
    });
    vi.spyOn(mcpClient, 'chatStream').mockImplementation(
      async (_repo, _question, _history, onDelta) => {
        onDelta('Réponse OK');
      }
    );

    render(<SendProbe />);
    fireEvent.click(screen.getByRole('button', { name: 'Envoyer' }));

    await waitFor(() => {
      const session = useChatStore.getState().sessions[0];
      expect(session.title).toBe('Trace le flux courrier');
      expect(session.messages.map((message) => message.role)).toEqual(['user', 'assistant']);
    });
  });
});

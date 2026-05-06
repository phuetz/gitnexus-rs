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
});

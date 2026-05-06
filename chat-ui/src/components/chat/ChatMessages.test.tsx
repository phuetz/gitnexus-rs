import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useChatStore } from '../../stores/chat-store';
import { ChatMessages } from './ChatMessages';

describe('ChatMessages', () => {
  beforeEach(() => {
    localStorage.clear();
    useChatStore.setState({
      sessions: [
        {
          id: 's1',
          title: 'Flux courrier',
          createdAt: 1774507049000,
          updatedAt: 1774507059000,
          messages: [
            {
              id: 'm1',
              role: 'user',
              content: 'Trace le flux courrier',
              createdAt: 1774507049000,
            },
            {
              id: 'm2',
              role: 'assistant',
              content: 'Réponse longue avec diagramme et sources.',
              createdAt: 1774507059000,
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

  it('lets readers jump back to the latest message after scrolling away', () => {
    render(<ChatMessages />);

    const log = screen.getByRole('log');
    const scrollTo = vi.fn();
    Object.defineProperties(log, {
      scrollHeight: { configurable: true, value: 1000 },
      clientHeight: { configurable: true, value: 300 },
      scrollTop: { configurable: true, writable: true, value: 100 },
      scrollTo: { configurable: true, value: scrollTo },
    });

    fireEvent.scroll(log);
    fireEvent.click(screen.getByRole('button', { name: /aller au dernier message/i }));

    expect(scrollTo).toHaveBeenCalledWith({ top: 1000, behavior: 'smooth' });
  });
});

import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useChatStore } from '../../stores/chat-store';
import { ChatInput } from './ChatInput';

describe('ChatInput', () => {
  beforeEach(() => {
    localStorage.clear();
    useChatStore.setState({
      sessions: [
        {
          id: 's1',
          title: 'Flux courrier',
          createdAt: 1774507049000,
          updatedAt: 1774507079000,
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
              content: 'Réponse 1',
              createdAt: 1774507059000,
            },
            {
              id: 'm3',
              role: 'user',
              content: 'Liste les endpoints HTTP',
              createdAt: 1774507069000,
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

  it('navigates previous prompts with arrow keys', () => {
    render(<ChatInput />);

    const textarea = screen.getByLabelText(/message à envoyer/i) as HTMLTextAreaElement;

    fireEvent.keyDown(textarea, { key: 'ArrowUp' });
    expect(textarea.value).toBe('Liste les endpoints HTTP');

    fireEvent.keyDown(textarea, { key: 'ArrowUp' });
    expect(textarea.value).toBe('Trace le flux courrier');

    fireEvent.keyDown(textarea, { key: 'ArrowDown' });
    expect(textarea.value).toBe('Liste les endpoints HTTP');

    fireEvent.keyDown(textarea, { key: 'ArrowDown' });
    expect(textarea.value).toBe('');
  });

  it('keeps a typed draft unless history browsing has started', () => {
    render(<ChatInput />);

    const textarea = screen.getByLabelText(/message à envoyer/i) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'question en cours' } });
    fireEvent.keyDown(textarea, { key: 'ArrowUp' });

    expect(textarea.value).toBe('question en cours');
  });
});

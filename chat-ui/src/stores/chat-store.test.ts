import { afterEach, describe, expect, it, vi } from 'vitest';
import { migratePersistedChatState, useChatStore } from './chat-store';

afterEach(() => {
  vi.useRealTimers();
});

describe('chat-store persistence', () => {
  it('does not persist transient UI and streaming state', () => {
    const options = useChatStore.persist.getOptions();
    const partialized = options.partialize?.({
      ...useChatStore.getState(),
      isStreaming: true,
      isSfdPanelOpen: true,
      inputDraft: 'Trace un flux',
    });

    expect(partialized).toMatchObject({
      inputDraft: 'Trace un flux',
    });
    expect(partialized).not.toHaveProperty('isStreaming');
    expect(partialized).not.toHaveProperty('isSfdPanelOpen');
  });

  it('migrates older persisted state while clearing volatile fields', () => {
    const migrated = migratePersistedChatState({
      sessions: [],
      currentSessionId: 's1',
      selectedRepo: 'repo_alise',
      selectedRepoName: 'Alise_v2',
      inputDraft: 'Question en cours',
      isStreaming: true,
      isSfdPanelOpen: true,
    });

    expect(migrated).toEqual({
      sessions: [],
      currentSessionId: null,
      selectedRepo: 'repo_alise',
      selectedRepoName: 'Alise_v2',
      inputDraft: 'Question en cours',
    });
    expect(migrated).not.toHaveProperty('isStreaming');
    expect(migrated).not.toHaveProperty('isSfdPanelOpen');
  });

  it('recovers usable chat history from malformed persisted state', () => {
    const migrated = migratePersistedChatState({
      sessions: [
        {
          id: 's1',
          createdAt: '1000',
          updatedAt: '2000',
          messages: [
            {
              id: 'm1',
              role: 'assistant',
              content: 'Réponse conservée',
              createdAt: '3000',
              toolCalls: [
                {
                  id: 'tc1',
                  name: 'query_repo',
                  args: { q: 'courrier' },
                  result: { count: 2 },
                  status: 'done',
                },
                { id: 'tc2', name: 'broken_tool', args: [], status: 'unknown' },
              ],
            },
            { id: 'm2', role: 'assistant', content: 42, createdAt: 4000 },
            { id: 'm3', role: 'alien', content: 'à ignorer', createdAt: 5000 },
          ],
        },
        { id: 42, title: 'Session cassée', messages: [] },
        null,
      ],
      currentSessionId: 'missing',
      selectedRepo: 123,
      selectedRepoName: 'Nom orphelin',
      inputDraft: 123,
    });

    expect(migrated).toEqual({
      sessions: [
        {
          id: 's1',
          title: 'Conversation récupérée',
          createdAt: 1000,
          updatedAt: 2000,
          messages: [
            {
              id: 'm1',
              role: 'assistant',
              content: 'Réponse conservée',
              createdAt: 3000,
              toolCalls: [
                {
                  id: 'tc1',
                  name: 'query_repo',
                  args: { q: 'courrier' },
                  result: { count: 2 },
                  status: 'done',
                },
              ],
            },
          ],
        },
      ],
      currentSessionId: 's1',
      selectedRepo: null,
      selectedRepoName: null,
      inputDraft: '',
    });
  });

  it('orders persisted and updated sessions by recent activity', () => {
    const migrated = migratePersistedChatState({
      sessions: [
        { id: 'old', title: 'Ancienne', createdAt: 1000, updatedAt: 1000, messages: [] },
        { id: 'new', title: 'Récente', createdAt: 1000, updatedAt: 2000, messages: [] },
      ],
      currentSessionId: 'missing',
    });

    expect(migrated.sessions.map((session) => session.id)).toEqual(['new', 'old']);
    expect(migrated.currentSessionId).toBe('new');

    vi.useFakeTimers();
    vi.setSystemTime(5000);
    useChatStore.setState({
      sessions: migrated.sessions,
      currentSessionId: 'old',
      selectedRepo: null,
      selectedRepoName: null,
      inputDraft: '',
      isStreaming: false,
      isSfdPanelOpen: false,
    });

    useChatStore.getState().appendMessage('old', {
      id: 'm1',
      role: 'user',
      content: 'Nouvelle question',
      createdAt: 5000,
    });

    expect(useChatStore.getState().sessions.map((session) => session.id)).toEqual(['old', 'new']);
    expect(useChatStore.getState().sessions[0].updatedAt).toBe(5000);
  });
});

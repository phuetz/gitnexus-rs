import { describe, expect, it } from 'vitest';
import { migratePersistedChatState, useChatStore } from './chat-store';

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
});

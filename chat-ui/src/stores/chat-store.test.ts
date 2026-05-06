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
      currentSessionId: 's1',
      selectedRepo: 'repo_alise',
      selectedRepoName: 'Alise_v2',
      inputDraft: 'Question en cours',
    });
    expect(migrated).not.toHaveProperty('isStreaming');
    expect(migrated).not.toHaveProperty('isSfdPanelOpen');
  });
});

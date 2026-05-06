import { describe, expect, it } from 'vitest';
import { conversationToMarkdown, exportFilename } from './chat-export';
import type { Session } from '../types/chat';

const session: Session = {
  id: 's1',
  title: 'Trace courrier',
  createdAt: 1774507049000,
  updatedAt: 1774507049000,
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
    },
  ],
};

describe('conversationToMarkdown', () => {
  it('includes repo, llm metadata, timestamps, and message contents', () => {
    const markdown = conversationToMarkdown(session, {
      repo: 'Alise_v2',
      llm: {
        configured: true,
        provider: 'chatgpt',
        model: 'gpt-5.5',
        reasoningEffort: 'high',
      },
    });

    expect(markdown).toContain('# Trace courrier');
    expect(markdown).toContain('- Projet: Alise_v2');
    expect(markdown).toContain('- LLM: chatgpt / gpt-5.5, raisonnement high');
    expect(markdown).toContain('## Vous - ');
    expect(markdown).toContain('Trace le flux');
    expect(markdown).toContain('```mermaid');
  });
});

describe('exportFilename', () => {
  it('normalizes repo and session names', () => {
    const filename = exportFilename(session, 'Alisé v2', 'md');
    expect(filename).toMatch(/^gitnexus-alise-v2-trace-courrier-\d{8}-\d{6}\.md$/);
  });
});

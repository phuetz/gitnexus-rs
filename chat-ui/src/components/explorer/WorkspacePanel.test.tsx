import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WorkspacePanel } from './WorkspacePanel';
import { useChatStore } from '../../stores/chat-store';

function jsonResponse(body: unknown, init?: ResponseInit) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
    ...init,
  });
}

describe('WorkspacePanel', () => {
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

    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.includes('/files')) {
          return jsonResponse({
            files: [
              {
                name: 'Controllers',
                path: 'Controllers',
                isDir: true,
                children: [
                  {
                    name: 'CourrierController.cs',
                    path: 'Controllers/CourrierController.cs',
                    isDir: false,
                    children: [],
                  },
                ],
              },
            ],
          });
        }
        if (url.includes('/source')) {
          return jsonResponse({
            path: 'Controllers/CourrierController.cs',
            content: 'public class CourrierController {}',
            language: 'csharp',
            totalLines: 1,
            startLine: 1,
            endLine: 1,
            truncated: false,
          });
        }
        if (url.includes('/symbols')) {
          return jsonResponse({
            symbols: [
              {
                nodeId: 'node-controller',
                name: 'CourrierController',
                label: 'Controller',
                filePath: 'Controllers/CourrierController.cs',
                score: 1,
                startLine: 10,
                endLine: 42,
              },
            ],
          });
        }
        if (url.includes('/graph/neighborhood')) {
          return jsonResponse({
            nodes: [
              {
                id: 'node-controller',
                name: 'CourrierController',
                label: 'Controller',
                filePath: 'Controllers/CourrierController.cs',
                startLine: 10,
                endLine: 42,
                depth: 0,
                isTraced: true,
              },
              {
                id: 'node-service',
                name: 'CourriersService',
                label: 'Service',
                filePath: 'BAL/CourriersService.cs',
                depth: 1,
              },
            ],
            edges: [
              {
                id: 'edge-1',
                source: 'node-controller',
                target: 'node-service',
                relType: 'Calls',
                confidence: 0.9,
              },
            ],
            stats: { nodeCount: 2, edgeCount: 1, truncated: false },
          });
        }
        return jsonResponse({}, { status: 404 });
      })
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('opens a source file and can send that context back to the chat draft', async () => {
    render(<WorkspacePanel onClose={() => {}} />);

    fireEvent.click(await screen.findByText('CourrierController.cs'));

    expect(await screen.findByText('public class CourrierController {}')).toBeTruthy();
    fireEvent.click(screen.getByTitle('Envoyer ce contexte au chat'));

    await waitFor(() => {
      expect(useChatStore.getState().inputDraft).toContain('Controllers/CourrierController.cs');
    });
  });

  it('searches the graph and opens a selected node source', async () => {
    render(<WorkspacePanel onClose={() => {}} />);

    fireEvent.click(screen.getByText('Graphe'));
    fireEvent.change(screen.getByPlaceholderText('Chercher une classe, methode, action...'), {
      target: { value: 'Courrier' },
    });
    fireEvent.click(screen.getByText('Chercher'));

    fireEvent.click(await screen.findByText('CourrierController'));

    expect(await screen.findByText('Voisinage visuel')).toBeTruthy();
    fireEvent.click(screen.getAllByText('Source')[0]);

    expect(await screen.findByText('public class CourrierController {}')).toBeTruthy();
  });
});

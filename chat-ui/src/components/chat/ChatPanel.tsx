import { useState } from 'react';
import { Braces, FileText, MessageSquareText } from 'lucide-react';
import { ChatSidebar } from './ChatSidebar';
import { ChatMessages } from './ChatMessages';
import { ChatInput } from './ChatInput';
import { ProjectSelector } from './ProjectSelector';
import { BackendStatus } from './BackendStatus';
import { SfdDraftsPanel } from './SfdDraftsPanel';
import { ChatExports } from './ChatExports';
import { LlmStatus } from './LlmStatus';
import { SystemDiagnostics } from './SystemDiagnostics';
import { useChatStore } from '../../stores/chat-store';
import { useLlmConfig } from '../../hooks/use-llm-config';
import { formatMessageTimestamp } from '../../utils/dates';
import { WorkspacePanel } from '../explorer/WorkspacePanel';

export function ChatPanel() {
  const session = useChatStore((s) => s.getCurrentSession());
  const isSfdOpen = useChatStore((s) => s.isSfdPanelOpen);
  const setSfdOpen = useChatStore((s) => s.setSfdPanelOpen);
  const [isWorkspaceOpen, setWorkspaceOpen] = useState(false);
  const llm = useLlmConfig();
  const sessionTitle = session?.title.trim() || 'GitNexus Chat';
  const sessionSubtitle = session
    ? `${session.messages.length} message${session.messages.length > 1 ? 's' : ''} - Dernière activité ${formatMessageTimestamp(session.updatedAt) || 'inconnue'}`
    : 'Analyse de code et recherche outillée';

  return (
    <div className="flex h-full w-full bg-neutral-950 text-neutral-100">
      <ChatSidebar />
      <main className="relative flex min-w-0 flex-1 flex-col">
        <header className="flex min-h-14 items-center gap-3 border-b border-neutral-900 bg-neutral-950 px-4 text-sm text-neutral-400">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-neutral-800 bg-neutral-900 text-neutral-300">
              <MessageSquareText className="h-4 w-4" aria-hidden />
            </div>
            <div className="min-w-0">
              <div className="truncate font-medium text-neutral-100">{sessionTitle}</div>
              <div className="truncate text-xs text-neutral-500">{sessionSubtitle}</div>
            </div>
          </div>
          <div className="ml-auto flex items-center gap-2">
            <BackendStatus />
            <LlmStatus llm={llm} />
            <SystemDiagnostics />
            <ChatExports llm={llm} />
            <ProjectSelector />
            <button
              type="button"
              onClick={() => setWorkspaceOpen((open) => !open)}
              className={`flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs hover:bg-neutral-900 ${
                isWorkspaceOpen
                  ? 'border-neutral-700 bg-neutral-900 text-neutral-100'
                  : 'border-neutral-800 bg-neutral-900/60 text-neutral-300'
              }`}
              aria-pressed={isWorkspaceOpen}
              aria-label={isWorkspaceOpen ? "Fermer l'explorateur" : "Ouvrir l'explorateur sources et graphe"}
              title="Sources et graphe"
            >
              <Braces className="h-3.5 w-3.5" aria-hidden />
              <span className="hidden sm:inline">Explorer</span>
            </button>
            <button
              type="button"
              onClick={() => setSfdOpen(!isSfdOpen)}
              className={`flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs hover:bg-neutral-900 ${
                isSfdOpen
                  ? 'border-neutral-700 bg-neutral-900 text-neutral-100'
                  : 'border-neutral-800 bg-neutral-900/60 text-neutral-300'
              }`}
              aria-pressed={isSfdOpen}
              aria-label={isSfdOpen ? 'Fermer le panneau SFD' : 'Ouvrir le panneau SFD'}
            >
              <FileText className="h-3.5 w-3.5" aria-hidden />
              <span className="hidden sm:inline">SFD</span>
            </button>
          </div>
        </header>
        <div className="flex min-h-0 flex-1 bg-neutral-950">
          <div className="min-w-0 flex-1">
            <ChatMessages llm={llm.config} />
          </div>
          {isWorkspaceOpen && <WorkspacePanel onClose={() => setWorkspaceOpen(false)} />}
        </div>
        <ChatInput />
        <SfdDraftsPanel />
      </main>
    </div>
  );
}

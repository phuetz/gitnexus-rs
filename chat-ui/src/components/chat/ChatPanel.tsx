import { FileText } from 'lucide-react';
import { ChatSidebar } from './ChatSidebar';
import { ChatMessages } from './ChatMessages';
import { ChatInput } from './ChatInput';
import { ProjectSelector } from './ProjectSelector';
import { BackendStatus } from './BackendStatus';
import { SfdDraftsPanel } from './SfdDraftsPanel';
import { useChatStore } from '../../stores/chat-store';

export function ChatPanel() {
  const isSfdOpen = useChatStore((s) => s.isSfdPanelOpen);
  const setSfdOpen = useChatStore((s) => s.setSfdPanelOpen);

  return (
    <div className="flex h-full w-full bg-neutral-950">
      <ChatSidebar />
      <main className="relative flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 items-center gap-3 border-b border-neutral-900 bg-neutral-950/60 px-4 text-sm text-neutral-400">
          <div className="flex items-center">
            <span className="font-medium text-neutral-200">GitNexus Chat</span>
            <span className="mx-2 text-neutral-700">/</span>
            <span className="text-xs">V1</span>
          </div>
          <div className="ml-auto flex items-center gap-2">
            <BackendStatus />
            <ProjectSelector />
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
        <div className="min-h-0 flex-1">
          <ChatMessages />
        </div>
        <ChatInput />
        <SfdDraftsPanel />
      </main>
    </div>
  );
}

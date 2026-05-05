import { ChatSidebar } from './ChatSidebar';
import { ChatMessages } from './ChatMessages';
import { ChatInput } from './ChatInput';
import { ProjectSelector } from './ProjectSelector';
import { BackendStatus } from './BackendStatus';

export function ChatPanel() {
  return (
    <div className="flex h-full w-full bg-neutral-950">
      <ChatSidebar />
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 items-center gap-3 border-b border-neutral-900 bg-neutral-950/60 px-4 text-sm text-neutral-400">
          <div className="flex items-center">
            <span className="font-medium text-neutral-200">GitNexus Chat</span>
            <span className="mx-2 text-neutral-700">/</span>
            <span className="text-xs">V1</span>
          </div>
          <div className="ml-auto flex items-center gap-2">
            <BackendStatus />
            <ProjectSelector />
          </div>
        </header>
        <div className="min-h-0 flex-1">
          <ChatMessages />
        </div>
        <ChatInput />
      </main>
    </div>
  );
}

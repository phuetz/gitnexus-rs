import { ChatSidebar } from './ChatSidebar';
import { ChatMessages } from './ChatMessages';
import { ChatInput } from './ChatInput';

export function ChatPanel() {
  return (
    <div className="flex h-full w-full bg-neutral-950">
      <ChatSidebar />
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-12 items-center border-b border-neutral-900 bg-neutral-950/60 px-4 text-sm text-neutral-400">
          <span className="font-medium text-neutral-200">GitNexus Chat</span>
          <span className="mx-2 text-neutral-700">/</span>
          <span>V0 mock</span>
        </header>
        <div className="min-h-0 flex-1">
          <ChatMessages />
        </div>
        <ChatInput />
      </main>
    </div>
  );
}

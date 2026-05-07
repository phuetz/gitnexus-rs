import { useCallback, useEffect, useState } from 'react';
import { mcpClient, type LlmConfigInfo } from '../api/mcp-client';

export type LlmConfigStatus = 'checking' | 'ready' | 'missing' | 'error';

export interface LlmConfigState {
  status: LlmConfigStatus;
  config: LlmConfigInfo | null;
  message: string;
  refresh: () => Promise<void>;
}

export function useLlmConfig(): LlmConfigState {
  const [status, setStatus] = useState<LlmConfigStatus>('checking');
  const [config, setConfig] = useState<LlmConfigInfo | null>(null);
  const [message, setMessage] = useState('Lecture de la configuration LLM...');

  const refresh = useCallback(async () => {
    setStatus('checking');
    setMessage('Lecture de la configuration LLM...');
    try {
      const next = await mcpClient.llmConfig();
      setConfig(next);
      if (next.configured) {
        setStatus('ready');
        setMessage(`${next.provider ?? 'llm'} / ${next.model ?? 'modèle inconnu'}`);
      } else {
        setStatus('missing');
        setMessage('Aucun LLM configuré');
      }
    } catch (e) {
      setStatus('error');
      setConfig(null);
      setMessage(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => void refresh(), 250);
    return () => clearTimeout(timer);
  }, [refresh]);

  return { status, config, message, refresh };
}

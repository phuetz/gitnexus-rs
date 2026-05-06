import { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import SyntaxHighlighter from 'react-syntax-highlighter/dist/esm/prism-light';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import bash from 'react-syntax-highlighter/dist/esm/languages/prism/bash';
import c from 'react-syntax-highlighter/dist/esm/languages/prism/c';
import cpp from 'react-syntax-highlighter/dist/esm/languages/prism/cpp';
import csharp from 'react-syntax-highlighter/dist/esm/languages/prism/csharp';
import css from 'react-syntax-highlighter/dist/esm/languages/prism/css';
import diff from 'react-syntax-highlighter/dist/esm/languages/prism/diff';
import go from 'react-syntax-highlighter/dist/esm/languages/prism/go';
import java from 'react-syntax-highlighter/dist/esm/languages/prism/java';
import javascript from 'react-syntax-highlighter/dist/esm/languages/prism/javascript';
import json from 'react-syntax-highlighter/dist/esm/languages/prism/json';
import jsx from 'react-syntax-highlighter/dist/esm/languages/prism/jsx';
import kotlin from 'react-syntax-highlighter/dist/esm/languages/prism/kotlin';
import markup from 'react-syntax-highlighter/dist/esm/languages/prism/markup';
import powershell from 'react-syntax-highlighter/dist/esm/languages/prism/powershell';
import python from 'react-syntax-highlighter/dist/esm/languages/prism/python';
import ruby from 'react-syntax-highlighter/dist/esm/languages/prism/ruby';
import rust from 'react-syntax-highlighter/dist/esm/languages/prism/rust';
import sql from 'react-syntax-highlighter/dist/esm/languages/prism/sql';
import swift from 'react-syntax-highlighter/dist/esm/languages/prism/swift';
import tsx from 'react-syntax-highlighter/dist/esm/languages/prism/tsx';
import typescript from 'react-syntax-highlighter/dist/esm/languages/prism/typescript';
import yaml from 'react-syntax-highlighter/dist/esm/languages/prism/yaml';

const LANGUAGES = {
  bash,
  c,
  cpp,
  csharp,
  css,
  diff,
  go,
  java,
  javascript,
  json,
  jsx,
  kotlin,
  markup,
  powershell,
  python,
  ruby,
  rust,
  sql,
  swift,
  tsx,
  typescript,
  yaml,
} as const;

const LANGUAGE_ALIASES = new Map<string, keyof typeof LANGUAGES>([
  ['html', 'markup'],
  ['xml', 'markup'],
]);

Object.entries(LANGUAGES).forEach(([name, grammar]) => {
  SyntaxHighlighter.registerLanguage(name, grammar);
});

interface Props {
  language: string;
  code: string;
}

export function SyntaxCodeBlock({ language, code }: Props) {
  const resolvedLanguage = resolveLanguage(language);
  const [copied, setCopied] = useState(false);
  const copyCode = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard access can be denied; the code remains selectable.
    }
  };

  const body = !resolvedLanguage ? (
    <pre className="overflow-x-auto bg-neutral-900 p-3 text-xs">
      <code>{code}</code>
    </pre>
  ) : (
    <SyntaxHighlighter
      language={resolvedLanguage}
      style={vscDarkPlus}
      PreTag="div"
      customStyle={{
        margin: 0,
        borderRadius: 0,
        border: 0,
        fontSize: '0.8125rem',
      }}
    >
      {code}
    </SyntaxHighlighter>
  );

  return (
    <div className="my-2 overflow-hidden rounded-md border border-neutral-800 bg-neutral-950/70">
      <div className="flex items-center justify-between gap-3 border-b border-neutral-800 bg-neutral-900/55 px-3 py-1.5 text-xs">
        <span className="truncate font-mono text-neutral-400">{resolvedLanguage ?? language}</span>
        <button
          type="button"
          onClick={() => void copyCode()}
          className="rounded p-1.5 text-neutral-500 hover:bg-neutral-800 hover:text-neutral-100"
          aria-label="Copier le bloc de code"
          title={copied ? 'Copié !' : 'Copier le code'}
        >
          {copied ? (
            <Check className="h-3.5 w-3.5" aria-hidden="true" />
          ) : (
            <Copy className="h-3.5 w-3.5" aria-hidden="true" />
          )}
        </button>
      </div>
      <div className="overflow-x-auto">{body}</div>
    </div>
  );
}

function resolveLanguage(language: string): keyof typeof LANGUAGES | null {
  const normalized = language.toLowerCase();
  if (normalized in LANGUAGES) return normalized as keyof typeof LANGUAGES;
  return LANGUAGE_ALIASES.get(normalized) ?? null;
}

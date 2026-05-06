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
  if (!resolvedLanguage) {
    return (
      <pre className="my-2 overflow-x-auto rounded-md border border-neutral-800 bg-neutral-900 p-3 text-xs">
        <code>{code}</code>
      </pre>
    );
  }

  return (
    <SyntaxHighlighter
      language={resolvedLanguage}
      style={vscDarkPlus}
      PreTag="div"
      customStyle={{
        margin: '0.5rem 0',
        borderRadius: '0.375rem',
        border: '1px solid rgb(38 38 38)',
        fontSize: '0.8125rem',
      }}
    >
      {code}
    </SyntaxHighlighter>
  );
}

function resolveLanguage(language: string): keyof typeof LANGUAGES | null {
  const normalized = language.toLowerCase();
  if (normalized in LANGUAGES) return normalized as keyof typeof LANGUAGES;
  return LANGUAGE_ALIASES.get(normalized) ?? null;
}

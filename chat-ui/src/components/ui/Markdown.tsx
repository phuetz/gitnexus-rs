import { lazy, Suspense } from 'react';
import ReactMarkdown, { type Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { MermaidBlock } from './MermaidBlock';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';

const SyntaxHighlighter = lazy(() =>
  import('react-syntax-highlighter/dist/esm/prism').then((m) => ({ default: m.Prism }))
);

interface Props {
  children: string;
}

export function Markdown({ children }: Props) {
  return (
    <div className="prose prose-invert prose-sm max-w-none prose-pre:bg-transparent prose-pre:border-0 prose-pre:p-0 prose-code:before:content-[''] prose-code:after:content-['']">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {children}
      </ReactMarkdown>
    </div>
  );
}

// Mermaid graph types we recognise from raw content when the LLM forgot the
// triple-backtick fence (Gemini drops it about 1 reply in 3 even when the
// system prompt requests it). Order matches docs.mermaid.live, longest
// prefixes first so `flowchart` wins over the legacy `graph`.
const MERMAID_GRAPH_TYPES = [
  'flowchart',
  'sequenceDiagram',
  'classDiagram',
  'erDiagram',
  'stateDiagram',
  'gantt',
  'pie',
  'mindmap',
  'gitGraph',
  'journey',
  'graph',
];

function looksLikeMermaid(text: string): boolean {
  const head = text.trimStart().split(/\s|\n/, 1)[0] ?? '';
  return MERMAID_GRAPH_TYPES.includes(head);
}

const components: Components = {
  code(props) {
    const { className, children, ...rest } = props;
    const match = /language-(\w+)/.exec(className ?? '');
    const language = match?.[1];
    const raw = String(children).replace(/\n$/, '');

    // Explicit fence wins. Defensive fallback: if the block has no language
    // tag (LLM dropped it) but the content starts with a known Mermaid graph
    // type keyword, treat it as Mermaid anyway. Avoids the failure mode
    // where the model writes `flowchart TD` directly after `Diagramme :`
    // without a triple-backtick header — react-markdown then renders it as
    // a generic code block and we get plain text instead of an SVG.
    if (language === 'mermaid' || (!language && looksLikeMermaid(raw))) {
      return <MermaidBlock text={raw} />;
    }

    if (!language) {
      return (
        <code className={className} {...rest}>
          {children}
        </code>
      );
    }

    return (
      <Suspense
        fallback={
          <pre className="overflow-x-auto rounded-md border border-neutral-800 bg-neutral-900 p-3 text-xs">
            <code>{raw}</code>
          </pre>
        }
      >
        <SyntaxHighlighter
          language={language}
          style={vscDarkPlus}
          PreTag="div"
          customStyle={{
            margin: '0.5rem 0',
            borderRadius: '0.375rem',
            border: '1px solid rgb(38 38 38)',
            fontSize: '0.8125rem',
          }}
        >
          {raw}
        </SyntaxHighlighter>
      </Suspense>
    );
  },
};

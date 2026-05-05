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

const components: Components = {
  code(props) {
    const { className, children, ...rest } = props;
    const match = /language-(\w+)/.exec(className ?? '');
    const language = match?.[1];
    const raw = String(children).replace(/\n$/, '');

    if (language === 'mermaid') {
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

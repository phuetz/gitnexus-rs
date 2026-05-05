import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface Props {
  children: string;
}

export function Markdown({ children }: Props) {
  return (
    <div className="prose prose-invert prose-sm max-w-none prose-pre:bg-neutral-900 prose-pre:border prose-pre:border-neutral-800 prose-code:before:content-[''] prose-code:after:content-['']">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{children}</ReactMarkdown>
    </div>
  );
}

import { Copy } from "lucide-react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { toast } from "sonner";

function extractTextFromChildren(children: React.ReactNode): string {
  if (typeof children === "string") return children;
  if (typeof children === "number") return String(children);
  if (!children) return "";
  if (Array.isArray(children)) return children.map(extractTextFromChildren).join("");
  if (typeof children === "object" && children !== null && "props" in children) {
    const el = children as React.ReactElement<{ children?: React.ReactNode }>;
    return extractTextFromChildren(el.props.children);
  }
  return "";
}

const markdownComponents: Partial<Components> = {
  pre: ({ children }: { children?: React.ReactNode }) => (
    <div className="relative group my-3">
      <pre
        className="p-4 rounded-lg overflow-x-auto text-[12px] leading-relaxed"
        style={{
          background: "var(--bg-0)",
          border: "1px solid var(--surface-border)",
          fontFamily: "var(--font-mono)",
          borderRadius: 8,
        }}
      >
        {children}
      </pre>
      <button
        className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity px-2 py-1 rounded text-[11px]"
        style={{ background: "var(--bg-3)", color: "var(--text-2)" }}
        onClick={() => {
          const text = extractTextFromChildren(children);
          navigator.clipboard.writeText(text).then(
            () => toast.success("Copied!"),
            () => toast.error("Failed to copy"),
          );
        }}
      >
        <Copy size={12} className="inline mr-1" />
        Copy
      </button>
    </div>
  ),
  code: ({ className, children }: { className?: string; children?: React.ReactNode }) => {
    if (className) {
      return <code className={className}>{children}</code>;
    }
    return (
      <code
        className="px-1 py-0.5 rounded text-[11px]"
        style={{
          background: "var(--bg-3)",
          color: "var(--accent)",
          fontFamily: "var(--font-mono)",
        }}
      >
        {children}
      </code>
    );
  },
  p: ({ children }: { children?: React.ReactNode }) => (
    <p className="mb-2 leading-relaxed">{children}</p>
  ),
  ul: ({ children }: { children?: React.ReactNode }) => (
    <ul className="mb-2 pl-4 space-y-0.5" style={{ listStyleType: "disc" }}>
      {children}
    </ul>
  ),
  ol: ({ children }: { children?: React.ReactNode }) => (
    <ol className="mb-2 pl-4 space-y-0.5" style={{ listStyleType: "decimal" }}>
      {children}
    </ol>
  ),
  strong: ({ children }: { children?: React.ReactNode }) => (
    <strong style={{ color: "var(--text-0)" }}>{children}</strong>
  ),
  a: ({ href, children }: { href?: string; children?: React.ReactNode }) => (
    <a href={href} style={{ color: "var(--accent)", textDecoration: "underline" }}>
      {children}
    </a>
  ),
  h3: ({ children }: { children?: React.ReactNode }) => (
    <h3 className="text-sm font-semibold mt-3 mb-1" style={{ color: "var(--text-0)" }}>
      {children}
    </h3>
  ),
};

export function ChatMarkdown({ content }: { content: string }) {
  return (
    <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
      {content}
    </ReactMarkdown>
  );
}

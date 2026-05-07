export interface SourceReference {
  path: string;
  startLine?: number;
  endLine?: number;
}

const SOURCE_REFERENCE_HASH = '#gitnexus-source';
const SOURCE_REFERENCE_RE =
  /(^|[\s([{"'])((?:[A-Za-z0-9_.-]+[\\/])+[A-Za-z0-9_.-]+\.(?:cshtml|csproj|config|razor|tsx|jsx|toml|swift|html|java|json|yaml|bat|cmd|css|cs|go|hpp|js|kt|md|php|ps1|py|rb|rs|sln|sql|ts|xml|yml|h))(?::(\d+)(?:-(\d+))?)?/g;

export function linkifySourceReferences(markdown: string): string {
  return markdown
    .split(/(```[\s\S]*?```|~~~[\s\S]*?~~~)/g)
    .map((block) => {
      if (block.startsWith('```') || block.startsWith('~~~')) return block;
      return block
        .split(/(`[^`\n]+`)/g)
        .map((inline) => (inline.startsWith('`') ? inline : linkifySourceReferenceSegment(inline)))
        .join('');
    })
    .join('');
}

export function parseSourceReferenceHref(href: string | undefined): SourceReference | null {
  if (!href?.startsWith(SOURCE_REFERENCE_HASH)) return null;
  const queryIndex = href.indexOf('?');
  if (queryIndex === -1) return null;
  const params = new URLSearchParams(href.slice(queryIndex + 1));
  const path = params.get('path');
  if (!path) return null;
  return {
    path,
    startLine: parsePositiveInt(params.get('start')),
    endLine: parsePositiveInt(params.get('end')),
  };
}

function linkifySourceReferenceSegment(text: string): string {
  return text.replace(
    SOURCE_REFERENCE_RE,
    (full, prefix: string, path: string, start?: string, end?: string, offset?: number, input?: string) => {
      if (prefix === '(' && typeof offset === 'number' && input?.[offset - 1] === ']') {
        return full;
      }
      const normalizedPath = path.replace(/\\/g, '/');
      const label = `${normalizedPath}${start ? `:${start}${end ? `-${end}` : ''}` : ''}`;
      const href = buildSourceReferenceHref({
        path: normalizedPath,
        startLine: start ? Number(start) : undefined,
        endLine: end ? Number(end) : start ? Number(start) : undefined,
      });
      return `${prefix}[${label}](${href})`;
    }
  );
}

function buildSourceReferenceHref(reference: SourceReference): string {
  const query = new URLSearchParams({ path: reference.path });
  if (reference.startLine) query.set('start', String(reference.startLine));
  if (reference.endLine) query.set('end', String(reference.endLine));
  return `${SOURCE_REFERENCE_HASH}?${query.toString()}`;
}

function parsePositiveInt(value: string | null): number | undefined {
  if (!value) return undefined;
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
}

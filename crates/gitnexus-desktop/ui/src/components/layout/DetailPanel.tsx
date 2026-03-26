import { useAppStore, type DetailTab } from "../../stores/app-store";
import { useSymbolContext } from "../../hooks/use-tauri-query";
import { CodePanel } from "../code/CodePanel";

const TABS: { id: DetailTab; label: string }[] = [
  { id: "context", label: "Context" },
  { id: "code", label: "Code" },
  { id: "properties", label: "Properties" },
];

export function DetailPanel() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const detailTab = useAppStore((s) => s.detailTab);
  const setDetailTab = useAppStore((s) => s.setDetailTab);

  if (!selectedNodeId) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)] p-4 text-center">
        Click a node in the graph to see its details
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Tab bar */}
      <div className="flex border-b border-[var(--border)] bg-[var(--bg-secondary)]">
        {TABS.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => setDetailTab(id)}
            className={`px-4 py-2 text-xs font-medium transition-colors border-b-2 ${
              detailTab === id
                ? "border-[var(--accent)] text-[var(--accent)]"
                : "border-transparent text-[var(--text-muted)] hover:text-[var(--text-secondary)]"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {detailTab === "context" && <ContextTab />}
        {detailTab === "code" && <CodePanel />}
        {detailTab === "properties" && <PropertiesTab />}
      </div>
    </div>
  );
}

function ContextTab() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const { data: context, isLoading } = useSymbolContext(selectedNodeId);

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)]">
        Loading...
      </div>
    );
  }

  if (!context) return null;

  return (
    <div className="h-full overflow-y-auto p-3 space-y-4">
      {/* Node header */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          <span className="px-2 py-0.5 rounded text-[11px] font-medium bg-[var(--accent)] text-white">
            {context.node.label}
          </span>
          {context.node.isExported && (
            <span className="px-2 py-0.5 rounded text-[11px] font-medium bg-[var(--success)] text-black">
              exported
            </span>
          )}
        </div>
        <h2 className="text-lg font-semibold text-[var(--text-primary)]">
          {context.node.name}
        </h2>
        <p className="text-[var(--text-muted)] text-xs">
          {context.node.filePath}
          {context.node.startLine && `:${context.node.startLine}`}
          {context.node.endLine && `-${context.node.endLine}`}
        </p>
      </div>

      {context.callers.length > 0 && (
        <RelationSection title="Callers" items={context.callers} onSelect={setSelectedNodeId} />
      )}
      {context.callees.length > 0 && (
        <RelationSection title="Callees" items={context.callees} onSelect={setSelectedNodeId} />
      )}
      {context.imports.length > 0 && (
        <RelationSection title="Imports" items={context.imports} onSelect={setSelectedNodeId} />
      )}
      {context.importedBy.length > 0 && (
        <RelationSection title="Imported By" items={context.importedBy} onSelect={setSelectedNodeId} />
      )}
      {context.inherits.length > 0 && (
        <RelationSection title="Inherits" items={context.inherits} onSelect={setSelectedNodeId} />
      )}
      {context.inheritedBy.length > 0 && (
        <RelationSection title="Inherited By" items={context.inheritedBy} onSelect={setSelectedNodeId} />
      )}

      {context.community && (
        <div>
          <h3 className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wider mb-1">
            Community
          </h3>
          <div className="rounded border border-[var(--border)] p-2 bg-[var(--bg-secondary)]">
            <p className="font-medium">{context.community.name}</p>
            {context.community.description && (
              <p className="text-xs text-[var(--text-muted)] mt-1">{context.community.description}</p>
            )}
            <div className="flex gap-3 mt-1 text-xs text-[var(--text-muted)]">
              {context.community.memberCount != null && <span>{context.community.memberCount} members</span>}
              {context.community.cohesion != null && <span>Cohesion: {context.community.cohesion.toFixed(2)}</span>}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function PropertiesTab() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const { data: context } = useSymbolContext(selectedNodeId);

  if (!context) return null;

  const node = context.node;
  const props: [string, string | undefined][] = [
    ["ID", node.id],
    ["Label", node.label],
    ["Name", node.name],
    ["File", node.filePath],
    ["Lines", node.startLine ? `${node.startLine}${node.endLine ? `-${node.endLine}` : ""}` : undefined],
    ["Language", node.language ?? undefined],
    ["Exported", node.isExported != null ? String(node.isExported) : undefined],
    ["Parameters", node.parameterCount != null ? String(node.parameterCount) : undefined],
    ["Return Type", node.returnType ?? undefined],
    ["Community", node.community ?? undefined],
    ["Description", node.description ?? undefined],
  ];

  return (
    <div className="h-full overflow-y-auto p-3">
      <table className="w-full text-xs">
        <tbody>
          {props
            .filter(([, v]) => v !== undefined)
            .map(([key, value]) => (
              <tr key={key} className="border-b border-[var(--border)]">
                <td className="py-1.5 pr-3 text-[var(--text-muted)] font-medium whitespace-nowrap align-top">
                  {key}
                </td>
                <td className="py-1.5 text-[var(--text-primary)] break-all">
                  {value}
                </td>
              </tr>
            ))}
        </tbody>
      </table>
    </div>
  );
}

function RelationSection({
  title,
  items,
  onSelect,
}: {
  title: string;
  items: { id: string; name: string; label: string; filePath: string }[];
  onSelect: (id: string) => void;
}) {
  return (
    <div>
      <h3 className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wider mb-1">
        {title} <span className="text-[var(--text-muted)]">({items.length})</span>
      </h3>
      <ul className="space-y-0.5">
        {items.map((item) => (
          <li
            key={item.id}
            className="flex items-center gap-2 px-2 py-1 rounded cursor-pointer hover:bg-[var(--bg-tertiary)] transition-colors"
            onClick={() => onSelect(item.id)}
          >
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--bg-tertiary)] text-[var(--text-muted)]">
              {item.label}
            </span>
            <span className="truncate">{item.name}</span>
            <span className="ml-auto text-[10px] text-[var(--text-muted)] truncate max-w-[120px]">
              {item.filePath}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

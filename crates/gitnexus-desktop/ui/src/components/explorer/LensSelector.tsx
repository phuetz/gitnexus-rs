import { Eye } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import type { LensType } from "../../stores/app-store";

const LENS_OPTIONS: { value: LensType; label: string; description: string }[] = [
  { value: "all", label: "All", description: "Show all relationships" },
  { value: "calls", label: "Calls", description: "Function/method calls" },
  { value: "structure", label: "Structure", description: "HasMethod, HasProperty, ContainedIn" },
  { value: "heritage", label: "Heritage", description: "Extends, Implements" },
  { value: "impact", label: "Impact", description: "Calls, Imports, DependsOn" },
  { value: "dead-code", label: "Dead Code", description: "Highlight dead code candidates" },
  { value: "tracing", label: "Tracing", description: "Highlight traced methods" },
];

// Maps lens type to visible edge rel_type strings
export const LENS_EDGE_TYPES: Record<LensType, string[] | null> = {
  all: null, // show all
  calls: ["CALLS"],
  structure: ["HAS_METHOD", "HAS_PROPERTY", "CONTAINED_IN", "DEFINED_IN"],
  heritage: ["EXTENDS", "IMPLEMENTS", "INHERITS"],
  impact: ["CALLS", "IMPORTS", "DEPENDS_ON"],
  "dead-code": null, // show all, but highlight dead candidates
  tracing: null, // show all, but highlight traced
};

export function LensSelector() {
  const activeLens = useAppStore((s) => s.activeLens);
  const setActiveLens = useAppStore((s) => s.setActiveLens);

  return (
    <div className="flex items-center gap-1.5">
      <Eye size={14} style={{ color: "var(--text-3)" }} />
      <select
        value={activeLens}
        onChange={(e) => {
          const lens = e.target.value as LensType;
          setActiveLens(lens);
          // Dispatch event for GraphExplorer to pick up
          window.dispatchEvent(
            new CustomEvent("gitnexus:lens-change", {
              detail: { lens, edgeTypes: LENS_EDGE_TYPES[lens] },
            }),
          );
        }}
        className="text-xs rounded px-1.5 py-1 border-none outline-none cursor-pointer"
        style={{
          background: "var(--bg-3)",
          color: "var(--text-1)",
          fontFamily: "var(--font-body)",
        }}
      >
        {LENS_OPTIONS.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

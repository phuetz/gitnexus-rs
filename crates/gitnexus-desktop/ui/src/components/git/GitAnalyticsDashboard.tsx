/**
 * Git Analytics Dashboard — Hotspots, Coupling, and Ownership views.
 */

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Flame, Link2, Users } from "lucide-react";
import { commands } from "../../lib/tauri-commands";
import { AnimatedPage } from "../shared/motion";
import { HotspotsView } from "./HotspotsView";
import { CouplingView } from "./CouplingView";
import { OwnershipView } from "./OwnershipView";

type Tab = "hotspots" | "coupling" | "ownership";

const TABS: { id: Tab; label: string; icon: typeof Flame }[] = [
  { id: "hotspots", label: "Hotspots", icon: Flame },
  { id: "coupling", label: "Coupling", icon: Link2 },
  { id: "ownership", label: "Ownership", icon: Users },
];

export function GitAnalyticsDashboard() {
  const [tab, setTab] = useState<Tab>("hotspots");

  const { data: hotspots, isLoading: loadingH } = useQuery({
    queryKey: ["git-hotspots"],
    queryFn: () => commands.getHotspots(90),
    staleTime: 60_000,
  });

  const { data: coupling, isLoading: loadingC } = useQuery({
    queryKey: ["git-coupling"],
    queryFn: () => commands.getCoupling(3),
    staleTime: 60_000,
  });

  const { data: ownership, isLoading: loadingO } = useQuery({
    queryKey: ["git-ownership"],
    queryFn: () => commands.getOwnership(),
    staleTime: 60_000,
  });

  return (
    <AnimatedPage className="h-full flex flex-col">
      {/* Tab bar */}
      <div
        className="flex items-center gap-1 px-4 py-2 border-b"
        style={{
          backgroundColor: "var(--bg-2)",
          borderColor: "var(--border)",
        }}
      >
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className="flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-medium transition-all"
            style={{
              background: tab === t.id ? "var(--accent-subtle)" : "transparent",
              color: tab === t.id ? "var(--accent)" : "var(--text-2)",
              border: "none",
              cursor: "pointer",
            }}
          >
            <t.icon size={14} />
            {t.label}
            {t.id === "hotspots" && hotspots && (
              <span
                className="px-1.5 rounded-full text-[10px]"
                style={{ background: "var(--bg-3)", color: "var(--text-3)" }}
              >
                {hotspots.length}
              </span>
            )}
            {t.id === "coupling" && coupling && (
              <span
                className="px-1.5 rounded-full text-[10px]"
                style={{ background: "var(--bg-3)", color: "var(--text-3)" }}
              >
                {coupling.length}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {tab === "hotspots" && (
          <HotspotsView data={hotspots || []} loading={loadingH} />
        )}
        {tab === "coupling" && (
          <CouplingView data={coupling || []} loading={loadingC} />
        )}
        {tab === "ownership" && (
          <OwnershipView data={ownership || []} loading={loadingO} />
        )}
      </div>
    </AnimatedPage>
  );
}

import { useQuery } from "@tanstack/react-query";
import { useAppStore } from "../../stores/app-store";
import { commands } from "../../lib/tauri-commands";
import { AnalyzeNav } from "./AnalyzeNav";
import { OverviewView } from "./OverviewView";
import { HotspotsView } from "../git/HotspotsView";
import { CouplingView } from "../git/CouplingView";
import { OwnershipView } from "../git/OwnershipView";
import { CoverageView } from "../coverage/CoverageView";
import { DiagramView } from "../diagram/DiagramView";
import { ReportView } from "../report/ReportView";
import { CodeHealthCard } from "../health/CodeHealthCard";

// Wrapper views that own the data fetching for git analytics
function HotspotsWrapper() {
  const { data, isLoading } = useQuery({
    queryKey: ["git-hotspots"],
    queryFn: () => commands.getHotspots(90),
    staleTime: 60_000,
  });
  return <HotspotsView data={data ?? []} loading={isLoading} />;
}

function CouplingWrapper() {
  const { data, isLoading } = useQuery({
    queryKey: ["git-coupling"],
    queryFn: () => commands.getCoupling(3),
    staleTime: 60_000,
  });
  return <CouplingView data={data ?? []} loading={isLoading} />;
}

function OwnershipWrapper() {
  const { data, isLoading } = useQuery({
    queryKey: ["git-ownership"],
    queryFn: () => commands.getOwnership(),
    staleTime: 60_000,
  });
  return <OwnershipView data={data ?? []} loading={isLoading} />;
}

export function AnalyzeMode() {
  const analyzeView = useAppStore((s) => s.analyzeView);
  const activeRepo = useAppStore((s) => s.activeRepo);

  if (!activeRepo) {
    return (
      <div className="flex items-center justify-center h-full" style={{ color: "var(--text-2)" }}>
        <p style={{ fontFamily: "var(--font-display)", fontSize: 18 }}>Open a repository to view analytics</p>
      </div>
    );
  }

  const renderView = () => {
    switch (analyzeView) {
      case "overview":   return <OverviewView />;
      case "hotspots":   return <HotspotsWrapper />;
      case "coupling":   return <CouplingWrapper />;
      case "ownership":  return <OwnershipWrapper />;
      case "coverage":   return <CoverageView />;
      case "diagram":    return <DiagramView />;
      case "report":     return <ReportView />;
      case "health":
        return (
          <div className="p-6" style={{ maxWidth: 800, margin: "0 auto" }}>
            <h2 style={{ fontFamily: "var(--font-display)", fontSize: 20, fontWeight: 600, color: "var(--text-0)", marginBottom: 16 }}>
              Code Health
            </h2>
            <CodeHealthCard />
          </div>
        );
      default: return <OverviewView />;
    }
  };

  return (
    <div className="flex h-full">
      <AnalyzeNav />
      <div className="flex-1 min-w-0 overflow-auto">
        {renderView()}
      </div>
    </div>
  );
}

import { useAppStore } from "../../stores/app-store";
import { RepoManager } from "../repos/RepoManager";
import { GraphExplorer } from "../graph/GraphExplorer";
import { FileTreeView } from "../files/FileTreeView";
import { ImpactView } from "../impact/ImpactView";

export function MainView() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);

  if (!activeRepo) {
    return <RepoManager />;
  }

  switch (sidebarTab) {
    case "repos":
      return <RepoManager />;
    case "search":
      // Search is now a modal overlay; show graph as default
      return <GraphExplorer />;
    case "files":
      return <FileTreeView />;
    case "impact":
      return <ImpactView />;
    case "docs":
      return (
        <div className="h-full flex items-center justify-center" style={{ color: "var(--text-3)" }}>
          <div className="text-center">
            <p className="text-lg mb-2" style={{ fontFamily: "var(--font-display)", color: "var(--text-2)" }}>
              Documentation
            </p>
            <p className="text-sm">Generate documentation from the Repository card, then view it here.</p>
          </div>
        </div>
      );
    case "graph":
    default:
      return <GraphExplorer />;
  }
}

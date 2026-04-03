import { RepoDashboard } from "../repos/RepoDashboard";

export function OverviewView() {
  return (
    <div className="h-full overflow-auto">
      <RepoDashboard />
    </div>
  );
}

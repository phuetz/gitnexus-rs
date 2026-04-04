import { memo } from "react";
import { RepoDashboard } from "../repos/RepoDashboard";

export const OverviewView = memo(function OverviewView() {
  return (
    <div className="h-full overflow-auto">
      <RepoDashboard />
    </div>
  );
});

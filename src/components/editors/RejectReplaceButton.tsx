// Phase: 7

import { useCallback } from "react";
import { useAppStore } from "../../state/store";

export function RejectReplaceButton({ tabId }: { tabId: string }) {
  const proposedOutput = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.proposedOutput ?? null,
  );
  const status = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.status,
  );
  const rejectProposal = useAppStore((state) => state.rejectProposal);

  const enabled = status === "proposal_ready";

  const handleClick = useCallback(() => {
    if (!enabled) return;
    rejectProposal(tabId);
  }, [enabled, rejectProposal, tabId]);

  if (proposedOutput === null) return null;

  return (
    <button
      className="reject-replace-button"
      onClick={handleClick}
      disabled={!enabled}
      data-testid="reject-replace-button"
    >
      Reject
    </button>
  );
}

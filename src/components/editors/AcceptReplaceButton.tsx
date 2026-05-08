// Phase: 7

import { useCallback } from "react";
import { useAppStore } from "../../state/store";

export function AcceptReplaceButton({ tabId }: { tabId: string }) {
  const proposedOutput = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.proposedOutput ?? null,
  );
  const status = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.status,
  );
  const proposedOutputBaseVersion = useAppStore(
    (state) =>
      state.tabs.find((t) => t.id === tabId)?.proposedOutputBaseVersion ?? null,
  );
  const acceptedOutputVersion = useAppStore(
    (state) =>
      state.tabs.find((t) => t.id === tabId)?.acceptedOutputVersion ?? 0,
  );
  const acceptProposal = useAppStore((state) => state.acceptProposal);

  const versionMatch = proposedOutputBaseVersion === acceptedOutputVersion;
  const enabled = status === "proposal_ready" && versionMatch;

  const handleClick = useCallback(() => {
    if (!enabled) return;
    acceptProposal(tabId);
  }, [enabled, acceptProposal, tabId]);

  if (proposedOutput === null) return null;

  const tooltip = !versionMatch
    ? "Proposal is stale — re-generate or refine again"
    : undefined;

  return (
    <button
      className="accept-replace-button"
      onClick={handleClick}
      disabled={!enabled}
      title={tooltip}
      data-testid="accept-replace-button"
    >
      Accept
    </button>
  );
}

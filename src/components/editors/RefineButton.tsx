// Phase: 7

import { useCallback, useMemo } from "react";
import { useAppStore } from "../../state/store";
import { generationStartRefinement } from "../../ipc/commands";

export function RefineButton({ tabId }: { tabId: string }) {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === tabId),
  );
  const selectedProfileId = useAppStore((state) => {
    const settings = state.metadata.settings;
    if (settings?.lastSelectedProfileId) return settings.lastSelectedProfileId;
    const factoryDefault = state.metadata.profiles.find(
      (p) => p.isFactoryDefault,
    );
    return factoryDefault?.id ?? "factory-default";
  });

  const status = activeTab?.status;
  const acceptedOutput = activeTab?.acceptedOutput ?? null;
  const refinementInstruction = activeTab?.refinementInstruction ?? "";
  const acceptedOutputVersion = activeTab?.acceptedOutputVersion ?? 0;
  const inputVersionToken = activeTab?.inputVersionToken ?? "";

  const activeTagIds = useMemo(
    () => activeTab?.activeTagIds ?? [],
    [activeTab?.activeTagIds],
  );

  // Determine enabled state
  let enabled = false;
  let tooltip: string | undefined;

  if (status === "output_ready" && refinementInstruction.trim().length === 0) {
    enabled = false;
    tooltip = "Enter a refinement instruction";
  } else if (
    status === "refine_editing" &&
    refinementInstruction.trim().length > 0
  ) {
    enabled = true;
  } else if (status === "proposal_ready") {
    enabled = true;
    tooltip = "Submit new refinement";
  }

  const handleClick = useCallback(async () => {
    if (!enabled || acceptedOutput === null) return;

    try {
      await generationStartRefinement({
        contractVersion: 1,
        tabId,
        modelId: "default",
        profileId: selectedProfileId,
        activeTagIds,
        acceptedOutput,
        acceptedOutputVersion,
        refinementInstruction,
        inputVersionToken,
      });
    } catch {
      // Error will be surfaced via generation:failed event
    }
  }, [
    enabled,
    tabId,
    selectedProfileId,
    activeTagIds,
    acceptedOutput,
    acceptedOutputVersion,
    refinementInstruction,
    inputVersionToken,
  ]);

  // Hidden when no accepted output, or proposal_generating
  if (acceptedOutput === null || status === "proposal_generating") return null;

  return (
    <button
      className="refine-button"
      onClick={handleClick}
      disabled={!enabled}
      title={tooltip}
      data-testid="refine-button"
    >
      Refine
    </button>
  );
}

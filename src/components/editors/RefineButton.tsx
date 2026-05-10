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
  const workerState = useAppStore((state) => state.runtime.workerState);
  const loadedModelId = useAppStore((state) => state.runtime.loadedModelId);
  const selectedModelId = useAppStore(
    (state) => state.metadata.settings?.selectedModelId ?? null,
  );
  const loadingPhase = useAppStore((state) => state.modelLoading.phase);
  const isFallbackActive = loadingPhase === "fallback_active";
  const activeGenerationModelId =
    loadedModelId && (loadedModelId === selectedModelId || isFallbackActive)
      ? loadedModelId
      : null;
  const modelReady = workerState === "idle" && activeGenerationModelId !== null;

  const activeTagIds = useMemo(
    () => activeTab?.activeTagIds ?? [],
    [activeTab?.activeTagIds],
  );

  // Determine enabled state
  let enabled = false;
  let tooltip: string | undefined;

  if (!modelReady) {
    enabled = false;
    tooltip = "Model not ready";
  } else if (
    status === "output_ready" &&
    refinementInstruction.trim().length === 0
  ) {
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
    if (!enabled || acceptedOutput === null || !activeGenerationModelId) return;

    try {
      await generationStartRefinement({
        contractVersion: 1,
        tabId,
        modelId: activeGenerationModelId,
        profileId: selectedProfileId,
        activeTagIds,
        acceptedOutput,
        acceptedOutputVersion,
        refinementInstruction,
        inputVersionToken,
      });
    } catch (err) {
      useAppStore.getState().handleGenerationCommandFailed(tabId, err);
    }
  }, [
    enabled,
    tabId,
    activeGenerationModelId,
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

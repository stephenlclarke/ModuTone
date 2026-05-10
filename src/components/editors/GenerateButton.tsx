// Phase: 7
// Generate / Cancel button — context-aware button per UX spec section 2.2.
// Shows Generate, Re-Generate, Retry, Cancel, Select Model, or No Model Available
// depending on tab state and model pipeline readiness.

import { useCallback, useState } from "react";
import { useAppStore } from "../../state/store";
import { generationStartInitial, generationCancel } from "../../ipc/commands";

const EMPTY_TAG_IDS: string[] = [];

interface GenerateButtonProps {
  onOpenSettings: () => void;
}

export function GenerateButton({ onOpenSettings }: GenerateButtonProps) {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );
  const workerState = useAppStore((state) => state.runtime.workerState);
  const loadedModelId = useAppStore((state) => state.runtime.loadedModelId);
  const models = useAppStore((state) => state.metadata.models);
  const selectedModelId = useAppStore(
    (state) => state.metadata.settings?.selectedModelId ?? null,
  );

  if (!activeTab) return null;

  const { status } = activeTab;
  const isGenerating =
    status === "generating" || status === "proposal_generating";

  if (isGenerating) {
    return <CancelButton tabId={activeTab.id} />;
  }

  return (
    <GenerateActionButton
      tabId={activeTab.id}
      tabStatus={status}
      workerState={workerState}
      loadedModelId={loadedModelId}
      models={models}
      selectedModelId={selectedModelId}
      onOpenSettings={onOpenSettings}
    />
  );
}

function CancelButton({ tabId }: { tabId: string }) {
  const activeJob = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.activeJob,
  );
  const [cancelSent, setCancelSent] = useState(false);

  const handleCancel = useCallback(async () => {
    if (!activeJob || cancelSent) return;

    setCancelSent(true);
    try {
      await generationCancel({
        contractVersion: 1,
        jobId: activeJob.jobId,
        tabId,
      });
      // Do NOT unlock UI here. The frontend stays in generating/locked
      // state until the backend emits "generation:canceled" after the
      // worker actually acknowledges the cancellation.
    } catch (err) {
      // Cancel failures are non-critical — the 5s timeout is the safety net
      console.warn("Cancel request failed:", err);
      setCancelSent(false);
    }
  }, [activeJob, tabId, cancelSent]);

  return (
    <button
      className="generate-button generate-button-cancel"
      onClick={handleCancel}
      disabled={cancelSent}
      data-testid="cancel-button"
    >
      {cancelSent ? "Canceling\u2026" : "Cancel"}
    </button>
  );
}

type ButtonAction = "generate" | "open-settings";

function GenerateActionButton({
  tabId,
  tabStatus,
  workerState,
  loadedModelId,
  models,
  selectedModelId,
  onOpenSettings,
}: {
  tabId: string;
  tabStatus: string;
  workerState: string;
  loadedModelId: string | null;
  models: { id: string; isInstalled: boolean }[];
  selectedModelId: string | null;
  onOpenSettings: () => void;
}) {
  const inputText = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.inputText ?? "",
  );
  const inputVersionToken = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.inputVersionToken ?? "",
  );
  const activeTagIds = useAppStore(
    (state) =>
      state.tabs.find((t) => t.id === tabId)?.activeTagIds ?? EMPTY_TAG_IDS,
  );
  const selectedProfileId = useAppStore((state) => {
    const settings = state.metadata.settings;
    if (settings?.lastSelectedProfileId) return settings.lastSelectedProfileId;
    const factoryDefault = state.metadata.profiles.find(
      (p) => p.isFactoryDefault,
    );
    return factoryDefault?.id ?? "factory-default";
  });
  const loadingPhase = useAppStore((state) => state.modelLoading.phase);

  const anyInstalled = models.some((m) => m.isInstalled);
  const isFallbackActive = loadingPhase === "fallback_active";
  const activeGenerationModelId =
    loadedModelId && (loadedModelId === selectedModelId || isFallbackActive)
      ? loadedModelId
      : null;

  // Model readiness checks — only idle means the worker can accept a new job.
  // "busy" means a job is still running (or cancel is in-flight).
  const modelReady = workerState === "idle" && activeGenerationModelId !== null;
  const isWarming = workerState === "warming";
  const isLoadingPhaseActive =
    loadingPhase === "loading" ||
    loadingPhase === "waiting_retry" ||
    loadingPhase === "fallback_loading";
  const isFailed =
    loadingPhase === "failed" && !useAppStore.getState().runtime.loadedModelId;

  // Determine button label, tooltip, enabled state, and action based on model state
  let label: string;
  let tooltip: string | undefined;
  let enabled: boolean;
  let action: ButtonAction = "generate";

  if (!anyInstalled) {
    label = "Generate";
    tooltip = "No model available";
    enabled = false;
  } else if (!selectedModelId) {
    label = "Select Model";
    tooltip = undefined;
    enabled = true;
    action = "open-settings";
  } else if (isFailed) {
    const failClass =
      useAppStore.getState().modelLoading.lastError?.classification;
    label = "No Model Available";
    if (failClass === "model_invalid") {
      tooltip = "Model file incomplete or corrupt";
    } else if (failClass === "insufficient_memory") {
      tooltip = "Not enough memory for this model";
    } else {
      tooltip = "Model failed to load";
    }
    enabled = false;
  } else if (isWarming || isLoadingPhaseActive) {
    label = "Loading Model\u2026";
    tooltip = undefined;
    enabled = false;
  } else if (workerState === "unavailable") {
    label = "Generate";
    tooltip = "Restart required";
    enabled = false;
  } else if (selectedModelId && !modelReady) {
    // Model selected but worker not ready (shouldn't normally happen)
    label = "Generate";
    tooltip = "Model not ready";
    enabled = false;
  } else {
    // Normal flow — determine label from tab status
    // When fallback_active, the model is loaded and generation works
    if (tabStatus === "error") {
      label = "Retry";
    } else if (
      tabStatus === "output_ready" ||
      tabStatus === "refine_editing" ||
      tabStatus === "proposal_ready"
    ) {
      label = "Re-Generate";
    } else {
      label = "Generate";
    }

    // Determine enabled state
    const hasInput = inputText.trim().length > 0;
    enabled =
      tabStatus === "empty"
        ? false
        : tabStatus === "editing"
          ? hasInput && modelReady
          : tabStatus === "error" ||
            tabStatus === "output_ready" ||
            tabStatus === "refine_editing" ||
            tabStatus === "proposal_ready";

    // Determine tooltip
    if (tabStatus === "empty") {
      tooltip = "Enter text to rewrite";
    } else if (tabStatus === "editing" && !modelReady) {
      tooltip = "Model not ready";
    }
  }

  const requestReGenerate = useAppStore((state) => state.requestReGenerate);

  const handleGenerate = useCallback(async () => {
    if (!enabled || !activeGenerationModelId) return;

    // Route Re-Generate through confirmation dialog when output exists
    if (label === "Re-Generate") {
      requestReGenerate(tabId);
      return;
    }

    try {
      await generationStartInitial({
        contractVersion: 1,
        tabId,
        modelId: activeGenerationModelId,
        profileId: selectedProfileId,
        activeTagIds,
        sourceText: inputText,
        inputVersionToken,
      });
    } catch (err) {
      useAppStore.getState().handleGenerationCommandFailed(tabId, err);
    }
  }, [
    enabled,
    label,
    tabId,
    activeGenerationModelId,
    activeTagIds,
    inputText,
    inputVersionToken,
    selectedProfileId,
    requestReGenerate,
  ]);

  const handleClick = useCallback(() => {
    if (action === "open-settings") {
      onOpenSettings();
    } else {
      handleGenerate();
    }
  }, [action, handleGenerate, onOpenSettings]);

  const isSecondaryAction = action === "open-settings";

  return (
    <button
      className={`generate-button${isSecondaryAction ? " generate-button-secondary" : ""}`}
      onClick={handleClick}
      disabled={!enabled}
      title={tooltip}
      data-testid="generate-button"
    >
      {label}
    </button>
  );
}

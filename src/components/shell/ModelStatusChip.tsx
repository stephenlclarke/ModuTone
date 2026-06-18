// Model status chip — interactive toolbar button for current model state

import { useAppStore } from "../../state/store";
import { resolveModelDisplayName } from "../../utils/resolveModelDisplayName";

const EMPTY_ALIASES: Record<string, string> = {};

interface ModelStatusChipProps {
  onOpenSettings: () => void;
}

export function ModelStatusChip({ onOpenSettings }: ModelStatusChipProps) {
  const workerState = useAppStore((state) => state.runtime.workerState);
  const loadedModelId = useAppStore((state) => state.runtime.loadedModelId);
  const selectedModelId = useAppStore(
    (state) => state.metadata.settings?.selectedModelId ?? null,
  );
  const models = useAppStore((state) => state.metadata.models);
  const phase = useAppStore((state) => state.modelLoading.phase);
  const retryCount = useAppStore((state) => state.modelLoading.retryCount);
  const fallbackModelId = useAppStore(
    (state) => state.modelLoading.fallbackModelId,
  );
  const lastErrorClassification = useAppStore(
    (state) => state.modelLoading.lastError?.classification ?? null,
  );

  const aliases = useAppStore(
    (state) => state.metadata.settings?.modelAliases ?? EMPTY_ALIASES,
  );

  const anyInstalled = models.some((m) => m.isInstalled);
  const isReady =
    (workerState === "idle" || workerState === "busy") && loadedModelId != null;
  const isUnavailable =
    workerState === "unavailable" &&
    useAppStore.getState().runtime.appState === "degraded";

  // Resolve display names using aliases
  const resolveDisplayName = (id: string | null): string | null => {
    if (!id) return null;
    return resolveModelDisplayName(id, models, aliases);
  };

  let label: string;
  let dotClass: string | null = null;

  // Model loading phases take priority over basic worker state
  if (phase === "loading" && retryCount === 0) {
    label = "Loading\u2026";
    dotClass = "model-status-dot-warming";
  } else if (
    phase === "waiting_retry" ||
    (phase === "loading" && retryCount > 0)
  ) {
    label = `Retrying (${retryCount}/20)\u2026`;
    dotClass = "model-status-dot-warming";
  } else if (phase === "fallback_loading") {
    label = "Loading fallback\u2026";
    dotClass = "model-status-dot-warming";
  } else if (phase === "fallback_active") {
    const fbName = resolveDisplayName(fallbackModelId);
    label = `${fbName ?? "Fallback"} (fallback)`;
    dotClass = "model-status-dot-warning";
  } else if (phase === "failed") {
    if (lastErrorClassification === "model_invalid") {
      label = "Model Unavailable";
    } else if (lastErrorClassification === "runtime_missing") {
      label = "Runtime Missing";
    } else if (lastErrorClassification === "insufficient_memory") {
      label = "Insufficient Memory";
    } else {
      label = "Load Failed";
    }
    dotClass = "model-status-dot-error";
  } else if (isUnavailable) {
    label = "Offline";
    dotClass = "model-status-dot-error";
  } else if (!anyInstalled) {
    label = "No Model";
  } else if (!selectedModelId) {
    label = "Select Model";
  } else if (isReady && loadedModelId === selectedModelId) {
    const displayName = resolveDisplayName(selectedModelId);
    label = displayName ?? "Ready";
    dotClass = "model-status-dot-ready";
  } else if (workerState === "warming") {
    label = "Loading\u2026";
    dotClass = "model-status-dot-warming";
  } else {
    // Model selected but not loaded (idle, no model in worker)
    const displayName = resolveDisplayName(selectedModelId);
    label = displayName ?? "Not Loaded";
  }

  return (
    <button
      className="model-status-chip"
      data-testid="model-status-chip"
      onClick={onOpenSettings}
      title="Model settings"
    >
      {dotClass && <span className={`model-status-dot ${dotClass}`} />}
      {label}
    </button>
  );
}

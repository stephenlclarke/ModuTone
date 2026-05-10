// Phase: 5
// Status bar with runtime, model, privacy, job, and version segments

import { useState, useEffect, useRef } from "react";
import { useAppStore } from "../../state/store";
import { resolveModelDisplayName } from "../../utils/resolveModelDisplayName";

const EMPTY_ALIASES: Record<string, string> = {};

export function StatusBar() {
  const runtime = useAppStore((state) => state.runtime);
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );
  const models = useAppStore((state) => state.metadata.models);

  const runtimeLabel =
    runtime.workerState === "idle"
      ? "Ready"
      : runtime.workerState === "warming"
        ? "Warming up..."
        : runtime.workerState === "busy"
          ? "Processing..."
          : "Offline";

  const aliases = useAppStore(
    (state) => state.metadata.settings?.modelAliases ?? EMPTY_ALIASES,
  );

  // Resolve display name from loaded model ID
  const modelLabel = runtime.loadedModelId
    ? resolveModelDisplayName(runtime.loadedModelId, models, aliases)
    : "No model";

  const storeLabel = !runtime.metadataStoreWritable
    ? "\u26A0 Read-only mode"
    : null;

  // Check if the active tab has an active generation job
  const isGenerating =
    activeTab?.status === "generating" ||
    activeTab?.status === "proposal_generating";
  const requestKind = activeTab?.activeJob?.requestKind;

  return (
    <div className="status-bar" role="status" data-testid="status-bar">
      <span className="status-segment">{runtimeLabel}</span>
      <span className="status-segment">{modelLabel}</span>
      <span className="status-segment">Local-only</span>
      {isGenerating && (
        <span className="status-segment status-segment-active">
          <ElapsedTimer
            label={
              requestKind === "refinement" ? "Refining..." : "Generating..."
            }
          />
        </span>
      )}
      {storeLabel && (
        <span className="status-segment status-segment-warn">{storeLabel}</span>
      )}
      <span className="status-segment status-segment-version">v1.1.0</span>
    </div>
  );
}

/**
 * Elapsed time counter that ticks every 100ms while mounted.
 * Displays "Label (X.Xs)" format.
 */
function ElapsedTimer({ label }: { label: string }) {
  const [elapsed, setElapsed] = useState(0);
  const startTime = useRef(Date.now());

  useEffect(() => {
    startTime.current = Date.now();
    setElapsed(0);

    const interval = setInterval(() => {
      setElapsed(Date.now() - startTime.current);
    }, 100);

    return () => clearInterval(interval);
  }, []);

  const seconds = (elapsed / 1000).toFixed(1);

  return (
    <span data-testid="elapsed-timer">
      {label} ({seconds}s)
    </span>
  );
}

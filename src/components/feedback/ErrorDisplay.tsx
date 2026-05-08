// Phase: 8
// Per-tab error display with dismiss capability

import { useCallback } from "react";
import { useAppStore } from "../../state/store";

interface ErrorDisplayProps {
  tabId: string;
}

export function ErrorDisplay({ tabId }: ErrorDisplayProps) {
  const error = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.error ?? null,
  );
  const status = useAppStore(
    (state) => state.tabs.find((t) => t.id === tabId)?.status,
  );
  const dismissError = useAppStore((state) => state.dismissError);

  const handleDismiss = useCallback(() => {
    dismissError(tabId);
  }, [dismissError, tabId]);

  if (status !== "error" || error === null) return null;

  return (
    <div className="error-display" data-testid="error-display">
      <div className="error-display-content">
        <p className="error-display-message">{error.message}</p>
        {error.cause && (
          <p className="error-display-cause">Cause: {error.cause}</p>
        )}
        {error.action && <p className="error-display-action">{error.action}</p>}
      </div>
      <button
        className="error-display-dismiss"
        onClick={handleDismiss}
        aria-label="Dismiss error"
      >
        ×
      </button>
    </div>
  );
}

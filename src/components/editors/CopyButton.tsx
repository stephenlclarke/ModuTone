// Phase: 3
// Copy button — copies accepted output to clipboard with toast feedback

import { useCallback } from "react";
import { useAppStore } from "../../state/store";

export function CopyButton() {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );
  const copyAcceptedOutput = useAppStore((state) => state.copyAcceptedOutput);

  const hasOutput =
    activeTab?.acceptedOutput !== null &&
    activeTab?.acceptedOutput !== undefined;

  const handleClick = useCallback(() => {
    if (activeTab) {
      void copyAcceptedOutput(activeTab.id);
    }
  }, [activeTab, copyAcceptedOutput]);

  return (
    <button
      className="editor-btn copy-button"
      onClick={handleClick}
      disabled={!hasOutput}
      title={hasOutput ? "Copy to clipboard" : "No output to copy"}
      data-testid="copy-button"
    >
      Copy
    </button>
  );
}

// Phase: 3
// Clear button — shows confirmation dialog before clearing tab content

import { useCallback } from "react";
import { useAppStore } from "../../state/store";

export function ClearButton() {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );
  const requestClearTab = useAppStore((state) => state.requestClearTab);

  const hasContent =
    activeTab !== undefined &&
    (activeTab.inputText.length > 0 ||
      activeTab.acceptedOutput !== null ||
      activeTab.proposedOutput !== null);

  const handleClick = useCallback(() => {
    if (activeTab) {
      requestClearTab(activeTab.id);
    }
  }, [activeTab, requestClearTab]);

  return (
    <button
      className="editor-btn clear-button"
      onClick={handleClick}
      disabled={!hasContent}
      title={hasContent ? "Clear all content" : "Tab is empty"}
      data-testid="clear-button"
    >
      Clear
    </button>
  );
}

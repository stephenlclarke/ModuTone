// Phase: 8
// Accepted output editor — read-only display of accepted rewrite + error display

import { useAppStore } from "../../state/store";
import { CopyButton } from "./CopyButton";
import { ClearButton } from "./ClearButton";
import { ErrorDisplay } from "../feedback/ErrorDisplay";

export function AcceptedOutputEditor() {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );

  if (!activeTab) return null;

  const hasOutput = activeTab.acceptedOutput !== null;
  const isError = activeTab.status === "error";

  return (
    <div className="accepted-output-editor">
      <div className="editor-header">
        <span className="editor-label">Accepted Output</span>
        <div className="editor-actions">
          <CopyButton />
          <ClearButton />
        </div>
      </div>
      {isError && <ErrorDisplay tabId={activeTab.id} />}
      <textarea
        className="editor-textarea"
        placeholder="Your rewritten text will appear here after generating."
        value={activeTab.acceptedOutput ?? ""}
        readOnly
        tabIndex={hasOutput ? 0 : -1}
        data-testid="accepted-output-editor"
      />
    </div>
  );
}

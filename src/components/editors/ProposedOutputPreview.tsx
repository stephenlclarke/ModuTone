// Phase: 7
// Proposed output preview — read-only, shows during proposal_generating or when proposal exists

import { useCallback, useState } from "react";
import { useAppStore } from "../../state/store";
import { AcceptReplaceButton } from "./AcceptReplaceButton";
import { RejectReplaceButton } from "./RejectReplaceButton";
import { generationCancel } from "../../ipc/commands";

export function ProposedOutputPreview() {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );

  if (!activeTab) return null;

  const isGenerating = activeTab.status === "proposal_generating";
  const hasProposal = activeTab.proposedOutput !== null;

  if (!isGenerating && !hasProposal) return null;

  return (
    <div className="proposed-output-preview">
      <div className="editor-header">
        <span className="editor-label">Proposed Output</span>
        <div className="editor-actions">
          {isGenerating && <CancelRefinementButton tabId={activeTab.id} />}
          {activeTab.status === "proposal_ready" && (
            <>
              <AcceptReplaceButton tabId={activeTab.id} />
              <RejectReplaceButton tabId={activeTab.id} />
            </>
          )}
        </div>
      </div>
      {isGenerating ? (
        <div className="proposal-loading">Generating refinement...</div>
      ) : (
        <textarea
          className="editor-textarea"
          value={activeTab.proposedOutput ?? ""}
          readOnly
          data-testid="proposed-output-preview"
        />
      )}
    </div>
  );
}

function CancelRefinementButton({ tabId }: { tabId: string }) {
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
      // Do NOT unlock UI here. Wait for backend "generation:canceled" event.
    } catch {
      // Cancel failures are non-critical — the 5s timeout is the safety net
      setCancelSent(false);
    }
  }, [activeJob, tabId, cancelSent]);

  return (
    <button
      className="cancel-refinement-button"
      onClick={handleCancel}
      disabled={cancelSent}
      data-testid="cancel-refinement-button"
    >
      {cancelSent ? "Canceling\u2026" : "Cancel"}
    </button>
  );
}

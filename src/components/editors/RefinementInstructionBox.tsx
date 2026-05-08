// Phase: 7
// Refinement instruction box — hidden when no accepted output exists

import { useCallback } from "react";
import { useAppStore } from "../../state/store";
import { RefineButton } from "./RefineButton";

export function RefinementInstructionBox() {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );
  const setRefinementInstruction = useAppStore(
    (state) => state.setRefinementInstruction,
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      if (activeTab) {
        setRefinementInstruction(activeTab.id, e.target.value);
      }
    },
    [activeTab, setRefinementInstruction],
  );

  if (!activeTab || activeTab.acceptedOutput === null) return null;

  const isGenerating = activeTab.status === "proposal_generating";

  return (
    <div className="refinement-instruction-box">
      <div className="editor-header">
        <span className="editor-label">Refinement Instruction</span>
        <div className="editor-actions">
          <RefineButton tabId={activeTab.id} />
        </div>
      </div>
      <textarea
        className="editor-textarea"
        placeholder="Enter a refinement instruction..."
        value={activeTab.refinementInstruction}
        onChange={handleChange}
        readOnly={isGenerating}
        data-testid="refinement-instruction-box"
      />
    </div>
  );
}

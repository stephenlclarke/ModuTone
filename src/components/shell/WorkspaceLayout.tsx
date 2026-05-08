// Phase: 3
// Workspace layout with resizable panes and collapsible tags panel

import { useState, useCallback, useRef, useEffect } from "react";
import { InputEditor } from "../editors/InputEditor";
import { AcceptedOutputEditor } from "../editors/AcceptedOutputEditor";
import { ProposedOutputPreview } from "../editors/ProposedOutputPreview";
import { RefinementInstructionBox } from "../editors/RefinementInstructionBox";
import { TagPanel } from "../tags/TagPanel";

const TAG_PANEL_MIN = 200;
const TAG_PANEL_MAX = 400;
const TAG_PANEL_DEFAULT = 240;
const EDITOR_MIN_HEIGHT = 80;

interface WorkspaceLayoutProps {
  onOpenSettings: () => void;
}

export function WorkspaceLayout({ onOpenSettings }: WorkspaceLayoutProps) {
  // Tags panel state
  const [tagsPanelCollapsed, setTagsPanelCollapsed] = useState(false);
  const [tagsPanelWidth, setTagsPanelWidth] = useState(TAG_PANEL_DEFAULT);

  // Vertical split between input and output
  const [inputHeightFraction, setInputHeightFraction] = useState(0.5);

  const layoutRef = useRef<HTMLDivElement>(null);
  const mainRef = useRef<HTMLDivElement>(null);

  // Horizontal resize (tags panel width)
  const handleHorizontalDragStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      const startX = e.clientX;
      const startWidth = tagsPanelWidth;

      function onMouseMove(moveEvent: MouseEvent) {
        // Tags panel is on the right, so dragging left increases width
        const delta = startX - moveEvent.clientX;
        const newWidth = Math.max(
          TAG_PANEL_MIN,
          Math.min(TAG_PANEL_MAX, startWidth + delta),
        );
        setTagsPanelWidth(newWidth);
      }

      function onMouseUp() {
        document.removeEventListener("mousemove", onMouseMove);
        document.removeEventListener("mouseup", onMouseUp);
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      }

      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [tagsPanelWidth],
  );

  // Vertical resize (input ↔ output split)
  const handleVerticalDragStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      const startY = e.clientY;
      const mainEl = mainRef.current;
      if (!mainEl) return;

      const mainRect = mainEl.getBoundingClientRect();
      const mainHeight = mainRect.height;
      const startFraction = inputHeightFraction;

      function onMouseMove(moveEvent: MouseEvent) {
        const deltaY = moveEvent.clientY - startY;
        const deltaFraction = deltaY / mainHeight;
        let newFraction = startFraction + deltaFraction;

        // Enforce min heights
        const minFraction = EDITOR_MIN_HEIGHT / mainHeight;
        const maxFraction = 1 - minFraction;
        newFraction = Math.max(minFraction, Math.min(maxFraction, newFraction));

        setInputHeightFraction(newFraction);
      }

      function onMouseUp() {
        document.removeEventListener("mousemove", onMouseMove);
        document.removeEventListener("mouseup", onMouseUp);
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      }

      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "row-resize";
      document.body.style.userSelect = "none";
    },
    [inputHeightFraction],
  );

  // Clean up cursor on unmount
  useEffect(() => {
    return () => {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, []);

  const inputPercent = `${String(inputHeightFraction * 100)}%`;
  const outputPercent = `${String((1 - inputHeightFraction) * 100)}%`;

  return (
    <div className="workspace-layout" ref={layoutRef}>
      <div className="workspace-main" ref={mainRef}>
        <div className="workspace-pane-top" style={{ height: inputPercent }}>
          <InputEditor onOpenSettings={onOpenSettings} />
        </div>
        <div
          className="resize-handle resize-handle-vertical"
          onMouseDown={handleVerticalDragStart}
          role="separator"
          aria-orientation="horizontal"
          aria-label="Resize input and output areas"
        />
        <div
          className="workspace-pane-bottom"
          style={{ height: outputPercent }}
        >
          <AcceptedOutputEditor />
          <RefinementInstructionBox />
          <ProposedOutputPreview />
        </div>
      </div>

      {!tagsPanelCollapsed && (
        <>
          <div
            className="resize-handle resize-handle-horizontal"
            onMouseDown={handleHorizontalDragStart}
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize tags panel"
          />
          <div
            className="tag-panel-container"
            style={{ width: tagsPanelWidth }}
          >
            <TagPanel />
          </div>
        </>
      )}

      <button
        className="tag-panel-toggle"
        onClick={() => setTagsPanelCollapsed(!tagsPanelCollapsed)}
        title={tagsPanelCollapsed ? "Show tags panel" : "Hide tags panel"}
        aria-label={tagsPanelCollapsed ? "Show tags panel" : "Hide tags panel"}
        data-testid="tag-panel-toggle"
      >
        {tagsPanelCollapsed ? "\u25B6" : "\u25C0"}
      </button>
    </div>
  );
}

// Phase: 3
// Tab strip — tab creation, switching, close, and inline rename

import { useState, useRef, useEffect, useCallback } from "react";
import { useAppStore } from "../../state/store";

const MAX_TABS = 20;

export function TabStrip() {
  const tabs = useAppStore((state) => state.tabs);
  const activeTabId = useAppStore((state) => state.activeTabId);
  const createTab = useAppStore((state) => state.createTab);
  const switchTab = useAppStore((state) => state.switchTab);
  const requestCloseTab = useAppStore((state) => state.requestCloseTab);
  const renameTab = useAppStore((state) => state.renameTab);

  const [editingTabId, setEditingTabId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const atLimit = tabs.length >= MAX_TABS;

  const startEditing = useCallback((tabId: string, currentTitle: string) => {
    setEditingTabId(tabId);
    setEditValue(currentTitle);
  }, []);

  const commitEdit = useCallback(
    (tabId: string) => {
      renameTab(tabId, editValue);
      setEditingTabId(null);
    },
    [editValue, renameTab],
  );

  const cancelEdit = useCallback(() => {
    setEditingTabId(null);
  }, []);

  // Auto-focus and select all when editing starts
  useEffect(() => {
    if (editingTabId && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingTabId]);

  return (
    <div className="tab-strip" role="tablist">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          className={`tab ${tab.id === activeTabId ? "active" : ""}`}
          role="tab"
          aria-selected={tab.id === activeTabId}
          tabIndex={tab.id === activeTabId ? 0 : -1}
          onClick={() => switchTab(tab.id)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              switchTab(tab.id);
            }
          }}
        >
          {editingTabId === tab.id ? (
            <input
              ref={inputRef}
              className="tab-title-input"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  commitEdit(tab.id);
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  cancelEdit();
                }
                e.stopPropagation();
              }}
              onBlur={() => commitEdit(tab.id)}
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <span
              className="tab-title"
              onDoubleClick={(e) => {
                e.stopPropagation();
                startEditing(tab.id, tab.title);
              }}
            >
              {tab.title}
            </span>
          )}
          <button
            className="tab-close"
            onClick={(e) => {
              e.stopPropagation();
              requestCloseTab(tab.id);
            }}
            aria-label={`Close ${tab.title}`}
            tabIndex={-1}
          >
            ×
          </button>
        </div>
      ))}
      <button
        className="tab-new"
        onClick={createTab}
        disabled={atLimit}
        title={atLimit ? "Maximum tab limit reached" : "New tab"}
        aria-label="New tab"
      >
        +
      </button>
    </div>
  );
}

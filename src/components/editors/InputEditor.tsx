// Phase: 5
// Input editor — memory-only text entry with Generate button

import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../../state/store";
import { GenerateButton } from "./GenerateButton";

interface InputEditorProps {
  onOpenSettings: () => void;
}

export function InputEditor({ onOpenSettings }: InputEditorProps) {
  const activeTab = useAppStore((state) =>
    state.tabs.find((t) => t.id === state.activeTabId),
  );
  const updateInputText = useAppStore((state) => state.updateInputText);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Focus textarea on app launch, new tab, tab switch, or close-tab (activeTab.id changes)
  useEffect(() => {
    textareaRef.current?.focus();
  }, [activeTab?.id]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      if (activeTab) {
        updateInputText(activeTab.id, e.target.value);
      }
    },
    [activeTab, updateInputText],
  );

  if (!activeTab) return null;

  return (
    <div className="input-editor">
      <div className="editor-header">
        <span className="editor-label">Input</span>
        <GenerateButton onOpenSettings={onOpenSettings} />
      </div>
      <textarea
        ref={textareaRef}
        className="editor-textarea"
        placeholder="Paste or type the text you want to rewrite..."
        value={activeTab.inputText}
        onChange={handleChange}
        data-testid="input-editor"
      />
    </div>
  );
}

// Phase: 8

import { useCallback } from "react";
import { useAppStore } from "../../state/store";
import type { VisualStyle } from "../../ipc/types";

interface StyleOption {
  id: VisualStyle;
  name: string;
  colors: [string, string, string];
}

const STYLE_OPTIONS: StyleOption[] = [
  {
    id: "quiet-precision",
    name: "Quiet Precision",
    colors: ["#1a1a2e", "#7b88e0", "#d4d4d4"],
  },
  {
    id: "luminous-professional",
    name: "Luminous Pro",
    colors: ["#0d1b2e", "#00d4aa", "#e0e8f0"],
  },
  {
    id: "editorial-precision",
    name: "Editorial",
    colors: ["#0f0f0f", "#d4a853", "#f0f0f0"],
  },
  {
    id: "glass-slate",
    name: "Glass Slate",
    colors: ["#1a2332", "#a78bfa", "#e4e8f0"],
  },
];

export function VisualStyleSelector() {
  const visualStyle = useAppStore(
    (state) => state.metadata.settings?.visualStyle ?? "quiet-precision",
  );
  const updateSettings = useAppStore((state) => state.updateSettings);
  const addToast = useAppStore((state) => state.addToast);

  const handleSelect = useCallback(
    async (id: VisualStyle) => {
      if (id === visualStyle) return;
      await updateSettings({ visualStyle: id });
      addToast("Settings updated", "success");
    },
    [visualStyle, updateSettings, addToast],
  );

  return (
    <div className="settings-field" data-testid="visual-style-selector">
      <label className="settings-label">Visual Style</label>
      <div className="visual-style-grid">
        {STYLE_OPTIONS.map((opt) => (
          <button
            key={opt.id}
            type="button"
            className={`visual-style-card${opt.id === visualStyle ? " visual-style-card-selected" : ""}`}
            onClick={() => handleSelect(opt.id)}
            aria-pressed={opt.id === visualStyle}
          >
            <div className="visual-style-swatch">
              {opt.colors.map((c, i) => (
                <span
                  key={i}
                  className="visual-style-swatch-dot"
                  style={{ background: c }}
                />
              ))}
            </div>
            <span className="visual-style-card-name">{opt.name}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

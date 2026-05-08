// Phase: 8

import { useEffect, useCallback } from "react";
import { useAppStore } from "../../state/store";
import { useExitAnimation } from "../../utils/useExitAnimation";
import { AppearanceModeSelector } from "./AppearanceModeSelector";
import { VisualStyleSelector } from "./VisualStyleSelector";
import { MotionToggle } from "./MotionToggle";
import { PrivacyBlackoutToggle } from "./PrivacyBlackoutToggle";
import { ModelSelector } from "../models/ModelSelector";

interface SettingsPanelProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsPanel({ open, onClose }: SettingsPanelProps) {
  const metadataStoreWritable = useAppStore(
    (state) => state.runtime.metadataStoreWritable,
  );
  const { mounted, animClass, onAnimationEnd } = useExitAnimation(open);

  // Close on Escape
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [onClose],
  );

  useEffect(() => {
    if (!open) return;
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, handleKeyDown]);

  if (!mounted) return null;

  const isExiting = animClass === "exiting";

  return (
    <div
      className={`settings-overlay ${isExiting ? "settings-overlay-exit" : ""}`}
      onClick={isExiting ? undefined : onClose}
      onAnimationEnd={onAnimationEnd}
    >
      <div
        className={`settings-panel ${isExiting ? "settings-panel-exit" : ""}`}
        data-testid="settings-panel"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="settings-panel-header">
          <h2 className="settings-panel-title">Settings</h2>
          <button
            className="settings-panel-close"
            onClick={isExiting ? undefined : onClose}
            aria-label="Close settings"
          >
            ×
          </button>
        </div>

        {!metadataStoreWritable && (
          <p className="settings-readonly-notice">
            Settings are read-only. Changes won't be saved.
          </p>
        )}

        <div className="settings-section">
          <h3 className="settings-section-title">Appearance</h3>
          <AppearanceModeSelector />
          <VisualStyleSelector />
          <MotionToggle />
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">Model</h3>
          <ModelSelector />
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">Privacy</h3>
          <PrivacyBlackoutToggle />
        </div>
      </div>
    </div>
  );
}

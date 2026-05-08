// Phase: 8

import { useCallback } from "react";
import { useAppStore } from "../../state/store";
import { appSetPrivacyBlackout } from "../../ipc/commands";

export function PrivacyBlackoutToggle() {
  const enabled = useAppStore(
    (state) => state.metadata.settings?.privacyBlackoutEnabled ?? false,
  );
  const supported = useAppStore(
    (state) => state.runtime.privacyBlackoutSupported,
  );
  const updateSettings = useAppStore((state) => state.updateSettings);
  const addToast = useAppStore((state) => state.addToast);

  const handleChange = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const newValue = e.target.checked;
      const response = await appSetPrivacyBlackout({
        contractVersion: 1,
        enabled: newValue,
      });

      if (response.supported) {
        await updateSettings({ privacyBlackoutEnabled: newValue });
        addToast("Settings updated", "success");
      } else {
        addToast(
          "Screen-share protection is not available in this version",
          "error",
          4000,
        );
      }
    },
    [updateSettings, addToast],
  );

  return (
    <div className="settings-field" data-testid="privacy-blackout-toggle">
      <label className="settings-toggle">
        <input
          type="checkbox"
          checked={enabled}
          onChange={handleChange}
          disabled={!supported}
        />
        <span className="settings-toggle-label">Screen-share protection</span>
      </label>
      <p className="settings-sublabel">
        Hides window content from screen capture (best effort)
      </p>
      {!supported && (
        <p className="settings-unsupported-hint">
          Not available in this version
        </p>
      )}
    </div>
  );
}

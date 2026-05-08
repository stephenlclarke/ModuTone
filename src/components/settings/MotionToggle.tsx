// Phase: 8

import { useCallback } from "react";
import { useAppStore } from "../../state/store";

export function MotionToggle() {
  const motionPreference = useAppStore(
    (state) => state.metadata.settings?.motionPreference ?? "standard",
  );
  const updateSettings = useAppStore((state) => state.updateSettings);
  const addToast = useAppStore((state) => state.addToast);

  const handleChange = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.checked ? "reduced" : "standard";
      await updateSettings({ motionPreference: value });
      addToast("Settings updated", "success");
    },
    [updateSettings, addToast],
  );

  return (
    <div className="settings-field" data-testid="motion-toggle">
      <label className="settings-toggle">
        <input
          type="checkbox"
          checked={motionPreference === "reduced"}
          onChange={handleChange}
        />
        <span className="settings-toggle-label">Reduce motion</span>
      </label>
      <p className="settings-sublabel">
        Disables animations and transitions. Always respected when your OS
        requests reduced motion.
      </p>
    </div>
  );
}

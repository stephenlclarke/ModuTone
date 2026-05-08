// Phase: 8

import { useCallback } from "react";
import { useAppStore } from "../../state/store";
import type { ThemePreference } from "../../ipc/types";

export function AppearanceModeSelector() {
  const themePreference = useAppStore(
    (state) => state.metadata.settings?.themePreference ?? "system",
  );
  const updateSettings = useAppStore((state) => state.updateSettings);
  const addToast = useAppStore((state) => state.addToast);

  const handleChange = useCallback(
    async (e: React.ChangeEvent<HTMLSelectElement>) => {
      const value = e.target.value as ThemePreference;
      await updateSettings({ themePreference: value });
      addToast("Settings updated", "success");
    },
    [updateSettings, addToast],
  );

  return (
    <div className="settings-field" data-testid="theme-selector">
      <label className="settings-label" htmlFor="theme-preference">
        Mode
      </label>
      <select
        id="theme-preference"
        className="settings-select"
        value={themePreference}
        onChange={handleChange}
      >
        <option value="system">System</option>
        <option value="light">Light</option>
        <option value="dark">Dark</option>
      </select>
    </div>
  );
}

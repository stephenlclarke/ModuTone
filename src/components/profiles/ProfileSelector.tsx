// Phase: 6
// Profile dropdown selector — global profile selection persisted in settings

import { useState, useCallback } from "react";
import { useAppStore } from "../../state/store";
import { ProfileEditor } from "./ProfileEditor";

export function ProfileSelector() {
  const profiles = useAppStore((s) => s.metadata.profiles);
  const settings = useAppStore((s) => s.metadata.settings);
  const updateSettings = useAppStore((s) => s.updateSettings);

  const [editorOpen, setEditorOpen] = useState(false);

  // Resolve selected profile: use settings value, fallback to factory default by flag
  const selectedId =
    settings?.lastSelectedProfileId ??
    profiles.find((p) => p.isFactoryDefault)?.id ??
    "";

  const handleChange = useCallback(
    async (e: React.ChangeEvent<HTMLSelectElement>) => {
      const value = e.target.value;
      if (value === "__edit__") {
        setEditorOpen(true);
        return;
      }
      await updateSettings({ lastSelectedProfileId: value });
    },
    [updateSettings],
  );

  return (
    <>
      <select
        className="profile-selector"
        value={selectedId}
        onChange={handleChange}
        data-testid="profile-selector"
      >
        {profiles.map((profile) => (
          <option key={profile.id} value={profile.id}>
            {profile.name}
          </option>
        ))}
        <option value="__edit__">Edit Profiles...</option>
      </select>

      {editorOpen && <ProfileEditor onClose={() => setEditorOpen(false)} />}
    </>
  );
}

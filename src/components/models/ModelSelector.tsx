// Phase: 9

import { useCallback, useState, useRef, useEffect } from "react";
import { useAppStore } from "../../state/store";
import { resolveModelDisplayName } from "../../utils/resolveModelDisplayName";

const EMPTY_ALIASES: Record<string, string> = {};

export function ModelSelector() {
  const models = useAppStore((state) => state.metadata.models);
  const selectedModelId = useAppStore(
    (state) => state.metadata.settings?.selectedModelId ?? null,
  );
  const aliases = useAppStore(
    (state) => state.metadata.settings?.modelAliases ?? EMPTY_ALIASES,
  );
  const updateSettings = useAppStore((state) => state.updateSettings);
  const initiateModelLoad = useAppStore((state) => state.initiateModelLoad);
  const setModelAlias = useAppStore((state) => state.setModelAlias);
  const clearModelAlias = useAppStore((state) => state.clearModelAlias);

  const [renaming, setRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);

  const anyInstalled = models.some((m) => m.isInstalled);

  const handleChange = useCallback(
    async (e: React.ChangeEvent<HTMLSelectElement>) => {
      const value = e.target.value || null;
      await updateSettings({ selectedModelId: value });

      if (value) {
        const model = models.find((m) => m.id === value);
        if (model?.isInstalled) {
          initiateModelLoad(value);
        }
      }
    },
    [updateSettings, models, initiateModelLoad],
  );

  const startRename = useCallback(() => {
    if (!selectedModelId) return;
    const currentName = resolveModelDisplayName(
      selectedModelId,
      models,
      aliases,
    );
    setRenameValue(currentName);
    setRenaming(true);
  }, [selectedModelId, models, aliases]);

  const commitRename = useCallback(async () => {
    if (!selectedModelId) return;
    const trimmed = renameValue.trim();
    if (trimmed) {
      // Check if the alias matches the original display name — if so, clear it
      const model = models.find((m) => m.id === selectedModelId);
      if (model && trimmed === model.displayName) {
        await clearModelAlias(selectedModelId);
      } else {
        await setModelAlias(selectedModelId, trimmed);
      }
    } else {
      await clearModelAlias(selectedModelId);
    }
    setRenaming(false);
  }, [selectedModelId, renameValue, models, setModelAlias, clearModelAlias]);

  const cancelRename = useCallback(() => {
    setRenaming(false);
  }, []);

  useEffect(() => {
    if (renaming && renameInputRef.current) {
      renameInputRef.current.focus();
      renameInputRef.current.select();
    }
  }, [renaming]);

  /** Format a model option label: Name (RAM class) */
  function formatModelLabel(model: (typeof models)[number]): string {
    const name = resolveModelDisplayName(model.id, models, aliases);
    return `${name} (${model.ramClassLabel})`;
  }

  if (models.length === 0) {
    return (
      <div className="model-selector" data-testid="model-selector">
        <span className="model-selector-empty">No models available</span>
        <p className="settings-sublabel">
          Place a GGUF model file in the models folder, then restart to begin.
        </p>
      </div>
    );
  }

  if (!anyInstalled) {
    return (
      <div className="model-selector" data-testid="model-selector">
        <select className="settings-select" disabled>
          <option>No models installed</option>
        </select>
        <p className="settings-sublabel">
          Place a GGUF model file in the models folder, then restart to begin.
        </p>
      </div>
    );
  }

  return (
    <div className="model-selector" data-testid="model-selector">
      <label className="settings-label" htmlFor="model-select">
        Model
      </label>
      <select
        id="model-select"
        className="settings-select"
        value={selectedModelId ?? ""}
        onChange={handleChange}
      >
        <option value="">Select a model</option>
        {models
          .filter((m) => m.isInstalled)
          .map((model) => (
            <option key={model.id} value={model.id}>
              {formatModelLabel(model)}
            </option>
          ))}
      </select>
      {selectedModelId && !renaming && (
        <button
          className="model-rename-btn"
          onClick={startRename}
          type="button"
        >
          Rename
        </button>
      )}
      {renaming && (
        <div className="model-rename-row">
          <input
            ref={renameInputRef}
            className="model-rename-input"
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                commitRename();
              } else if (e.key === "Escape") {
                e.preventDefault();
                cancelRename();
              }
            }}
            onBlur={commitRename}
          />
        </div>
      )}
    </div>
  );
}

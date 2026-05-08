// Phase: 6
// Profile editor modal — list, create, edit profiles

import { useState, useEffect, useRef } from "react";
import { useAppStore } from "../../state/store";
import type { ProfileEntry } from "../../ipc/types";

type EditorMode = "list" | "create" | "edit";

interface ProfileEditorProps {
  onClose: () => void;
}

export function ProfileEditor({ onClose }: ProfileEditorProps) {
  const profiles = useAppStore((s) => s.metadata.profiles);
  const createProfile = useAppStore((s) => s.createProfile);
  const updateProfile = useAppStore((s) => s.updateProfile);
  const deleteProfile = useAppStore((s) => s.deleteProfile);
  const addToast = useAppStore((s) => s.addToast);

  const [mode, setMode] = useState<EditorMode>("list");
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [instructionBody, setInstructionBody] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const previousFocusRef = useRef<Element | null>(null);

  // Focus management
  useEffect(() => {
    previousFocusRef.current = document.activeElement;
    return () => {
      if (previousFocusRef.current instanceof HTMLElement) {
        previousFocusRef.current.focus();
      }
    };
  }, []);

  // Escape key
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (mode !== "list") {
          setMode("list");
          setEditingProfileId(null);
          setError(null);
          setConfirmingDelete(false);
        } else {
          onClose();
        }
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [mode, onClose]);

  function startEdit(profile: ProfileEntry) {
    setEditingProfileId(profile.id);
    setName(profile.name);
    setInstructionBody(profile.instructionBody);
    setError(null);
    setConfirmingDelete(false);
    setMode("edit");
  }

  function startCreate() {
    setEditingProfileId(null);
    setName("");
    setInstructionBody("");
    setError(null);
    setMode("create");
  }

  function startDuplicate(profile: ProfileEntry) {
    setEditingProfileId(null);
    setName(`${profile.name} (copy)`);
    setInstructionBody(profile.instructionBody);
    setError(null);
    setMode("create");
  }

  function validate(): string | null {
    const trimmedName = name.trim();
    if (trimmedName.length === 0) return "Name is required.";
    if (trimmedName.length > 100)
      return "Name must be 100 characters or fewer.";
    const trimmedBody = instructionBody.trim();
    if (trimmedBody.length === 0) return "Instruction body is required.";
    if (trimmedBody.length > 10000)
      return "Instruction body must be 10000 characters or fewer.";
    return null;
  }

  async function handleSave() {
    const validationError = validate();
    if (validationError) {
      setError(validationError);
      return;
    }

    setSaving(true);
    setError(null);
    try {
      if (mode === "create") {
        await createProfile(name.trim(), instructionBody.trim());
        addToast("Profile created", "success");
      } else if (mode === "edit" && editingProfileId) {
        await updateProfile(editingProfileId, {
          name: name.trim(),
          instructionBody: instructionBody.trim(),
        });
        addToast("Profile saved", "success");
      }
      setMode("list");
      setEditingProfileId(null);
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : "An unexpected error occurred.";
      setError(message);
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!editingProfileId) return;

    setSaving(true);
    setError(null);
    try {
      await deleteProfile(editingProfileId);
      addToast("Profile deleted", "success");
      setMode("list");
      setEditingProfileId(null);
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : "An unexpected error occurred.";
      setError(message);
    } finally {
      setSaving(false);
      setConfirmingDelete(false);
    }
  }

  const editingProfile = editingProfileId
    ? profiles.find((p) => p.id === editingProfileId)
    : undefined;

  const isFactoryDefault = editingProfile?.isFactoryDefault ?? false;

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog-content profile-editor"
        role="dialog"
        aria-modal="true"
        aria-label="Edit Profiles"
        onClick={(e) => e.stopPropagation()}
      >
        {mode === "list" && (
          <>
            <h2 className="dialog-title">Profiles</h2>
            <button
              className="dialog-btn dialog-btn-confirm profile-editor-new-btn"
              onClick={startCreate}
            >
              + New Profile
            </button>
            <div className="profile-editor-list">
              {profiles.map((profile) => (
                <button
                  key={profile.id}
                  className="profile-editor-item"
                  onClick={() => startEdit(profile)}
                >
                  <span className="profile-editor-item-name">
                    {profile.name}
                  </span>
                  {profile.isFactoryDefault && (
                    <span className="profile-editor-item-badge">default</span>
                  )}
                </button>
              ))}
            </div>
            <div className="dialog-actions">
              <button
                className="dialog-btn dialog-btn-cancel"
                onClick={onClose}
              >
                Close
              </button>
            </div>
          </>
        )}

        {mode === "edit" && isFactoryDefault && (
          <>
            <h2 className="dialog-title">Default Profile</h2>
            <span className="profile-editor-system-badge">System profile</span>
            <p className="profile-editor-summary">
              General-purpose writing assistant for professional workplace
              communication. Rewrites text to be clear, polished, and
              business-appropriate.
            </p>
            <div className="dialog-actions">
              <button
                className="dialog-btn dialog-btn-cancel"
                onClick={() => {
                  setMode("list");
                  setEditingProfileId(null);
                  setError(null);
                }}
              >
                Back
              </button>
            </div>
          </>
        )}

        {(mode === "create" || (mode === "edit" && !isFactoryDefault)) && (
          <>
            <h2 className="dialog-title">
              {mode === "create" ? "New Profile" : (editingProfile?.name ?? "")}
            </h2>

            <div className="profile-editor-field">
              <label className="profile-editor-label" htmlFor="profile-name">
                Name
              </label>
              <input
                id="profile-name"
                className="profile-editor-input"
                type="text"
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                  setError(null);
                }}
                maxLength={100}
                disabled={saving}
              />
            </div>

            <div className="profile-editor-field">
              <label
                className="profile-editor-label"
                htmlFor="profile-instruction"
              >
                Instruction Body
              </label>
              <textarea
                id="profile-instruction"
                className="profile-editor-textarea"
                value={instructionBody}
                onChange={(e) => {
                  setInstructionBody(e.target.value);
                  setError(null);
                }}
                maxLength={10000}
                rows={10}
                disabled={saving}
              />
            </div>

            {error && <p className="profile-editor-error">{error}</p>}

            <div className="dialog-actions">
              {/* Delete — only for custom profiles in edit mode */}
              {mode === "edit" && editingProfile && (
                <>
                  {!confirmingDelete ? (
                    <button
                      className="dialog-btn dialog-btn-destructive"
                      onClick={() => setConfirmingDelete(true)}
                      disabled={saving}
                      style={{ marginRight: "auto" }}
                    >
                      Delete
                    </button>
                  ) : (
                    <>
                      <button
                        className="dialog-btn dialog-btn-destructive"
                        onClick={handleDelete}
                        disabled={saving}
                      >
                        Confirm Delete
                      </button>
                      <button
                        className="dialog-btn dialog-btn-cancel"
                        onClick={() => setConfirmingDelete(false)}
                        disabled={saving}
                      >
                        Keep
                      </button>
                    </>
                  )}
                </>
              )}

              {!confirmingDelete && (
                <>
                  {/* Duplicate for custom profiles */}
                  {mode === "edit" && editingProfile && (
                    <button
                      className="dialog-btn dialog-btn-cancel"
                      onClick={() => startDuplicate(editingProfile)}
                      disabled={saving}
                    >
                      Duplicate
                    </button>
                  )}
                  <button
                    className="dialog-btn dialog-btn-cancel"
                    onClick={() => {
                      setMode("list");
                      setEditingProfileId(null);
                      setError(null);
                    }}
                    disabled={saving}
                  >
                    Cancel
                  </button>
                  <button
                    className="dialog-btn dialog-btn-confirm"
                    onClick={handleSave}
                    disabled={saving}
                  >
                    Save
                  </button>
                </>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

// Phase: 6
// Modal for creating and editing custom tags

import { useState, useEffect, useRef } from "react";
import { useAppStore } from "../../state/store";
import type { TagCategory } from "../../ipc/types";

const TAG_CATEGORIES: { value: TagCategory; label: string }[] = [
  { value: "audience", label: "Audience" },
  { value: "tone", label: "Tone" },
  { value: "format", label: "Format" },
  { value: "clarity", label: "Clarity" },
  { value: "length", label: "Length" },
  { value: "directness", label: "Directness" },
  { value: "technicality", label: "Technicality" },
  { value: "other", label: "Other" },
];

interface CustomTagEditorModalProps {
  tagId: string | null; // null = create mode, string = edit mode
  onClose: () => void;
}

export function CustomTagEditorModal({
  tagId,
  onClose,
}: CustomTagEditorModalProps) {
  const customTags = useAppStore((s) => s.metadata.customTags);
  const createTag = useAppStore((s) => s.createTag);
  const updateTag = useAppStore((s) => s.updateTag);
  const deleteTag = useAppStore((s) => s.deleteTag);
  const removeTagFromAllTabs = useAppStore((s) => s.removeTagFromAllTabs);
  const addToast = useAppStore((s) => s.addToast);

  const existingTag = tagId
    ? customTags.find((t) => t.id === tagId)
    : undefined;
  const isEditMode = tagId !== null;

  const [name, setName] = useState(existingTag?.name ?? "");
  const [category, setCategory] = useState<TagCategory>(
    existingTag?.category ?? "other",
  );
  const [instructionBody, setInstructionBody] = useState(
    existingTag?.instructionBody ?? "",
  );
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const previousFocusRef = useRef<Element | null>(null);
  const nameInputRef = useRef<HTMLInputElement>(null);

  // Focus management
  useEffect(() => {
    previousFocusRef.current = document.activeElement;
    nameInputRef.current?.focus();
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
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  function validate(): string | null {
    const trimmedName = name.trim();
    if (trimmedName.length === 0) return "Name is required.";
    if (trimmedName.length > 50) return "Name must be 50 characters or fewer.";
    const trimmedBody = instructionBody.trim();
    if (trimmedBody.length === 0) return "Instruction body is required.";
    if (trimmedBody.length > 2000)
      return "Instruction body must be 2000 characters or fewer.";
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
      if (isEditMode && tagId) {
        await updateTag(tagId, {
          name: name.trim(),
          category,
          instructionBody: instructionBody.trim(),
        });
        addToast("Tag saved", "success");
      } else {
        await createTag(name.trim(), category, instructionBody.trim());
        addToast("Tag created", "success");
      }
      onClose();
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : "An unexpected error occurred.";
      setError(message);
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!tagId) return;

    setSaving(true);
    setError(null);
    try {
      await deleteTag(tagId);
      removeTagFromAllTabs(tagId);
      addToast("Tag deleted", "success");
      onClose();
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : "An unexpected error occurred.";
      setError(message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog-content custom-tag-editor"
        role="dialog"
        aria-modal="true"
        aria-label={isEditMode ? "Edit Tag" : "Create Tag"}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="dialog-title">
          {isEditMode ? "Edit Tag" : "Create Tag"}
        </h2>

        <div className="custom-tag-editor-field">
          <label className="custom-tag-editor-label" htmlFor="tag-name">
            Name
          </label>
          <input
            ref={nameInputRef}
            id="tag-name"
            className="custom-tag-editor-input"
            type="text"
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              setError(null);
            }}
            maxLength={50}
            disabled={saving}
          />
        </div>

        <div className="custom-tag-editor-field">
          <label className="custom-tag-editor-label" htmlFor="tag-category">
            Category
          </label>
          <select
            id="tag-category"
            className="custom-tag-editor-select"
            value={category}
            onChange={(e) => setCategory(e.target.value as TagCategory)}
            disabled={saving}
          >
            {TAG_CATEGORIES.map((cat) => (
              <option key={cat.value} value={cat.value}>
                {cat.label}
              </option>
            ))}
          </select>
        </div>

        <div className="custom-tag-editor-field">
          <label className="custom-tag-editor-label" htmlFor="tag-instruction">
            Instruction Body
          </label>
          <textarea
            id="tag-instruction"
            className="custom-tag-editor-textarea"
            value={instructionBody}
            onChange={(e) => {
              setInstructionBody(e.target.value);
              setError(null);
            }}
            maxLength={2000}
            rows={5}
            disabled={saving}
          />
        </div>

        {error && <p className="custom-tag-editor-error">{error}</p>}

        <div className="dialog-actions">
          {isEditMode && !confirmingDelete && (
            <button
              className="dialog-btn dialog-btn-destructive"
              onClick={() => setConfirmingDelete(true)}
              disabled={saving}
              style={{ marginRight: "auto" }}
            >
              Delete
            </button>
          )}
          {isEditMode && confirmingDelete && (
            <>
              <span className="custom-tag-editor-confirm-text">
                Delete this tag?
              </span>
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
          {!confirmingDelete && (
            <>
              <button
                className="dialog-btn dialog-btn-cancel"
                onClick={onClose}
                disabled={saving}
              >
                Cancel
              </button>
              <button
                className="dialog-btn dialog-btn-confirm"
                onClick={handleSave}
                disabled={saving}
              >
                {isEditMode ? "Save" : "Create"}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

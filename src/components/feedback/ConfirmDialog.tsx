// Phase: 3
// Generic confirmation dialog with overlay and matched enter/exit animation

import { useEffect, useRef, useCallback, useState } from "react";

interface ConfirmDialogProps {
  title: string;
  body: string;
  extraBody?: string | undefined;
  confirmLabel: string;
  confirmDestructive?: boolean | undefined;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  title,
  body,
  extraBody,
  confirmLabel,
  confirmDestructive = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  const previousFocusRef = useRef<Element | null>(null);
  const [exiting, setExiting] = useState(false);

  // Capture previous focus on mount, focus Cancel button, restore on unmount
  useEffect(() => {
    previousFocusRef.current = document.activeElement;
    cancelRef.current?.focus();
    return () => {
      if (previousFocusRef.current instanceof HTMLElement) {
        previousFocusRef.current.focus();
      }
    };
  }, []);

  const startExit = useCallback(
    (callback: () => void) => {
      if (exiting) return;
      setExiting(true);
      // After exit animation completes, fire the actual callback
      // Duration matches CSS exit animation (150ms)
      setTimeout(callback, 150);
    },
    [exiting],
  );

  const handleCancel = useCallback(
    () => startExit(onCancel),
    [startExit, onCancel],
  );
  const handleConfirm = useCallback(
    () => startExit(onConfirm),
    [startExit, onConfirm],
  );

  // Escape key closes the dialog
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        handleCancel();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleCancel]);

  return (
    <div
      className={`dialog-overlay ${exiting ? "dialog-overlay-exit" : ""}`}
      onClick={handleCancel}
    >
      <div
        className={`dialog-content ${exiting ? "dialog-content-exit" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="dialog-title">{title}</h2>
        <p className="dialog-body">{body}</p>
        {extraBody && (
          <p className="dialog-body dialog-body-extra">{extraBody}</p>
        )}
        <div className="dialog-actions">
          <button
            ref={cancelRef}
            className="dialog-btn dialog-btn-cancel"
            onClick={handleCancel}
          >
            Cancel
          </button>
          <button
            className={`dialog-btn ${confirmDestructive ? "dialog-btn-destructive" : "dialog-btn-confirm"}`}
            onClick={handleConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

// Phase: 3
// Toast container — renders transient notifications with matched enter/exit animation

import { useEffect, useState, useCallback } from "react";
import { useAppStore } from "../../state/store";

export function ToastContainer() {
  const toasts = useAppStore((state) => state.toasts);
  const dismissToast = useAppStore((state) => state.dismissToast);

  return (
    <div className="toast-container">
      {toasts.map((toast) => (
        <ToastItem
          key={toast.id}
          id={toast.id}
          message={toast.message}
          style={toast.style}
          duration={toast.duration}
          onDismiss={dismissToast}
        />
      ))}
    </div>
  );
}

function ToastItem({
  id,
  message,
  style,
  duration,
  onDismiss,
}: {
  id: string;
  message: string;
  style: "success" | "error" | "neutral";
  duration: number;
  onDismiss: (id: string) => void;
}) {
  const [exiting, setExiting] = useState(false);

  const startExit = useCallback(() => {
    if (exiting) return;
    setExiting(true);
    // After exit animation completes (150ms), actually remove
    setTimeout(() => onDismiss(id), 150);
  }, [exiting, onDismiss, id]);

  useEffect(() => {
    const timer = setTimeout(() => {
      startExit();
    }, duration);
    return () => clearTimeout(timer);
  }, [duration, startExit]);

  return (
    <div
      className={`toast toast-${style} ${exiting ? "toast-exit" : ""}`}
      role="status"
      aria-live="polite"
    >
      <span className="toast-message">{message}</span>
      <button
        className="toast-dismiss"
        onClick={startExit}
        aria-label="Dismiss"
      >
        ×
      </button>
    </div>
  );
}

// Phase: 8
// Applies theme, visual style, and motion preferences to document.documentElement

import { useEffect } from "react";
import { useAppStore } from "../state/store";
import type { VisualStyle, MotionPreference } from "../ipc/types";

function resolveTheme(
  preference: "system" | "light" | "dark",
): "light" | "dark" {
  if (preference === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return preference;
}

function toKebab(value: string): string {
  return value.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

function resolveMotion(preference: MotionPreference): "standard" | "reduced" {
  if (preference === "reduced") return "reduced";
  // "standard" setting defers to OS preference
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ? "reduced"
    : "standard";
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const themePreference = useAppStore(
    (state) => state.metadata.settings?.themePreference ?? "system",
  );
  const visualStyle: VisualStyle = useAppStore(
    (state) => state.metadata.settings?.visualStyle ?? "quiet-precision",
  );
  const motionPreference: MotionPreference = useAppStore(
    (state) => state.metadata.settings?.motionPreference ?? "standard",
  );

  // Apply data-theme
  useEffect(() => {
    const apply = () => {
      document.documentElement.setAttribute(
        "data-theme",
        resolveTheme(themePreference),
      );
    };

    apply();

    if (themePreference !== "system") return;

    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => apply();
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, [themePreference]);

  // Apply data-style
  useEffect(() => {
    document.documentElement.setAttribute("data-style", toKebab(visualStyle));
  }, [visualStyle]);

  // Apply data-motion
  useEffect(() => {
    const apply = () => {
      document.documentElement.setAttribute(
        "data-motion",
        resolveMotion(motionPreference),
      );
    };

    apply();

    // Only listen to OS changes when app setting is "standard"
    if (motionPreference !== "standard") return;

    const mql = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handler = () => apply();
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, [motionPreference]);

  return <>{children}</>;
}

// Tests for ThemeProvider — attribute setting for theme, style, motion

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render } from "@testing-library/react";
import { ThemeProvider } from "./ThemeProvider";
import { useAppStore } from "../state/store";
import { act } from "react";

// Mock matchMedia
function createMockMatchMedia(matches: boolean) {
  const listeners: Array<(e: { matches: boolean }) => void> = [];
  return {
    matches,
    addEventListener: (_event: string, fn: (e: { matches: boolean }) => void) =>
      listeners.push(fn),
    removeEventListener: (
      _event: string,
      fn: (e: { matches: boolean }) => void,
    ) => {
      const idx = listeners.indexOf(fn);
      if (idx >= 0) listeners.splice(idx, 1);
    },
    trigger: (newMatches: boolean) => {
      listeners.forEach((fn) => fn({ matches: newMatches }));
    },
    listeners,
  };
}

describe("ThemeProvider", () => {
  let darkMql: ReturnType<typeof createMockMatchMedia>;
  let motionMql: ReturnType<typeof createMockMatchMedia>;

  beforeEach(() => {
    darkMql = createMockMatchMedia(false);
    motionMql = createMockMatchMedia(false);

    vi.spyOn(window, "matchMedia").mockImplementation((query: string) => {
      if (query === "(prefers-color-scheme: dark)") return darkMql as never;
      if (query === "(prefers-reduced-motion: reduce)")
        return motionMql as never;
      return createMockMatchMedia(false) as never;
    });

    // Reset document attributes
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.removeAttribute("data-style");
    document.documentElement.removeAttribute("data-motion");
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function setSettings(patch: Record<string, unknown>) {
    act(() => {
      useAppStore.setState((state) => ({
        metadata: {
          ...state.metadata,
          settings: {
            schemaVersion: 1,
            themePreference: "system" as const,
            trayEnabled: false,
            launchAtLogin: false,
            privacyBlackoutEnabled: false,
            selectedModelId: null,
            lastSelectedProfileId: null,
            lastSuccessfulModelId: null,
            visualStyle: "quiet-precision" as const,
            motionPreference: "standard" as const,
            modelAliases: {},
            ...patch,
          },
        },
      }));
    });
  }

  it("sets data-theme to light when themePreference is light", () => {
    setSettings({ themePreference: "light" });
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  it("sets data-theme to dark when themePreference is dark", () => {
    setSettings({ themePreference: "dark" });
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("sets data-theme from system preference when themePreference is system", () => {
    darkMql = createMockMatchMedia(true);
    vi.spyOn(window, "matchMedia").mockImplementation((query: string) => {
      if (query === "(prefers-color-scheme: dark)") return darkMql as never;
      if (query === "(prefers-reduced-motion: reduce)")
        return motionMql as never;
      return createMockMatchMedia(false) as never;
    });
    setSettings({ themePreference: "system" });
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("sets data-style attribute from visualStyle", () => {
    setSettings({ visualStyle: "luminous-professional" });
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-style")).toBe(
      "luminous-professional",
    );
  });

  it("sets data-style to quiet-precision by default", () => {
    setSettings({});
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-style")).toBe(
      "quiet-precision",
    );
  });

  it("sets data-motion to standard when motionPreference is standard and OS is normal", () => {
    setSettings({ motionPreference: "standard" });
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-motion")).toBe(
      "standard",
    );
  });

  it("sets data-motion to reduced when motionPreference is reduced", () => {
    setSettings({ motionPreference: "reduced" });
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-motion")).toBe(
      "reduced",
    );
  });

  it("sets data-motion to reduced when app is standard but OS prefers reduced motion", () => {
    motionMql = createMockMatchMedia(true);
    vi.spyOn(window, "matchMedia").mockImplementation((query: string) => {
      if (query === "(prefers-color-scheme: dark)") return darkMql as never;
      if (query === "(prefers-reduced-motion: reduce)")
        return motionMql as never;
      return createMockMatchMedia(false) as never;
    });
    setSettings({ motionPreference: "standard" });
    render(
      <ThemeProvider>
        <div />
      </ThemeProvider>,
    );
    expect(document.documentElement.getAttribute("data-motion")).toBe(
      "reduced",
    );
  });
});

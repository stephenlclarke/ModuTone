// Phase: 10
// Concurrency tests for metadata slice (RC-8)

import { describe, it, expect, beforeEach, vi } from "vitest";
import { create } from "zustand";
import { createMetadataSlice, type MetadataSlice } from "./metadataSlice";
import type { SettingsGetResponse } from "../../ipc/types";

// --- Mock IPC commands ---

vi.mock("../../ipc/commands", () => ({
  settingsGet: vi.fn(),
  settingsUpdate: vi.fn(),
  tagsList: vi.fn(),
  tagsCreate: vi.fn(),
  tagsUpdate: vi.fn(),
  tagsDelete: vi.fn(),
  profilesList: vi.fn(),
  profilesCreate: vi.fn(),
  profilesUpdate: vi.fn(),
  profilesDelete: vi.fn(),
  profilesResetToDefault: vi.fn(),
  modelsList: vi.fn(),
}));

import { settingsGet, settingsUpdate } from "../../ipc/commands";

const mockSettingsGet = vi.mocked(settingsGet);
const mockSettingsUpdate = vi.mocked(settingsUpdate);

function makeSettings(
  overrides?: Partial<SettingsGetResponse>,
): SettingsGetResponse {
  return {
    schemaVersion: 1,
    themePreference: "system",
    trayEnabled: false,
    launchAtLogin: false,
    privacyBlackoutEnabled: false,
    selectedModelId: null,
    lastSelectedProfileId: "profile-factory",
    lastSuccessfulModelId: null,
    visualStyle: "quiet-precision",
    motionPreference: "standard",
    modelAliases: {},
    ...overrides,
  };
}

function createTestStore() {
  return create<MetadataSlice>()(createMetadataSlice);
}

describe("metadata concurrency", () => {
  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    vi.clearAllMocks();
    store = createTestStore();
  });

  // RC-8: Concurrent settings writes
  it("RC-8: two rapid updateSettings calls both complete without error", async () => {
    // Simulate two rapid settings updates — both should resolve
    let callCount = 0;

    mockSettingsUpdate.mockImplementation(async () => {
      callCount++;
      return { updated: true };
    });

    // First call returns light, second returns dark (simulating sequential backend writes)
    mockSettingsGet
      .mockResolvedValueOnce(makeSettings({ themePreference: "light" }))
      .mockResolvedValueOnce(makeSettings({ themePreference: "dark" }));

    // Fire both updates concurrently
    const p1 = store.getState().updateSettings({ themePreference: "light" });
    const p2 = store.getState().updateSettings({ themePreference: "dark" });

    await Promise.all([p1, p2]);

    // Both calls should have completed
    expect(callCount).toBe(2);

    // Final state should reflect the last settled value
    // (since settingsGet is called after each update, the last one wins)
    const settings = store.getState().metadata.settings;
    expect(settings).not.toBeNull();
    expect(settings!.themePreference).toBe("dark");
  });

  it("RC-8: concurrent writes don't corrupt state structure", async () => {
    mockSettingsUpdate.mockResolvedValue({ updated: true });
    mockSettingsGet.mockResolvedValue(
      makeSettings({
        themePreference: "dark",
        trayEnabled: true,
        launchAtLogin: true,
      }),
    );

    // Fire multiple concurrent updates
    const promises = [
      store.getState().updateSettings({ themePreference: "dark" }),
      store.getState().updateSettings({ trayEnabled: true }),
      store.getState().updateSettings({ launchAtLogin: true }),
    ];

    await Promise.all(promises);

    // State should be a valid MetadataState, not corrupted
    const { metadata } = store.getState();
    expect(metadata.settings).not.toBeNull();
    expect(metadata.loadStatus).toBeDefined();
    expect(Array.isArray(metadata.profiles)).toBe(true);
    expect(Array.isArray(metadata.builtInTags)).toBe(true);
    expect(Array.isArray(metadata.customTags)).toBe(true);
  });
});

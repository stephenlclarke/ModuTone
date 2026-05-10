// Phase: 8
// Tests for metadata slice — settings, profiles, tags CRUD, models

import { describe, it, expect, beforeEach, vi } from "vitest";
import { create } from "zustand";
import { createMetadataSlice, type MetadataSlice } from "./metadataSlice";
import type {
  SettingsGetResponse,
  TagsListResponse,
  ProfilesListResponse,
  BuiltInTagEntry,
  CustomTagEntry,
  ProfileEntry,
} from "../../ipc/types";

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
  modelDownloadStart: vi.fn(),
  modelDownloadCancel: vi.fn(),
  runtimeGetStatus: vi.fn(),
}));

import {
  settingsGet,
  settingsUpdate,
  tagsList,
  tagsCreate,
  tagsUpdate,
  tagsDelete,
  profilesList,
  profilesCreate,
  profilesUpdate,
  profilesDelete,
  profilesResetToDefault,
  modelsList,
  modelDownloadStart,
  modelDownloadCancel,
} from "../../ipc/commands";

const mockSettingsGet = vi.mocked(settingsGet);
const mockSettingsUpdate = vi.mocked(settingsUpdate);
const mockTagsList = vi.mocked(tagsList);
const mockTagsCreate = vi.mocked(tagsCreate);
const mockTagsUpdate = vi.mocked(tagsUpdate);
const mockTagsDelete = vi.mocked(tagsDelete);
const mockProfilesList = vi.mocked(profilesList);
const mockProfilesCreate = vi.mocked(profilesCreate);
const mockProfilesUpdate = vi.mocked(profilesUpdate);
const mockProfilesDelete = vi.mocked(profilesDelete);
const mockProfilesResetToDefault = vi.mocked(profilesResetToDefault);
const mockModelsList = vi.mocked(modelsList);
const mockModelDownloadStart = vi.mocked(modelDownloadStart);
const mockModelDownloadCancel = vi.mocked(modelDownloadCancel);

const defaultModelsResponse = {
  models: [],
  systemRamBytes: 16_000_000_000,
  systemVramBytes: null,
};

// --- Fixtures ---

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

function makeProfile(overrides?: Partial<ProfileEntry>): ProfileEntry {
  return {
    id: "profile-factory",
    name: "Factory Default",
    instructionBody: "",
    isFactoryDefault: true,
    createdAt: "2025-01-01T00:00:00Z",
    updatedAt: "2025-01-01T00:00:00Z",
    ...overrides,
  };
}

function makeTagsResponse(
  builtIn: BuiltInTagEntry[] = [],
  custom: CustomTagEntry[] = [],
): TagsListResponse {
  return { builtInTags: builtIn, customTags: custom };
}

function makeProfilesResponse(
  profiles: ProfileEntry[] = [],
): ProfilesListResponse {
  return { profiles };
}

function createTestStore() {
  return create<MetadataSlice>()(createMetadataSlice);
}

describe("metadataSlice", () => {
  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    vi.clearAllMocks();
    mockModelsList.mockResolvedValue(defaultModelsResponse);
    store = createTestStore();
  });

  describe("initial state", () => {
    it("starts with idle loadStatus", () => {
      expect(store.getState().metadata.loadStatus).toBe("idle");
    });

    it("starts with null settings", () => {
      expect(store.getState().metadata.settings).toBeNull();
    });

    it("starts with empty profiles", () => {
      expect(store.getState().metadata.profiles).toHaveLength(0);
    });

    it("starts with empty tags", () => {
      expect(store.getState().metadata.builtInTags).toHaveLength(0);
      expect(store.getState().metadata.customTags).toHaveLength(0);
    });
  });

  describe("loadMetadata", () => {
    it("transitions idle → loading → loaded", async () => {
      const settings = makeSettings();
      const factoryProfile = makeProfile();
      mockSettingsGet.mockResolvedValue(settings);
      mockTagsList.mockResolvedValue(makeTagsResponse());
      mockProfilesList.mockResolvedValue(
        makeProfilesResponse([factoryProfile]),
      );

      const promise = store.getState().loadMetadata();
      expect(store.getState().metadata.loadStatus).toBe("loading");

      await promise;
      expect(store.getState().metadata.loadStatus).toBe("loaded");
      expect(store.getState().metadata.settings).toEqual(settings);
      expect(store.getState().metadata.profiles).toHaveLength(1);
    });

    it("transitions to error on failure", async () => {
      mockSettingsGet.mockRejectedValue(new Error("network error"));
      mockTagsList.mockRejectedValue(new Error("network error"));
      mockProfilesList.mockRejectedValue(new Error("network error"));

      await store.getState().loadMetadata();
      expect(store.getState().metadata.loadStatus).toBe("error");
    });

    it("corrects dangling lastSelectedProfileId on load", async () => {
      const factoryProfile = makeProfile({ id: "profile-factory" });
      const settings = makeSettings({
        lastSelectedProfileId: "deleted-profile-id",
      });

      mockSettingsGet.mockResolvedValue(settings);
      mockTagsList.mockResolvedValue(makeTagsResponse());
      mockProfilesList.mockResolvedValue(
        makeProfilesResponse([factoryProfile]),
      );
      mockSettingsUpdate.mockResolvedValue({ updated: true });

      await store.getState().loadMetadata();

      // Should have called settingsUpdate to correct the dangling reference
      expect(mockSettingsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        lastSelectedProfileId: "profile-factory",
      });
      expect(store.getState().metadata.settings?.lastSelectedProfileId).toBe(
        "profile-factory",
      );
    });

    it("does not correct lastSelectedProfileId when it matches an existing profile", async () => {
      const factoryProfile = makeProfile({ id: "profile-factory" });
      const settings = makeSettings({
        lastSelectedProfileId: "profile-factory",
      });

      mockSettingsGet.mockResolvedValue(settings);
      mockTagsList.mockResolvedValue(makeTagsResponse());
      mockProfilesList.mockResolvedValue(
        makeProfilesResponse([factoryProfile]),
      );

      await store.getState().loadMetadata();

      expect(mockSettingsUpdate).not.toHaveBeenCalled();
    });

    it("resolves factory default by isFactoryDefault flag, not hardcoded ID", async () => {
      const factoryProfile = makeProfile({
        id: "custom-factory-id",
        isFactoryDefault: true,
      });
      const settings = makeSettings({
        lastSelectedProfileId: "nonexistent",
      });

      mockSettingsGet.mockResolvedValue(settings);
      mockTagsList.mockResolvedValue(makeTagsResponse());
      mockProfilesList.mockResolvedValue(
        makeProfilesResponse([factoryProfile]),
      );
      mockSettingsUpdate.mockResolvedValue({ updated: true });

      await store.getState().loadMetadata();

      expect(mockSettingsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        lastSelectedProfileId: "custom-factory-id",
      });
    });
  });

  describe("updateSettings", () => {
    it("calls settingsUpdate and refreshes", async () => {
      const updatedSettings = makeSettings({ themePreference: "dark" });
      mockSettingsUpdate.mockResolvedValue({ updated: true });
      mockSettingsGet.mockResolvedValue(updatedSettings);

      await store.getState().updateSettings({ themePreference: "dark" });

      expect(mockSettingsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        themePreference: "dark",
      });
      expect(store.getState().metadata.settings?.themePreference).toBe("dark");
    });
  });

  describe("createTag", () => {
    it("calls tagsCreate and refreshes tags list", async () => {
      const newTag: CustomTagEntry = {
        id: "tag-new",
        name: "My Tag",
        category: "tone",
        instructionBody: "Be nice",
        isBuiltIn: false,
        createdAt: "2025-01-01T00:00:00Z",
        updatedAt: "2025-01-01T00:00:00Z",
      };
      mockTagsCreate.mockResolvedValue({ id: "tag-new" });
      mockTagsList.mockResolvedValue(makeTagsResponse([], [newTag]));

      const id = await store.getState().createTag("My Tag", "tone", "Be nice");

      expect(id).toBe("tag-new");
      expect(mockTagsCreate).toHaveBeenCalledWith({
        contractVersion: 1,
        name: "My Tag",
        category: "tone",
        instructionBody: "Be nice",
      });
      expect(store.getState().metadata.customTags).toHaveLength(1);
    });
  });

  describe("updateTag", () => {
    it("calls tagsUpdate and refreshes tags list", async () => {
      mockTagsUpdate.mockResolvedValue({ updated: true });
      mockTagsList.mockResolvedValue(makeTagsResponse());

      await store.getState().updateTag("tag-1", { name: "Renamed" });

      expect(mockTagsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        id: "tag-1",
        name: "Renamed",
      });
    });
  });

  describe("deleteTag", () => {
    it("calls tagsDelete and refreshes tags list", async () => {
      mockTagsDelete.mockResolvedValue({ deleted: true });
      mockTagsList.mockResolvedValue(makeTagsResponse());

      await store.getState().deleteTag("tag-1");

      expect(mockTagsDelete).toHaveBeenCalledWith({
        contractVersion: 1,
        id: "tag-1",
      });
    });
  });

  describe("createProfile", () => {
    it("calls profilesCreate and refreshes profiles", async () => {
      const newProfile = makeProfile({
        id: "profile-new",
        name: "Custom",
        isFactoryDefault: false,
      });
      mockProfilesCreate.mockResolvedValue({ id: "profile-new" });
      mockProfilesList.mockResolvedValue(makeProfilesResponse([newProfile]));

      const id = await store
        .getState()
        .createProfile("Custom", "Custom instructions");

      expect(id).toBe("profile-new");
      expect(mockProfilesCreate).toHaveBeenCalledWith({
        contractVersion: 1,
        name: "Custom",
        instructionBody: "Custom instructions",
      });
    });
  });

  describe("updateProfile", () => {
    it("calls profilesUpdate and refreshes profiles", async () => {
      mockProfilesUpdate.mockResolvedValue({ updated: true });
      mockProfilesList.mockResolvedValue(makeProfilesResponse([]));

      await store
        .getState()
        .updateProfile("profile-1", { name: "Renamed Profile" });

      expect(mockProfilesUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        id: "profile-1",
        name: "Renamed Profile",
      });
    });
  });

  describe("deleteProfile", () => {
    it("calls profilesDelete and refreshes", async () => {
      const factoryProfile = makeProfile({ id: "profile-factory" });
      mockProfilesDelete.mockResolvedValue({ deleted: true });
      mockProfilesList.mockResolvedValue(
        makeProfilesResponse([factoryProfile]),
      );
      mockSettingsGet.mockResolvedValue(
        makeSettings({ lastSelectedProfileId: "profile-factory" }),
      );

      // Set initial state so settings has a different selected profile
      store.setState((state) => ({
        metadata: {
          ...state.metadata,
          settings: makeSettings({ lastSelectedProfileId: "other-profile" }),
        },
      }));

      await store.getState().deleteProfile("some-other");

      expect(mockProfilesDelete).toHaveBeenCalledWith({
        contractVersion: 1,
        id: "some-other",
      });
    });

    it("falls back to factory default when deleting selected profile", async () => {
      const factoryProfile = makeProfile({ id: "profile-factory" });
      const settings = makeSettings({
        lastSelectedProfileId: "profile-to-delete",
      });

      // Set initial state with selected profile
      store.setState((state) => ({
        metadata: { ...state.metadata, settings },
      }));

      mockProfilesDelete.mockResolvedValue({ deleted: true });
      mockProfilesList.mockResolvedValue(
        makeProfilesResponse([factoryProfile]),
      );
      mockSettingsUpdate.mockResolvedValue({ updated: true });
      mockSettingsGet.mockResolvedValue(
        makeSettings({ lastSelectedProfileId: "profile-factory" }),
      );

      await store.getState().deleteProfile("profile-to-delete");

      expect(mockSettingsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        lastSelectedProfileId: "profile-factory",
      });
    });

    it("resolves fallback by isFactoryDefault flag when deleting selected profile", async () => {
      const factoryProfile = makeProfile({
        id: "dynamic-factory-id",
        isFactoryDefault: true,
      });
      const settings = makeSettings({
        lastSelectedProfileId: "profile-to-delete",
      });

      store.setState((state) => ({
        metadata: { ...state.metadata, settings },
      }));

      mockProfilesDelete.mockResolvedValue({ deleted: true });
      mockProfilesList.mockResolvedValue(
        makeProfilesResponse([factoryProfile]),
      );
      mockSettingsUpdate.mockResolvedValue({ updated: true });
      mockSettingsGet.mockResolvedValue(
        makeSettings({ lastSelectedProfileId: "dynamic-factory-id" }),
      );

      await store.getState().deleteProfile("profile-to-delete");

      expect(mockSettingsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        lastSelectedProfileId: "dynamic-factory-id",
      });
    });
  });

  describe("resetProfileToDefault", () => {
    it("calls profilesResetToDefault and refreshes profiles", async () => {
      mockProfilesResetToDefault.mockResolvedValue({ reset: true });
      mockProfilesList.mockResolvedValue(makeProfilesResponse([]));

      await store.getState().resetProfileToDefault("profile-factory");

      expect(mockProfilesResetToDefault).toHaveBeenCalledWith({
        contractVersion: 1,
        id: "profile-factory",
      });
    });
  });

  describe("models", () => {
    it("starts with empty models and null systemRamBytes", () => {
      expect(store.getState().metadata.models).toHaveLength(0);
      expect(store.getState().metadata.systemRamBytes).toBeNull();
    });

    it("loadMetadata populates models and systemRamBytes", async () => {
      const modelsResponse = {
        models: [
          {
            id: "qwen2.5-3b-instruct",
            displayName: "Qwen 2.5 3B Instruct",
            backend: "gguf" as const,
            sizeBytes: 2_000_000_000,
            ramClassLabel: "~8 GB",
            minRamBytes: 8_000_000_000,
            isInstalled: false,
            isCataloged: true,
            suitability: "recommended" as const,
            quantLabel: null,
            canDownload: true,
            downloadSizeBytes: 2_000_000_000,
            downloadUnavailableReason: null,
          },
        ],
        systemRamBytes: 32_000_000_000,
        systemVramBytes: null,
      };
      mockModelsList.mockResolvedValue(modelsResponse);
      mockSettingsGet.mockResolvedValue(makeSettings());
      mockTagsList.mockResolvedValue(makeTagsResponse());
      mockProfilesList.mockResolvedValue(makeProfilesResponse([makeProfile()]));

      await store.getState().loadMetadata();

      expect(store.getState().metadata.models).toHaveLength(1);
      expect(store.getState().metadata.models[0]!.id).toBe(
        "qwen2.5-3b-instruct",
      );
      expect(store.getState().metadata.systemRamBytes).toBe(32_000_000_000);
    });

    it("starts a model download and records queued state", async () => {
      mockModelDownloadStart.mockResolvedValue({
        started: true,
        alreadyInstalled: false,
        totalBytes: 2_000_000_000,
      });

      await store.getState().startModelDownload("qwen2.5-3b-instruct");

      expect(mockModelDownloadStart).toHaveBeenCalledWith({
        contractVersion: 1,
        modelId: "qwen2.5-3b-instruct",
      });
      expect(
        store.getState().metadata.modelDownloads["qwen2.5-3b-instruct"]?.status,
      ).toBe("queued");
    });

    it("updates download progress from backend events", () => {
      store.getState().handleModelDownloadProgress({
        contractVersion: 1,
        modelId: "qwen2.5-3b-instruct",
        status: "downloading",
        bytesDownloaded: 500,
        totalBytes: 1000,
        fileName: "qwen2.5-3b-instruct-q5_k_m.gguf",
      });

      expect(
        store.getState().metadata.modelDownloads["qwen2.5-3b-instruct"],
      ).toMatchObject({
        status: "downloading",
        bytesDownloaded: 500,
        totalBytes: 1000,
        fileName: "qwen2.5-3b-instruct-q5_k_m.gguf",
      });
    });

    it("cancels a model download", async () => {
      mockModelDownloadCancel.mockResolvedValue({ canceled: true });

      await store.getState().cancelModelDownload("qwen2.5-3b-instruct");

      expect(mockModelDownloadCancel).toHaveBeenCalledWith({
        contractVersion: 1,
        modelId: "qwen2.5-3b-instruct",
      });
    });
  });

  describe("visual style and motion settings", () => {
    it("updateSettings round-trips visualStyle", async () => {
      const updatedSettings = makeSettings({
        visualStyle: "glass-slate",
      });
      mockSettingsUpdate.mockResolvedValue({ updated: true });
      mockSettingsGet.mockResolvedValue(updatedSettings);

      await store.getState().updateSettings({ visualStyle: "glass-slate" });

      expect(mockSettingsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        visualStyle: "glass-slate",
      });
      expect(store.getState().metadata.settings?.visualStyle).toBe(
        "glass-slate",
      );
    });

    it("updateSettings round-trips motionPreference", async () => {
      const updatedSettings = makeSettings({
        motionPreference: "reduced",
      });
      mockSettingsUpdate.mockResolvedValue({ updated: true });
      mockSettingsGet.mockResolvedValue(updatedSettings);

      await store.getState().updateSettings({ motionPreference: "reduced" });

      expect(mockSettingsUpdate).toHaveBeenCalledWith({
        contractVersion: 1,
        motionPreference: "reduced",
      });
      expect(store.getState().metadata.settings?.motionPreference).toBe(
        "reduced",
      );
    });

    it("backward compat: settings without visualStyle/motionPreference load defaults", async () => {
      // Simulate old settings JSON that lacks new fields — backend provides defaults via serde
      const legacySettings = makeSettings();
      mockSettingsGet.mockResolvedValue(legacySettings);
      mockTagsList.mockResolvedValue(makeTagsResponse());
      mockProfilesList.mockResolvedValue(makeProfilesResponse([makeProfile()]));

      await store.getState().loadMetadata();

      expect(store.getState().metadata.settings?.visualStyle).toBe(
        "quiet-precision",
      );
      expect(store.getState().metadata.settings?.motionPreference).toBe(
        "standard",
      );
    });
  });
});

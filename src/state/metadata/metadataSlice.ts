// Phase: 8
// Metadata slice — settings, profiles, tags, models (cached from backend)

import type { StateCreator } from "zustand";
import type { MetadataState, MlxRuntimeState } from "./types";
import type {
  MotionPreference,
  TagCategory,
  ThemePreference,
  VisualStyle,
} from "../../ipc/types";
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
  mlxRuntimeStatus,
  mlxRuntimeInstallStart,
  modelAliasSet,
  modelAliasClear,
} from "../../ipc/commands";
import type {
  ModelDownloadProgressEvent,
  MlxRuntimeInstallProgressEvent,
  MlxRuntimeStatusResponse,
  ProfileEntry,
} from "../../ipc/types";

/** Resolve the factory default profile ID from the profiles list by its flag. */
function getFactoryDefaultId(profiles: ProfileEntry[]): string {
  const factoryDefault = profiles.find((p) => p.isFactoryDefault);
  return factoryDefault ? factoryDefault.id : "factory-default";
}

function emptyMlxRuntimeState(): MlxRuntimeState {
  return {
    supported: false,
    installed: false,
    installing: false,
    installDir: null,
    pythonPath: null,
    unavailableReason: null,
    status: "idle" as const,
    step: null,
    detail: null,
    error: null,
  };
}

function mergeMlxRuntimeState(
  previous: MlxRuntimeState | null,
  response: MlxRuntimeStatusResponse,
): MlxRuntimeState {
  return {
    ...(previous ?? emptyMlxRuntimeState()),
    supported: response.supported,
    installed: response.installed,
    installing: response.installing,
    installDir: response.installDir,
    pythonPath: response.pythonPath,
    unavailableReason: response.unavailableReason,
    status: response.installed
      ? ("completed" as const)
      : (previous?.status ?? "idle"),
    error: response.installed ? null : (previous?.error ?? null),
  };
}

export interface MetadataSlice {
  metadata: MetadataState;

  // Load all metadata from backend
  loadMetadata: () => Promise<void>;

  // Settings
  updateSettings: (
    patch: Partial<{
      themePreference: ThemePreference;
      trayEnabled: boolean;
      launchAtLogin: boolean;
      privacyBlackoutEnabled: boolean;
      selectedModelId: string | null;
      lastSelectedProfileId: string | null;
      lastSuccessfulModelId: string | null;
      visualStyle: VisualStyle;
      motionPreference: MotionPreference;
    }>,
  ) => Promise<void>;

  // Profile CRUD
  createProfile: (name: string, instructionBody: string) => Promise<string>;
  updateProfile: (
    id: string,
    patch: { name?: string; instructionBody?: string },
  ) => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  resetProfileToDefault: (id: string) => Promise<void>;

  // Model Aliases
  setModelAlias: (modelId: string, alias: string) => Promise<void>;
  clearModelAlias: (modelId: string) => Promise<void>;

  // Model Downloads
  startModelDownload: (modelId: string) => Promise<void>;
  cancelModelDownload: (modelId: string) => Promise<void>;
  handleModelDownloadProgress: (event: ModelDownloadProgressEvent) => void;

  // Apple Silicon MLX runtime setup
  refreshMlxRuntime: () => Promise<void>;
  installMlxRuntime: () => Promise<void>;
  handleMlxRuntimeProgress: (event: MlxRuntimeInstallProgressEvent) => void;

  // Tag CRUD
  createTag: (
    name: string,
    category: TagCategory,
    instructionBody: string,
  ) => Promise<string>;
  updateTag: (
    id: string,
    patch: { name?: string; category?: TagCategory; instructionBody?: string },
  ) => Promise<void>;
  deleteTag: (id: string) => Promise<void>;
}

export const createMetadataSlice: StateCreator<MetadataSlice> = (set, get) => ({
  metadata: {
    settings: null,
    profiles: [],
    builtInTags: [],
    customTags: [],
    models: [],
    modelDownloads: {},
    mlxRuntime: null,
    systemRamBytes: null,
    loadStatus: "idle",
  },

  loadMetadata: async () => {
    set((state) => ({
      metadata: { ...state.metadata, loadStatus: "loading" },
    }));
    try {
      const [
        settings,
        tagsResponse,
        profilesResponse,
        modelsResponse,
        mlxRuntimeResponse,
      ] = await Promise.all([
        settingsGet(),
        tagsList(),
        profilesList(),
        modelsList(),
        mlxRuntimeStatus(),
      ]);
      set({
        metadata: {
          settings,
          builtInTags: tagsResponse.builtInTags,
          customTags: tagsResponse.customTags,
          profiles: profilesResponse.profiles,
          models: modelsResponse.models,
          modelDownloads: get().metadata.modelDownloads,
          mlxRuntime: mergeMlxRuntimeState(
            get().metadata.mlxRuntime,
            mlxRuntimeResponse,
          ),
          systemRamBytes: modelsResponse.systemRamBytes,
          loadStatus: "loaded",
        },
      });

      // Validate: if lastSelectedProfileId points to a non-existent profile, correct it
      const { metadata } = get();
      const fallbackId = getFactoryDefaultId(metadata.profiles);
      if (
        metadata.settings &&
        metadata.settings.lastSelectedProfileId &&
        !metadata.profiles.some(
          (p) => p.id === metadata.settings!.lastSelectedProfileId,
        )
      ) {
        await settingsUpdate({
          contractVersion: 1,
          lastSelectedProfileId: fallbackId,
        });
        set((state) => ({
          metadata: {
            ...state.metadata,
            settings: state.metadata.settings
              ? {
                  ...state.metadata.settings,
                  lastSelectedProfileId: fallbackId,
                }
              : null,
          },
        }));
      }
    } catch {
      set((state) => ({
        metadata: { ...state.metadata, loadStatus: "error" },
      }));
    }
  },

  updateSettings: async (patch) => {
    await settingsUpdate({ contractVersion: 1, ...patch });
    const settings = await settingsGet();
    set((state) => ({
      metadata: { ...state.metadata, settings },
    }));
  },

  // --- Profile CRUD ---

  createProfile: async (name, instructionBody) => {
    const response = await profilesCreate({
      contractVersion: 1,
      name,
      instructionBody,
    });
    const profilesResponse = await profilesList();
    set((state) => ({
      metadata: { ...state.metadata, profiles: profilesResponse.profiles },
    }));
    return response.id;
  },

  updateProfile: async (id, patch) => {
    await profilesUpdate({ contractVersion: 1, id, ...patch });
    const profilesResponse = await profilesList();
    set((state) => ({
      metadata: { ...state.metadata, profiles: profilesResponse.profiles },
    }));
  },

  deleteProfile: async (id) => {
    await profilesDelete({ contractVersion: 1, id });

    // Refresh profiles first so we can resolve the fallback by flag
    const profilesResponse = await profilesList();
    const fallbackId = getFactoryDefaultId(profilesResponse.profiles);

    // If the deleted profile was the selected profile, fall back to factory default
    const currentSettings = get().metadata.settings;
    if (currentSettings?.lastSelectedProfileId === id) {
      await settingsUpdate({
        contractVersion: 1,
        lastSelectedProfileId: fallbackId,
      });
    }

    // Refresh settings to reflect any fallback change
    const settings = await settingsGet();
    set((state) => ({
      metadata: {
        ...state.metadata,
        profiles: profilesResponse.profiles,
        settings,
      },
    }));
  },

  resetProfileToDefault: async (id) => {
    await profilesResetToDefault({ contractVersion: 1, id });
    const profilesResponse = await profilesList();
    set((state) => ({
      metadata: { ...state.metadata, profiles: profilesResponse.profiles },
    }));
  },

  // --- Model Aliases ---

  setModelAlias: async (modelId, alias) => {
    await modelAliasSet({ contractVersion: 1, modelId, alias });
    const settings = await settingsGet();
    set((state) => ({
      metadata: { ...state.metadata, settings },
    }));
  },

  clearModelAlias: async (modelId) => {
    await modelAliasClear({ contractVersion: 1, modelId });
    const settings = await settingsGet();
    set((state) => ({
      metadata: { ...state.metadata, settings },
    }));
  },

  // --- Model Downloads ---

  startModelDownload: async (modelId) => {
    set((state) => ({
      metadata: {
        ...state.metadata,
        modelDownloads: {
          ...state.metadata.modelDownloads,
          [modelId]: {
            status: "queued",
            bytesDownloaded: 0,
            totalBytes:
              state.metadata.models.find((model) => model.id === modelId)
                ?.downloadSizeBytes ?? 0,
            fileName: null,
            error: null,
          },
        },
      },
    }));

    try {
      const response = await modelDownloadStart({
        contractVersion: 1,
        modelId,
      });
      if (response.alreadyInstalled) {
        await get().loadMetadata();
      }
    } catch (error) {
      const detail =
        error && typeof error === "object" && "message" in error
          ? String((error as { message?: unknown }).message)
          : "Failed to start model download";
      set((state) => ({
        metadata: {
          ...state.metadata,
          modelDownloads: {
            ...state.metadata.modelDownloads,
            [modelId]: {
              status: "failed",
              bytesDownloaded: 0,
              totalBytes:
                state.metadata.models.find((model) => model.id === modelId)
                  ?.downloadSizeBytes ?? 0,
              fileName: null,
              error: detail,
            },
          },
        },
      }));
    }
  },

  cancelModelDownload: async (modelId) => {
    await modelDownloadCancel({ contractVersion: 1, modelId });
  },

  handleModelDownloadProgress: (event) => {
    set((state) => ({
      metadata: {
        ...state.metadata,
        modelDownloads: {
          ...state.metadata.modelDownloads,
          [event.modelId]: {
            status: event.status,
            bytesDownloaded: event.bytesDownloaded,
            totalBytes: event.totalBytes,
            fileName: event.fileName ?? null,
            error: event.error ?? null,
          },
        },
      },
    }));

    if (event.status === "completed") {
      void get().loadMetadata();
    }
  },

  // --- Apple Silicon MLX Runtime ---

  refreshMlxRuntime: async () => {
    const response = await mlxRuntimeStatus();
    set((state) => ({
      metadata: {
        ...state.metadata,
        mlxRuntime: mergeMlxRuntimeState(state.metadata.mlxRuntime, response),
      },
    }));
  },

  installMlxRuntime: async () => {
    set((state) => ({
      metadata: {
        ...state.metadata,
        mlxRuntime: {
          ...(state.metadata.mlxRuntime ?? emptyMlxRuntimeState()),
          installing: true,
          status: "queued",
          step: "queued",
          detail: "Preparing MLX runtime setup",
          error: null,
        },
      },
    }));

    try {
      const response = await mlxRuntimeInstallStart({ contractVersion: 1 });
      if (response.alreadyInstalled) {
        await get().refreshMlxRuntime();
      }
    } catch (error) {
      const detail =
        error && typeof error === "object" && "message" in error
          ? String((error as { message?: unknown }).message)
          : "Failed to start MLX runtime setup";
      set((state) => ({
        metadata: {
          ...state.metadata,
          mlxRuntime: {
            ...(state.metadata.mlxRuntime ?? emptyMlxRuntimeState()),
            installing: false,
            status: "failed",
            step: "failed",
            detail: null,
            error: detail,
          },
        },
      }));
    }
  },

  handleMlxRuntimeProgress: (event) => {
    set((state) => ({
      metadata: {
        ...state.metadata,
        mlxRuntime: {
          ...(state.metadata.mlxRuntime ?? emptyMlxRuntimeState()),
          installed: event.status === "completed",
          installing:
            event.status === "queued" || event.status === "installing",
          status: event.status,
          step: event.step,
          detail: event.detail ?? null,
          error: event.error ?? null,
        },
      },
    }));

    if (event.status === "completed") {
      void get().refreshMlxRuntime();
    }
  },

  // --- Tag CRUD ---

  createTag: async (name, category, instructionBody) => {
    const response = await tagsCreate({
      contractVersion: 1,
      name,
      category,
      instructionBody,
    });
    const tagsResponse = await tagsList();
    set((state) => ({
      metadata: {
        ...state.metadata,
        builtInTags: tagsResponse.builtInTags,
        customTags: tagsResponse.customTags,
      },
    }));
    return response.id;
  },

  updateTag: async (id, patch) => {
    await tagsUpdate({ contractVersion: 1, id, ...patch });
    const tagsResponse = await tagsList();
    set((state) => ({
      metadata: {
        ...state.metadata,
        builtInTags: tagsResponse.builtInTags,
        customTags: tagsResponse.customTags,
      },
    }));
  },

  deleteTag: async (id) => {
    await tagsDelete({ contractVersion: 1, id });
    const tagsResponse = await tagsList();
    set((state) => ({
      metadata: {
        ...state.metadata,
        builtInTags: tagsResponse.builtInTags,
        customTags: tagsResponse.customTags,
      },
    }));
  },
});

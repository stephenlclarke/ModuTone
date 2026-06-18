// Phase: 8
// Metadata state types — cached from backend

import type {
  BuiltInTagEntry,
  CustomTagEntry,
  ModelEntry,
  ModelDownloadProgressEvent,
  MlxRuntimeInstallProgressEvent,
  MotionPreference,
  ProfileEntry,
  ThemePreference,
  VisualStyle,
} from "../../ipc/types";

export interface ModelDownloadState {
  status: ModelDownloadProgressEvent["status"] | "idle";
  bytesDownloaded: number;
  totalBytes: number;
  fileName: string | null;
  error: string | null;
}

export interface MlxRuntimeState {
  supported: boolean;
  installed: boolean;
  installing: boolean;
  installDir: string | null;
  pythonPath: string | null;
  unavailableReason: string | null;
  status: MlxRuntimeInstallProgressEvent["status"] | "idle";
  step: string | null;
  detail: string | null;
  error: string | null;
}

export interface MetadataState {
  settings: {
    schemaVersion: number;
    themePreference: ThemePreference;
    trayEnabled: boolean;
    launchAtLogin: boolean;
    privacyBlackoutEnabled: boolean;
    selectedModelId: string | null;
    lastSelectedProfileId: string | null;
    lastSuccessfulModelId: string | null;
    visualStyle: VisualStyle;
    motionPreference: MotionPreference;
    modelAliases: Record<string, string>;
  } | null;
  profiles: ProfileEntry[];
  builtInTags: BuiltInTagEntry[];
  customTags: CustomTagEntry[];
  models: ModelEntry[];
  modelDownloads: Record<string, ModelDownloadState>;
  mlxRuntime: MlxRuntimeState | null;
  systemRamBytes: number | null;
  loadStatus: "idle" | "loading" | "loaded" | "error";
}

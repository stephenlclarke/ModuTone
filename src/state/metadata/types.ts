// Phase: 8
// Metadata state types — cached from backend

import type {
  BuiltInTagEntry,
  CustomTagEntry,
  ModelEntry,
  ModelDownloadProgressEvent,
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
  systemRamBytes: number | null;
  loadStatus: "idle" | "loading" | "loaded" | "error";
}

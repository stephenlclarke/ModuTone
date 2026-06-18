// Phase: 1
// All IPC types — mirrors Rust contracts in src-tauri/src/contracts/

// --- Global ---

export const CONTRACT_VERSION = 1 as const;
export type ContractVersion = typeof CONTRACT_VERSION;

export interface IpcError {
  code: string;
  message: string;
  detail?: string;
  subsystem: string;
}

export type CommandResponse<T> =
  | { ok: true; data: T }
  | { ok: false; error: IpcError };

// --- Shared Enums ---

export type TagCategory =
  | "audience"
  | "tone"
  | "format"
  | "clarity"
  | "length"
  | "directness"
  | "technicality"
  | "other";

export type ThemePreference = "system" | "light" | "dark";
export type VisualStyle =
  | "quiet-precision"
  | "luminous-professional"
  | "editorial-precision"
  | "glass-slate";
export type MotionPreference = "standard" | "reduced";
export type RequestKind = "initial_rewrite" | "refinement";
export type AppState = "ready" | "degraded";
export type WorkerState = "idle" | "warming" | "busy" | "unavailable";
export type ModelSuitability = "recommended" | "caution" | "unsupported";

// --- Settings ---

export interface SettingsGetResponse {
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
}

export interface SettingsUpdateRequest {
  contractVersion: ContractVersion;
  themePreference?: ThemePreference;
  trayEnabled?: boolean;
  launchAtLogin?: boolean;
  privacyBlackoutEnabled?: boolean;
  selectedModelId?: string | null;
  lastSelectedProfileId?: string | null;
  lastSuccessfulModelId?: string | null;
  visualStyle?: VisualStyle;
  motionPreference?: MotionPreference;
}

export interface SettingsUpdateResponse {
  updated: boolean;
}

// --- Profiles ---

export interface ProfileEntry {
  id: string;
  name: string;
  instructionBody: string;
  isFactoryDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ProfilesListResponse {
  profiles: ProfileEntry[];
}

export interface ProfileCreateRequest {
  contractVersion: ContractVersion;
  name: string;
  instructionBody: string;
}

export interface ProfileCreateResponse {
  id: string;
}

export interface ProfileUpdateRequest {
  contractVersion: ContractVersion;
  id: string;
  name?: string;
  instructionBody?: string;
}

export interface ProfileUpdateResponse {
  updated: boolean;
}

export interface ProfileDeleteRequest {
  contractVersion: ContractVersion;
  id: string;
}

export interface ProfileDeleteResponse {
  deleted: boolean;
}

export interface ProfileResetRequest {
  contractVersion: ContractVersion;
  id: string;
}

export interface ProfileResetResponse {
  reset: boolean;
}

// --- Tags ---

export interface BuiltInTagEntry {
  id: string;
  name: string;
  category: TagCategory;
  instructionBody: string;
  isBuiltIn: true;
  balancingGroup?: string;
}

export interface CustomTagEntry {
  id: string;
  name: string;
  category: TagCategory;
  instructionBody: string;
  isBuiltIn: false;
  createdAt: string;
  updatedAt: string;
}

export interface TagsListResponse {
  builtInTags: BuiltInTagEntry[];
  customTags: CustomTagEntry[];
}

export interface TagCreateRequest {
  contractVersion: ContractVersion;
  name: string;
  category: TagCategory;
  instructionBody: string;
}

export interface TagCreateResponse {
  id: string;
}

export interface TagUpdateRequest {
  contractVersion: ContractVersion;
  id: string;
  name?: string;
  category?: TagCategory;
  instructionBody?: string;
}

export interface TagUpdateResponse {
  updated: boolean;
}

export interface TagDeleteRequest {
  contractVersion: ContractVersion;
  id: string;
}

export interface TagDeleteResponse {
  deleted: boolean;
}

// --- Models ---

export interface ModelEntry {
  id: string;
  displayName: string;
  backend: "gguf" | "mlx";
  sizeBytes: number;
  ramClassLabel: string;
  minRamBytes: number;
  isInstalled: boolean;
  isCataloged: boolean;
  suitability: ModelSuitability;
  quantLabel: string | null;
  canDownload: boolean;
  downloadSizeBytes: number | null;
  downloadUnavailableReason: string | null;
}

// --- Model Aliases ---

export interface ModelAliasSetRequest {
  contractVersion: ContractVersion;
  modelId: string;
  alias: string;
}

export interface ModelAliasClearRequest {
  contractVersion: ContractVersion;
  modelId: string;
}

export interface ModelsListResponse {
  models: ModelEntry[];
  systemRamBytes: number;
  systemVramBytes: number | null;
}

export interface ModelDownloadStartRequest {
  contractVersion: ContractVersion;
  modelId: string;
}

export interface ModelDownloadStartResponse {
  started: boolean;
  alreadyInstalled: boolean;
  totalBytes: number;
}

export interface ModelDownloadCancelRequest {
  contractVersion: ContractVersion;
  modelId: string;
}

export interface ModelDownloadCancelResponse {
  canceled: boolean;
}

export interface MlxRuntimeStatusResponse {
  supported: boolean;
  installed: boolean;
  installing: boolean;
  installDir: string;
  pythonPath: string | null;
  unavailableReason: string | null;
}

export interface MlxRuntimeInstallStartRequest {
  contractVersion: ContractVersion;
}

export interface MlxRuntimeInstallStartResponse {
  started: boolean;
  alreadyInstalled: boolean;
  installDir: string;
  pythonPath: string | null;
}

// --- Runtime ---

export interface RuntimeStatusResponse {
  appState: AppState;
  workerState: WorkerState;
  loadedModelId: string | null;
  metadataStoreWritable: boolean;
  privacyBlackoutSupported: boolean;
  traySupported: boolean;
  launchAtLoginSupported: boolean;
}

export interface WarmModelRequest {
  contractVersion: ContractVersion;
  modelId: string;
}

// --- Generation ---

export interface StartInitialRequest {
  contractVersion: ContractVersion;
  tabId: string;
  modelId: string;
  profileId: string;
  activeTagIds: string[];
  sourceText: string;
  inputVersionToken: string;
}

export interface StartRefinementRequest {
  contractVersion: ContractVersion;
  tabId: string;
  modelId: string;
  profileId: string;
  activeTagIds: string[];
  acceptedOutput: string;
  acceptedOutputVersion: number;
  refinementInstruction: string;
  inputVersionToken: string;
}

export interface StartGenerationResponse {
  jobId: string;
}

export interface CancelGenerationRequest {
  contractVersion: ContractVersion;
  jobId: string;
  tabId: string;
}

// --- Platform ---

export interface SetBooleanRequest {
  contractVersion: ContractVersion;
  enabled: boolean;
}

export interface PlatformFeatureResponse {
  applied: boolean;
  supported: boolean;
}

// --- Events ---

export interface RuntimeStatusChangedEvent {
  contractVersion: ContractVersion;
  appState: AppState;
  workerState: WorkerState;
  loadedModelId: string | null;
  reason?: string;
  loadErrorClass?: string;
}

export interface GenerationStartedEvent {
  contractVersion: ContractVersion;
  jobId: string;
  tabId: string;
  requestKind: RequestKind;
}

export interface GenerationProgressEvent {
  contractVersion: ContractVersion;
  jobId: string;
  tabId: string;
  partialText?: string;
  tokenCount?: number;
}

export interface GenerationCompletedEvent {
  contractVersion: ContractVersion;
  jobId: string;
  tabId: string;
  requestKind: RequestKind;
  inputVersionToken: string;
  acceptedOutputVersion: number | null;
  outputText: string;
}

export interface GenerationFailedEvent {
  contractVersion: ContractVersion;
  jobId: string;
  tabId: string;
  requestKind: RequestKind;
  error: IpcError;
}

export interface GenerationCanceledEvent {
  contractVersion: ContractVersion;
  jobId: string;
  tabId: string;
  requestKind: RequestKind;
}

export interface WorkerCrashedEvent {
  contractVersion: ContractVersion;
  restartAttempt: number;
  willRestart: boolean;
  reason?: string;
}

export interface PrivacySupportStatusChangedEvent {
  contractVersion: ContractVersion;
  privacyBlackoutSupported: boolean;
  platform: string;
}

export type ModelDownloadStatus =
  | "queued"
  | "downloading"
  | "completed"
  | "failed"
  | "canceled";

export interface ModelDownloadProgressEvent {
  contractVersion: ContractVersion;
  modelId: string;
  status: ModelDownloadStatus;
  bytesDownloaded: number;
  totalBytes: number;
  fileName?: string;
  error?: string;
}

export type MlxRuntimeInstallStatus =
  | "queued"
  | "installing"
  | "completed"
  | "failed";

export interface MlxRuntimeInstallProgressEvent {
  contractVersion: ContractVersion;
  status: MlxRuntimeInstallStatus;
  step: string;
  detail?: string;
  error?: string;
}

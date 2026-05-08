// Phase: 7
// Typed command wrappers for all backend IPC calls

import { invokeCommand } from "./client";
import type {
  SettingsGetResponse,
  SettingsUpdateRequest,
  SettingsUpdateResponse,
  ModelAliasSetRequest,
  ModelAliasClearRequest,
  TagsListResponse,
  TagCreateRequest,
  TagCreateResponse,
  TagUpdateRequest,
  TagUpdateResponse,
  TagDeleteRequest,
  TagDeleteResponse,
  ProfilesListResponse,
  ProfileCreateRequest,
  ProfileCreateResponse,
  ProfileUpdateRequest,
  ProfileUpdateResponse,
  ProfileDeleteRequest,
  ProfileDeleteResponse,
  ProfileResetRequest,
  ProfileResetResponse,
  ModelsListResponse,
  RuntimeStatusResponse,
  WarmModelRequest,
  SetBooleanRequest,
  PlatformFeatureResponse,
  StartInitialRequest,
  StartRefinementRequest,
  StartGenerationResponse,
  CancelGenerationRequest,
} from "./types";

// --- Settings ---

export async function settingsGet(): Promise<SettingsGetResponse> {
  return invokeCommand<SettingsGetResponse>("settings_get");
}

export async function settingsUpdate(
  request: SettingsUpdateRequest,
): Promise<SettingsUpdateResponse> {
  return invokeCommand<SettingsUpdateResponse>("settings_update", { request });
}

// --- Model Aliases ---

export async function modelAliasSet(
  request: ModelAliasSetRequest,
): Promise<SettingsUpdateResponse> {
  return invokeCommand<SettingsUpdateResponse>("model_alias_set", { request });
}

export async function modelAliasClear(
  request: ModelAliasClearRequest,
): Promise<SettingsUpdateResponse> {
  return invokeCommand<SettingsUpdateResponse>("model_alias_clear", {
    request,
  });
}

// --- Tags ---

export async function tagsList(): Promise<TagsListResponse> {
  return invokeCommand<TagsListResponse>("tags_list");
}

export async function tagsCreate(
  request: TagCreateRequest,
): Promise<TagCreateResponse> {
  return invokeCommand<TagCreateResponse>("tags_create", { request });
}

export async function tagsUpdate(
  request: TagUpdateRequest,
): Promise<TagUpdateResponse> {
  return invokeCommand<TagUpdateResponse>("tags_update", { request });
}

export async function tagsDelete(
  request: TagDeleteRequest,
): Promise<TagDeleteResponse> {
  return invokeCommand<TagDeleteResponse>("tags_delete", { request });
}

// --- Profiles ---

export async function profilesList(): Promise<ProfilesListResponse> {
  return invokeCommand<ProfilesListResponse>("profiles_list");
}

export async function profilesCreate(
  request: ProfileCreateRequest,
): Promise<ProfileCreateResponse> {
  return invokeCommand<ProfileCreateResponse>("profiles_create", { request });
}

export async function profilesUpdate(
  request: ProfileUpdateRequest,
): Promise<ProfileUpdateResponse> {
  return invokeCommand<ProfileUpdateResponse>("profiles_update", { request });
}

export async function profilesDelete(
  request: ProfileDeleteRequest,
): Promise<ProfileDeleteResponse> {
  return invokeCommand<ProfileDeleteResponse>("profiles_delete", { request });
}

export async function profilesResetToDefault(
  request: ProfileResetRequest,
): Promise<ProfileResetResponse> {
  return invokeCommand<ProfileResetResponse>("profiles_reset_to_default", {
    request,
  });
}

// --- Models ---

export async function modelsList(): Promise<ModelsListResponse> {
  return invokeCommand<ModelsListResponse>("models_list");
}

// --- Runtime ---

export async function runtimeGetStatus(): Promise<RuntimeStatusResponse> {
  return invokeCommand<RuntimeStatusResponse>("runtime_get_status");
}

export async function runtimeWarmModel(
  request: WarmModelRequest,
): Promise<void> {
  return invokeCommand<void>("runtime_warm_model", { request });
}

// --- Platform ---

export async function appSetLaunchAtLogin(
  request: SetBooleanRequest,
): Promise<PlatformFeatureResponse> {
  return invokeCommand<PlatformFeatureResponse>("app_set_launch_at_login", {
    request,
  });
}

export async function appSetTrayEnabled(
  request: SetBooleanRequest,
): Promise<PlatformFeatureResponse> {
  return invokeCommand<PlatformFeatureResponse>("app_set_tray_enabled", {
    request,
  });
}

export async function appSetPrivacyBlackout(
  request: SetBooleanRequest,
): Promise<PlatformFeatureResponse> {
  return invokeCommand<PlatformFeatureResponse>("app_set_privacy_blackout", {
    request,
  });
}

// --- Generation ---

export async function generationStartInitial(
  request: StartInitialRequest,
): Promise<StartGenerationResponse> {
  return invokeCommand<StartGenerationResponse>("generation_start_initial", {
    request,
  });
}

export async function generationStartRefinement(
  request: StartRefinementRequest,
): Promise<StartGenerationResponse> {
  return invokeCommand<StartGenerationResponse>("generation_start_refinement", {
    request,
  });
}

export async function generationCancel(
  request: CancelGenerationRequest,
): Promise<void> {
  return invokeCommand<void>("generation_cancel", { request });
}

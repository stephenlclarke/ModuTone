// Phase: 1
// Root Zustand store with typed state slices

import { create } from "zustand";
import { createSessionSlice, type SessionSlice } from "./session/sessionSlice";
import {
  createMetadataSlice,
  type MetadataSlice,
} from "./metadata/metadataSlice";
import { createRuntimeSlice, type RuntimeSlice } from "./runtime/runtimeSlice";
import {
  createModelLoadingSlice,
  type ModelLoadingSlice,
} from "./modelLoading/modelLoadingSlice";

export type AppStore = SessionSlice &
  MetadataSlice &
  RuntimeSlice &
  ModelLoadingSlice;

export const useAppStore = create<AppStore>()((...args) => ({
  ...createSessionSlice(...args),
  ...createMetadataSlice(...args),
  ...createRuntimeSlice(...args),
  ...createModelLoadingSlice(...args),
}));

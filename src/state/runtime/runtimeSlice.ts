// Phase: 8
// Runtime slice — worker status, model status, platform capabilities, event-driven updates

import type { StateCreator } from "zustand";
import type { RuntimeState } from "./types";
import type {
  AppState,
  WorkerState,
  WorkerCrashedEvent,
} from "../../ipc/types";
import { runtimeGetStatus } from "../../ipc/commands";

export interface RuntimeSlice {
  runtime: RuntimeState;
  loadRuntimeStatus: () => Promise<void>;
  setRuntimeStatus: (
    appState: AppState,
    workerState: WorkerState,
    loadedModelId: string | null,
  ) => void;
  handleWorkerCrashed: (event: WorkerCrashedEvent) => void;
}

export const createRuntimeSlice: StateCreator<RuntimeSlice> = (set) => ({
  runtime: {
    appState: "ready",
    workerState: "unavailable",
    loadedModelId: null,
    metadataStoreWritable: true,
    privacyBlackoutSupported: false,
    traySupported: false,
    launchAtLoginSupported: false,
  },

  loadRuntimeStatus: async () => {
    try {
      const status = await runtimeGetStatus();
      set({
        runtime: {
          appState: status.appState,
          workerState: status.workerState,
          loadedModelId: status.loadedModelId,
          metadataStoreWritable: status.metadataStoreWritable,
          privacyBlackoutSupported: status.privacyBlackoutSupported,
          traySupported: status.traySupported,
          launchAtLoginSupported: status.launchAtLoginSupported,
        },
      });
    } catch {
      // Keep default state on failure
    }
  },

  setRuntimeStatus: (
    appState: AppState,
    workerState: WorkerState,
    loadedModelId: string | null,
  ) =>
    set((state) => ({
      runtime: {
        ...state.runtime,
        appState,
        workerState,
        loadedModelId,
      },
    })),

  handleWorkerCrashed: (_event: WorkerCrashedEvent) =>
    set((state) => ({
      runtime: {
        ...state.runtime,
        workerState: "unavailable" as WorkerState,
        appState: "degraded" as AppState,
      },
    })),
});

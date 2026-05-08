// Phase: 4
// Runtime state types

import type { AppState, WorkerState } from "../../ipc/types";

export interface RuntimeState {
  appState: AppState;
  workerState: WorkerState;
  loadedModelId: string | null;
  metadataStoreWritable: boolean;
  privacyBlackoutSupported: boolean;
  traySupported: boolean;
  launchAtLoginSupported: boolean;
}

// Phase: 5
// Event subscription setup — subscribes to Tauri backend events and
// routes them to the Zustand store.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useAppStore } from "../state/store";
import type {
  RuntimeStatusChangedEvent,
  WorkerCrashedEvent,
  GenerationStartedEvent,
  GenerationCompletedEvent,
  GenerationFailedEvent,
  GenerationCanceledEvent,
  ModelDownloadProgressEvent,
} from "./types";

/**
 * Subscribe to all backend events. Returns an unlisten function that
 * removes all subscriptions.
 *
 * Called once from AppShell on mount. Each listener updates the Zustand
 * store directly (no component re-render dependency).
 */
export async function subscribeToBackendEvents(): Promise<UnlistenFn> {
  const unlisteners: UnlistenFn[] = [];

  // runtime:status-changed — worker/model state updates
  unlisteners.push(
    await listen<RuntimeStatusChangedEvent>(
      "runtime:status-changed",
      (event) => {
        const { appState, workerState, loadedModelId, loadErrorClass } =
          event.payload;
        const store = useAppStore.getState();
        store.setRuntimeStatus(appState, workerState, loadedModelId);
        const loadEvent: {
          loadedModelId: string | null;
          workerState: string;
          loadErrorClass?: string;
        } = { loadedModelId, workerState };
        if (loadErrorClass !== undefined) {
          loadEvent.loadErrorClass = loadErrorClass;
        }
        store.handleModelLoadEvent(loadEvent);
      },
    ),
  );

  // worker:crashed — worker process crash notifications
  unlisteners.push(
    await listen<WorkerCrashedEvent>("worker:crashed", (event) => {
      useAppStore.getState().handleWorkerCrashed(event.payload);
    }),
  );

  // generation:started — job accepted by worker
  unlisteners.push(
    await listen<GenerationStartedEvent>("generation:started", (event) => {
      useAppStore.getState().handleGenerationStarted(event.payload);
    }),
  );

  // generation:completed — job finished successfully
  unlisteners.push(
    await listen<GenerationCompletedEvent>("generation:completed", (event) => {
      useAppStore.getState().handleGenerationCompleted(event.payload);
    }),
  );

  // generation:failed — job encountered an error
  unlisteners.push(
    await listen<GenerationFailedEvent>("generation:failed", (event) => {
      useAppStore.getState().handleGenerationFailed(event.payload);
    }),
  );

  // generation:canceled — job was canceled
  unlisteners.push(
    await listen<GenerationCanceledEvent>("generation:canceled", (event) => {
      useAppStore.getState().handleGenerationCanceled(event.payload);
    }),
  );

  // model:download-progress — explicit model download lifecycle updates
  unlisteners.push(
    await listen<ModelDownloadProgressEvent>(
      "model:download-progress",
      (event) => {
        useAppStore.getState().handleModelDownloadProgress(event.payload);
      },
    ),
  );

  // Return a combined unlisten function
  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}

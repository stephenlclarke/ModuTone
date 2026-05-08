// Model loading slice — state machine for auto-load, retry, and fallback

import type { StateCreator } from "zustand";
import type { ModelLoadingState, ModelLoadErrorClass } from "./types";
import { MAX_RETRIES } from "./types";
import type { RuntimeSlice } from "../runtime/runtimeSlice";
import type { MetadataSlice } from "../metadata/metadataSlice";

export interface ModelLoadingSlice {
  modelLoading: ModelLoadingState;
  initiateModelLoad: (modelId: string) => void;
  handleModelLoadEvent: (event: {
    loadedModelId: string | null;
    workerState: string;
    loadErrorClass?: string;
  }) => void;
  handleLoadIpcError: (code: string, message?: string) => void;
  startRetryAttempt: () => void;
  initializeLastKnownGood: (modelId: string | null) => void;
}

/** The slice reads from runtime and metadata but doesn't need session. */
type ModelLoadingDeps = RuntimeSlice & MetadataSlice & ModelLoadingSlice;

function classifyIpcError(code: string): ModelLoadErrorClass | "success" {
  switch (code) {
    case "MODEL_ALREADY_LOADED":
      return "success";
    case "MODEL_NOT_FOUND":
    case "MODEL_NOT_INSTALLED":
      return "model_invalid";
    case "WORKER_UNAVAILABLE":
    case "WORKER_SEND_FAILED":
      return "transient";
    default:
      return "transient";
  }
}

export const createModelLoadingSlice: StateCreator<
  ModelLoadingDeps,
  [],
  [],
  ModelLoadingSlice
> = (set, get) => ({
  modelLoading: {
    phase: "idle",
    targetModelId: null,
    originalTargetModelId: null,
    retryCount: 0,
    lastError: null,
    lastKnownGoodModelId: null,
    fallbackModelId: null,
  },

  initializeLastKnownGood: (modelId: string | null) => {
    set((state) => ({
      modelLoading: {
        ...state.modelLoading,
        lastKnownGoodModelId: modelId,
      },
    }));
  },

  initiateModelLoad: (modelId: string) => {
    const { runtime, modelLoading } = get();
    // Record current loaded model as last known good (if different from target and exists)
    const currentLoaded = runtime.loadedModelId;
    const newLastKnownGood =
      currentLoaded && currentLoaded !== modelId
        ? currentLoaded
        : modelLoading.lastKnownGoodModelId;

    set({
      modelLoading: {
        phase: "loading",
        targetModelId: modelId,
        originalTargetModelId: null,
        retryCount: 0,
        lastError: null,
        lastKnownGoodModelId: newLastKnownGood,
        fallbackModelId: null,
      },
    });
  },

  handleLoadIpcError: (code: string, message?: string) => {
    const classification = classifyIpcError(code);

    // MODEL_ALREADY_LOADED is treated as success
    if (classification === "success") {
      const { modelLoading } = get();
      const targetId = modelLoading.targetModelId;
      set((state) => ({
        modelLoading: {
          ...state.modelLoading,
          phase: "idle",
          lastError: null,
          lastKnownGoodModelId:
            targetId ?? state.modelLoading.lastKnownGoodModelId,
          fallbackModelId: null,
        },
      }));
      // Persist last successful
      if (targetId) {
        get()
          .updateSettings({ lastSuccessfulModelId: targetId })
          .catch(() => {});
      }
      return;
    }

    const errorMessage = message ?? `IPC error: ${code}`;

    if (classification !== "transient") {
      // Non-retryable — go to failed/fallback
      goToFailedOrFallback(set, get, {
        message: errorMessage,
        classification,
      });
      return;
    }

    // Transient — check retry budget
    const { modelLoading } = get();
    if (modelLoading.retryCount < MAX_RETRIES) {
      set((state) => ({
        modelLoading: {
          ...state.modelLoading,
          phase: "waiting_retry",
          retryCount: state.modelLoading.retryCount + 1,
          lastError: { message: errorMessage, classification },
        },
      }));
    } else {
      goToFailedOrFallback(set, get, {
        message: errorMessage,
        classification,
      });
    }
  },

  handleModelLoadEvent: (event) => {
    const { modelLoading } = get();

    // Only process events during loading or fallback_loading phases
    if (
      modelLoading.phase !== "loading" &&
      modelLoading.phase !== "fallback_loading"
    ) {
      return;
    }

    const isFallback = modelLoading.phase === "fallback_loading";

    // Success: target model is loaded
    if (
      event.loadedModelId &&
      event.loadedModelId === modelLoading.targetModelId
    ) {
      if (isFallback) {
        set((state) => ({
          modelLoading: {
            ...state.modelLoading,
            phase: "fallback_active",
            fallbackModelId: event.loadedModelId,
            lastKnownGoodModelId: event.loadedModelId!,
            lastError: null,
          },
        }));
      } else {
        set((state) => ({
          modelLoading: {
            ...state.modelLoading,
            phase: "idle",
            lastError: null,
            lastKnownGoodModelId: event.loadedModelId!,
            fallbackModelId: null,
          },
        }));
      }
      // Persist last successful model
      get()
        .updateSettings({ lastSuccessfulModelId: event.loadedModelId! })
        .catch(() => {});
      return;
    }

    // Failure: worker went idle without loading our target
    if (event.workerState === "idle" && event.loadErrorClass) {
      const classification =
        (event.loadErrorClass as ModelLoadErrorClass) ?? "transient";
      let errorMessage: string;
      switch (classification) {
        case "model_invalid":
          errorMessage =
            "Model file appears incomplete or corrupt and cannot be loaded";
          break;
        case "insufficient_memory":
          errorMessage = "Not enough system memory to load this model";
          break;
        default:
          errorMessage = `Model load failed (${classification})`;
      }

      if (isFallback) {
        // Fallback also failed — terminal failure
        set((state) => ({
          modelLoading: {
            ...state.modelLoading,
            phase: "failed",
            lastError: { message: errorMessage, classification },
          },
        }));
        return;
      }

      if (classification !== "transient") {
        goToFailedOrFallback(set, get, {
          message: errorMessage,
          classification,
        });
        return;
      }

      // Transient — check retry budget
      if (modelLoading.retryCount < MAX_RETRIES) {
        set((state) => ({
          modelLoading: {
            ...state.modelLoading,
            phase: "waiting_retry",
            retryCount: state.modelLoading.retryCount + 1,
            lastError: { message: errorMessage, classification },
          },
        }));
      } else {
        goToFailedOrFallback(set, get, {
          message: errorMessage,
          classification,
        });
      }
    }
  },

  startRetryAttempt: () => {
    set((state) => ({
      modelLoading: {
        ...state.modelLoading,
        phase: "loading",
      },
    }));
  },
});

function goToFailedOrFallback(
  set: Parameters<StateCreator<ModelLoadingDeps>>[0],
  get: Parameters<StateCreator<ModelLoadingDeps>>[1],
  error: { message: string; classification: ModelLoadErrorClass },
) {
  const { modelLoading } = get();
  const { lastKnownGoodModelId, targetModelId } = modelLoading;

  if (lastKnownGoodModelId && lastKnownGoodModelId !== targetModelId) {
    // Attempt fallback to last known good
    set({
      modelLoading: {
        ...modelLoading,
        phase: "fallback_loading",
        originalTargetModelId: targetModelId,
        targetModelId: lastKnownGoodModelId,
        retryCount: 0,
        lastError: error,
        fallbackModelId: null,
      },
    });
  } else {
    // No fallback available
    set((state) => ({
      modelLoading: {
        ...state.modelLoading,
        phase: "failed",
        lastError: error,
      },
    }));
  }
}

// Model loading slice tests — state machine for auto-load, retry, and fallback

import { describe, it, expect, beforeEach, vi } from "vitest";
import { create } from "zustand";
import {
  createModelLoadingSlice,
  type ModelLoadingSlice,
} from "../modelLoadingSlice";
import {
  createRuntimeSlice,
  type RuntimeSlice,
} from "../../runtime/runtimeSlice";
import {
  createMetadataSlice,
  type MetadataSlice,
} from "../../metadata/metadataSlice";
import { MAX_RETRIES } from "../types";

vi.mock("../../../ipc/commands", () => ({
  settingsGet: vi.fn().mockResolvedValue({
    schemaVersion: 1,
    themePreference: "system",
    trayEnabled: false,
    launchAtLogin: false,
    privacyBlackoutEnabled: false,
    selectedModelId: null,
    lastSelectedProfileId: null,
    lastSuccessfulModelId: null,
    visualStyle: "quiet-precision",
    motionPreference: "standard",
  }),
  settingsUpdate: vi.fn().mockResolvedValue({ updated: true }),
  tagsList: vi.fn().mockResolvedValue({ builtInTags: [], customTags: [] }),
  profilesList: vi.fn().mockResolvedValue({ profiles: [] }),
  modelsList: vi.fn().mockResolvedValue({ models: [], systemRamBytes: 0 }),
  runtimeGetStatus: vi.fn().mockResolvedValue({
    appState: "ready",
    workerState: "idle",
    loadedModelId: null,
    metadataStoreWritable: true,
    privacyBlackoutSupported: false,
    traySupported: false,
    launchAtLoginSupported: false,
  }),
}));

type TestStore = RuntimeSlice & MetadataSlice & ModelLoadingSlice;

function createTestStore() {
  return create<TestStore>()((...args) => ({
    ...createRuntimeSlice(...args),
    ...createMetadataSlice(...args),
    ...createModelLoadingSlice(...args),
  }));
}

describe("modelLoadingSlice", () => {
  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    vi.clearAllMocks();
    store = createTestStore();
  });

  // --- Initial state ---

  it("initializes with idle phase", () => {
    expect(store.getState().modelLoading.phase).toBe("idle");
  });

  it("initializes with null targetModelId", () => {
    expect(store.getState().modelLoading.targetModelId).toBeNull();
  });

  it("initializes with zero retryCount", () => {
    expect(store.getState().modelLoading.retryCount).toBe(0);
  });

  // --- initiateModelLoad ---

  it("transitions to loading phase on initiateModelLoad", () => {
    store.getState().initiateModelLoad("model-a");
    const { phase, targetModelId, retryCount } = store.getState().modelLoading;
    expect(phase).toBe("loading");
    expect(targetModelId).toBe("model-a");
    expect(retryCount).toBe(0);
  });

  it("records current loaded model as lastKnownGood on initiate", () => {
    // Simulate a model already loaded in runtime
    store.getState().setRuntimeStatus("ready", "idle", "model-b");

    store.getState().initiateModelLoad("model-a");
    expect(store.getState().modelLoading.lastKnownGoodModelId).toBe("model-b");
  });

  it("does not override lastKnownGood when loading the same model", () => {
    store.getState().setRuntimeStatus("ready", "idle", "model-a");
    store.getState().initializeLastKnownGood("model-prev");

    store.getState().initiateModelLoad("model-a");
    // Should keep "model-prev" (not overwrite with "model-a" since target == loaded)
    expect(store.getState().modelLoading.lastKnownGoodModelId).toBe(
      "model-prev",
    );
  });

  // --- Successful auto-load on selection ---

  it("transitions to idle on successful model load event", () => {
    store.getState().initiateModelLoad("model-a");
    expect(store.getState().modelLoading.phase).toBe("loading");

    store.getState().handleModelLoadEvent({
      loadedModelId: "model-a",
      workerState: "idle",
    });

    const { phase, lastKnownGoodModelId, fallbackModelId } =
      store.getState().modelLoading;
    expect(phase).toBe("idle");
    expect(lastKnownGoodModelId).toBe("model-a");
    expect(fallbackModelId).toBeNull();
  });

  // --- IPC error: MODEL_ALREADY_LOADED treated as success ---

  it("treats MODEL_ALREADY_LOADED as success", () => {
    store.getState().initiateModelLoad("model-a");
    store.getState().handleLoadIpcError("MODEL_ALREADY_LOADED");

    expect(store.getState().modelLoading.phase).toBe("idle");
  });

  // --- Transient failure with automatic retries ---

  it("transitions to waiting_retry on transient failure", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "transient",
    });

    const { phase, retryCount } = store.getState().modelLoading;
    expect(phase).toBe("waiting_retry");
    expect(retryCount).toBe(1);
  });

  it("transitions back to loading on startRetryAttempt", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "transient",
    });
    expect(store.getState().modelLoading.phase).toBe("waiting_retry");

    store.getState().startRetryAttempt();
    expect(store.getState().modelLoading.phase).toBe("loading");
  });

  // --- Stops after MAX_RETRIES attempts ---

  it("stops retrying after MAX_RETRIES and goes to failed (no fallback)", () => {
    store.getState().initiateModelLoad("model-a");

    // Initial attempt fails (retryCount goes to 1), then 19 more retries
    // Total: 1 initial + 20 retries = 21 failure events
    for (let i = 0; i <= MAX_RETRIES; i++) {
      store.getState().handleModelLoadEvent({
        loadedModelId: null,
        workerState: "idle",
        loadErrorClass: "transient",
      });

      if (i < MAX_RETRIES) {
        expect(store.getState().modelLoading.phase).toBe("waiting_retry");
        store.getState().startRetryAttempt();
      }
    }

    // After exhausting all retries, should transition to failed
    expect(store.getState().modelLoading.phase).toBe("failed");
  });

  // --- Immediate stop on non-recoverable errors ---

  it("immediately fails on model_invalid error (no retries)", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "model_invalid",
    });

    expect(store.getState().modelLoading.phase).toBe("failed");
    expect(store.getState().modelLoading.retryCount).toBe(0);
  });

  it("immediately fails on insufficient_memory error (no retries)", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "insufficient_memory",
    });

    expect(store.getState().modelLoading.phase).toBe("failed");
  });

  it("immediately fails on MODEL_NOT_FOUND IPC error (model_invalid)", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleLoadIpcError("MODEL_NOT_FOUND", "Model not found");

    expect(store.getState().modelLoading.phase).toBe("failed");
  });

  it("immediately fails on MODEL_NOT_INSTALLED IPC error (model_invalid)", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleLoadIpcError("MODEL_NOT_INSTALLED", "Not installed");

    expect(store.getState().modelLoading.phase).toBe("failed");
  });

  // --- Fallback to last working model ---

  it("falls back to lastKnownGoodModelId on non-retryable failure", () => {
    // Simulate a previously loaded model
    store.getState().initializeLastKnownGood("model-b");
    store.getState().initiateModelLoad("model-a");

    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "model_invalid",
    });

    const { phase, targetModelId, originalTargetModelId } =
      store.getState().modelLoading;
    expect(phase).toBe("fallback_loading");
    expect(targetModelId).toBe("model-b");
    expect(originalTargetModelId).toBe("model-a");
  });

  it("sets fallback_active when fallback model loads successfully", () => {
    store.getState().initializeLastKnownGood("model-b");
    store.getState().initiateModelLoad("model-a");

    // Target fails
    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "model_invalid",
    });
    expect(store.getState().modelLoading.phase).toBe("fallback_loading");

    // Fallback succeeds
    store.getState().handleModelLoadEvent({
      loadedModelId: "model-b",
      workerState: "idle",
    });

    const { phase, fallbackModelId, lastKnownGoodModelId } =
      store.getState().modelLoading;
    expect(phase).toBe("fallback_active");
    expect(fallbackModelId).toBe("model-b");
    expect(lastKnownGoodModelId).toBe("model-b");
  });

  it("goes to failed when fallback also fails", () => {
    store.getState().initializeLastKnownGood("model-b");
    store.getState().initiateModelLoad("model-a");

    // Target fails
    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "model_invalid",
    });
    expect(store.getState().modelLoading.phase).toBe("fallback_loading");

    // Fallback also fails
    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "model_invalid",
    });

    expect(store.getState().modelLoading.phase).toBe("failed");
  });

  // --- No-fallback failure state ---

  it("goes to failed when no lastKnownGoodModelId exists", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "model_invalid",
    });

    expect(store.getState().modelLoading.phase).toBe("failed");
    expect(store.getState().modelLoading.fallbackModelId).toBeNull();
  });

  it("goes to failed when lastKnownGood is same as target", () => {
    store.getState().initializeLastKnownGood("model-a");
    store.getState().initiateModelLoad("model-a");

    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "model_invalid",
    });

    // Can't fall back to the same model that just failed
    expect(store.getState().modelLoading.phase).toBe("failed");
  });

  // --- Re-selection cancels retry ---

  it("resets state when a different model is selected during retry", () => {
    store.getState().initiateModelLoad("model-a");

    // Fail with transient error, enter waiting_retry
    store.getState().handleModelLoadEvent({
      loadedModelId: null,
      workerState: "idle",
      loadErrorClass: "transient",
    });
    expect(store.getState().modelLoading.phase).toBe("waiting_retry");
    expect(store.getState().modelLoading.retryCount).toBe(1);

    // User selects a different model
    store.getState().initiateModelLoad("model-c");

    const {
      phase,
      targetModelId,
      retryCount,
      fallbackModelId,
      originalTargetModelId,
    } = store.getState().modelLoading;
    expect(phase).toBe("loading");
    expect(targetModelId).toBe("model-c");
    expect(retryCount).toBe(0);
    expect(fallbackModelId).toBeNull();
    expect(originalTargetModelId).toBeNull();
  });

  // --- Event ignored in idle phase ---

  it("ignores load events when phase is idle", () => {
    store.getState().handleModelLoadEvent({
      loadedModelId: "model-x",
      workerState: "idle",
    });

    expect(store.getState().modelLoading.phase).toBe("idle");
    expect(store.getState().modelLoading.targetModelId).toBeNull();
  });

  // --- Transient IPC errors ---

  it("retries on WORKER_UNAVAILABLE IPC error (transient)", () => {
    store.getState().initiateModelLoad("model-a");

    store
      .getState()
      .handleLoadIpcError("WORKER_UNAVAILABLE", "Worker not idle");

    const { phase, retryCount } = store.getState().modelLoading;
    expect(phase).toBe("waiting_retry");
    expect(retryCount).toBe(1);
  });

  it("retries on WORKER_SEND_FAILED IPC error (transient)", () => {
    store.getState().initiateModelLoad("model-a");

    store.getState().handleLoadIpcError("WORKER_SEND_FAILED", "Send failed");

    const { phase, retryCount } = store.getState().modelLoading;
    expect(phase).toBe("waiting_retry");
    expect(retryCount).toBe(1);
  });

  // --- Fallback after exhausted transient retries ---

  it("falls back to last known good after exhausting transient retries", () => {
    store.getState().initializeLastKnownGood("model-b");
    store.getState().initiateModelLoad("model-a");

    // Initial attempt fails, then MAX_RETRIES more retries
    for (let i = 0; i <= MAX_RETRIES; i++) {
      store.getState().handleModelLoadEvent({
        loadedModelId: null,
        workerState: "idle",
        loadErrorClass: "transient",
      });

      if (i < MAX_RETRIES) {
        store.getState().startRetryAttempt();
      }
    }

    // Should now be trying fallback
    expect(store.getState().modelLoading.phase).toBe("fallback_loading");
    expect(store.getState().modelLoading.targetModelId).toBe("model-b");
  });
});

// Phase: 8
// Runtime slice tests — event-driven state updates + loadRuntimeStatus

import { describe, it, expect, beforeEach, vi } from "vitest";
import { create } from "zustand";
import { createRuntimeSlice, type RuntimeSlice } from "./runtimeSlice";

vi.mock("../../ipc/commands", () => ({
  runtimeGetStatus: vi.fn(),
}));

import { runtimeGetStatus } from "../../ipc/commands";

const mockRuntimeGetStatus = vi.mocked(runtimeGetStatus);

function createTestStore() {
  return create<RuntimeSlice>()((...args) => ({
    ...createRuntimeSlice(...args),
  }));
}

describe("runtimeSlice", () => {
  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    vi.clearAllMocks();
    store = createTestStore();
  });

  // --- Initial state ---

  it("initializes with ready appState", () => {
    expect(store.getState().runtime.appState).toBe("ready");
  });

  it("initializes with unavailable workerState", () => {
    expect(store.getState().runtime.workerState).toBe("unavailable");
  });

  it("initializes with null loadedModelId", () => {
    expect(store.getState().runtime.loadedModelId).toBeNull();
  });

  it("initializes with metadataStoreWritable true", () => {
    expect(store.getState().runtime.metadataStoreWritable).toBe(true);
  });

  it("initializes platform support flags as false", () => {
    const { runtime } = store.getState();
    expect(runtime.privacyBlackoutSupported).toBe(false);
    expect(runtime.traySupported).toBe(false);
    expect(runtime.launchAtLoginSupported).toBe(false);
  });

  // --- loadRuntimeStatus ---

  it("loadRuntimeStatus populates all fields from backend", async () => {
    mockRuntimeGetStatus.mockResolvedValue({
      appState: "ready",
      workerState: "idle",
      loadedModelId: null,
      metadataStoreWritable: true,
      privacyBlackoutSupported: true,
      traySupported: false,
      launchAtLoginSupported: false,
    });

    await store.getState().loadRuntimeStatus();

    const { runtime } = store.getState();
    expect(runtime.appState).toBe("ready");
    expect(runtime.workerState).toBe("idle");
    expect(runtime.privacyBlackoutSupported).toBe(true);
    expect(runtime.traySupported).toBe(false);
    expect(runtime.launchAtLoginSupported).toBe(false);
  });

  it("loadRuntimeStatus keeps defaults on failure", async () => {
    mockRuntimeGetStatus.mockRejectedValue(new Error("fail"));

    await store.getState().loadRuntimeStatus();

    const { runtime } = store.getState();
    expect(runtime.appState).toBe("ready");
    expect(runtime.privacyBlackoutSupported).toBe(false);
  });

  // --- setRuntimeStatus ---

  it("setRuntimeStatus updates appState, workerState, loadedModelId", () => {
    store.getState().setRuntimeStatus("ready", "idle", null);
    const { runtime } = store.getState();
    expect(runtime.appState).toBe("ready");
    expect(runtime.workerState).toBe("idle");
    expect(runtime.loadedModelId).toBeNull();
  });

  it("setRuntimeStatus sets loadedModelId when provided", () => {
    store.getState().setRuntimeStatus("ready", "idle", "model-abc");
    expect(store.getState().runtime.loadedModelId).toBe("model-abc");
  });

  it("setRuntimeStatus to degraded with unavailable worker", () => {
    store.getState().setRuntimeStatus("degraded", "unavailable", null);
    const { runtime } = store.getState();
    expect(runtime.appState).toBe("degraded");
    expect(runtime.workerState).toBe("unavailable");
  });

  it("setRuntimeStatus to busy state", () => {
    store.getState().setRuntimeStatus("ready", "busy", "model-1");
    const { runtime } = store.getState();
    expect(runtime.appState).toBe("ready");
    expect(runtime.workerState).toBe("busy");
    expect(runtime.loadedModelId).toBe("model-1");
  });

  it("setRuntimeStatus to warming state", () => {
    store.getState().setRuntimeStatus("ready", "warming", null);
    expect(store.getState().runtime.workerState).toBe("warming");
  });

  it("setRuntimeStatus preserves other runtime fields", () => {
    store.getState().setRuntimeStatus("ready", "idle", "model-1");
    const { runtime } = store.getState();
    // These fields should remain at their initial values
    expect(runtime.metadataStoreWritable).toBe(true);
    expect(runtime.privacyBlackoutSupported).toBe(false);
    expect(runtime.traySupported).toBe(false);
    expect(runtime.launchAtLoginSupported).toBe(false);
  });

  // --- handleWorkerCrashed ---

  it("handleWorkerCrashed sets degraded + unavailable", () => {
    // Start from ready/idle
    store.getState().setRuntimeStatus("ready", "idle", "model-1");

    store.getState().handleWorkerCrashed({
      contractVersion: 1,
      restartAttempt: 1,
      willRestart: true,
      reason: "Worker process exited unexpectedly",
    });

    const { runtime } = store.getState();
    expect(runtime.appState).toBe("degraded");
    expect(runtime.workerState).toBe("unavailable");
  });

  it("handleWorkerCrashed when restart limit exceeded", () => {
    store.getState().handleWorkerCrashed({
      contractVersion: 1,
      restartAttempt: 3,
      willRestart: false,
      reason: "Restart limit exceeded",
    });

    const { runtime } = store.getState();
    expect(runtime.appState).toBe("degraded");
    expect(runtime.workerState).toBe("unavailable");
  });

  it("handleWorkerCrashed preserves other runtime fields", () => {
    store.getState().handleWorkerCrashed({
      contractVersion: 1,
      restartAttempt: 1,
      willRestart: true,
    });

    const { runtime } = store.getState();
    expect(runtime.metadataStoreWritable).toBe(true);
  });

  // --- Recovery sequence ---

  it("crash followed by recovery restores ready state", () => {
    // Worker becomes idle
    store.getState().setRuntimeStatus("ready", "idle", "model-1");
    expect(store.getState().runtime.appState).toBe("ready");

    // Worker crashes
    store.getState().handleWorkerCrashed({
      contractVersion: 1,
      restartAttempt: 1,
      willRestart: true,
    });
    expect(store.getState().runtime.appState).toBe("degraded");

    // Worker recovers
    store.getState().setRuntimeStatus("ready", "idle", null);
    expect(store.getState().runtime.appState).toBe("ready");
    expect(store.getState().runtime.workerState).toBe("idle");
    // Model is lost after crash/restart
    expect(store.getState().runtime.loadedModelId).toBeNull();
  });
});

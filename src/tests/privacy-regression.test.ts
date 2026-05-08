// Phase: 10
// Privacy regression tests (Vitest side)
//
// PR-4: localStorage/sessionStorage empty
// PR-7: No network requests during generation flow
// PR-9: Clipboard only from explicit action

import { describe, it, expect, afterEach, vi } from "vitest";
import { create } from "zustand";
import {
  createSessionSlice,
  type SessionSlice,
} from "../state/session/sessionSlice";

function createTestStore() {
  return create<SessionSlice>()(createSessionSlice);
}

describe("privacy regression (frontend)", () => {
  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // PR-4: localStorage/sessionStorage empty
  it("PR-4: localStorage remains empty after session operations", () => {
    const store = createTestStore();
    const tabId = store.getState().tabs[0]!.id;

    // Perform typical session operations
    store.getState().updateInputText(tabId, "sensitive user content");
    store.getState().createTab();
    store.getState().addToast("test", "success");

    // localStorage should be completely empty
    expect(localStorage.length).toBe(0);
  });

  it("PR-4: sessionStorage remains empty after session operations", () => {
    const store = createTestStore();
    const tabId = store.getState().tabs[0]!.id;

    store.getState().updateInputText(tabId, "sensitive user content");
    store.getState().handleGenerationStarted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });

    expect(sessionStorage.length).toBe(0);
  });

  it("PR-4: Zustand store does not use persist middleware", () => {
    // After importing and creating the store, localStorage should remain empty
    createTestStore();
    expect(localStorage.length).toBe(0);
  });

  // PR-7: No network requests
  it("PR-7: global fetch is never called during generation event flow", () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const store = createTestStore();
    const tabId = store.getState().tabs[0]!.id;

    // Simulate full generation lifecycle
    store.getState().updateInputText(tabId, "user input text");
    const token = store.getState().tabs[0]!.inputVersionToken;

    store.getState().handleGenerationStarted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });

    store.getState().handleGenerationCompleted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
      inputVersionToken: token,
      acceptedOutputVersion: null,
      outputText: "generated output",
    });

    store.getState().acceptProposal(tabId);

    // fetch should never have been called
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("PR-7: no XMLHttpRequest during session operations", () => {
    const xhrOpenSpy = vi.spyOn(XMLHttpRequest.prototype, "open");
    const store = createTestStore();
    const tabId = store.getState().tabs[0]!.id;

    store.getState().updateInputText(tabId, "test");
    store.getState().toggleTag(tabId, "tag-1");
    store.getState().createTab();

    expect(xhrOpenSpy).not.toHaveBeenCalled();
  });

  // PR-9: Clipboard only from explicit action
  it("PR-9: copyAcceptedOutput is the only clipboard write path", async () => {
    // Mock clipboard API
    const writeTextSpy = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: writeTextSpy },
      writable: true,
      configurable: true,
    });

    const store = createTestStore();
    const tabId = store.getState().tabs[0]!.id;

    // Set up state with accepted output
    store.setState((state) => ({
      tabs: state.tabs.map((t) =>
        t.id === tabId
          ? {
              ...t,
              inputText: "input",
              acceptedOutput: "output to copy",
              status: "output_ready" as const,
            }
          : t,
      ),
    }));

    // Before explicit copy, clipboard should not have been touched
    expect(writeTextSpy).not.toHaveBeenCalled();

    // Explicit copy action
    await store.getState().copyAcceptedOutput(tabId);

    // Now clipboard was written with the correct content
    expect(writeTextSpy).toHaveBeenCalledWith("output to copy");
    expect(writeTextSpy).toHaveBeenCalledTimes(1);
  });

  it("PR-9: no clipboard write occurs during generation flow", async () => {
    const writeTextSpy = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: writeTextSpy },
      writable: true,
      configurable: true,
    });

    const store = createTestStore();
    const tabId = store.getState().tabs[0]!.id;
    store.getState().updateInputText(tabId, "input");
    const token = store.getState().tabs[0]!.inputVersionToken;

    // Full generation lifecycle without explicit copy
    store.getState().handleGenerationStarted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });
    store.getState().handleGenerationCompleted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
      inputVersionToken: token,
      acceptedOutputVersion: null,
      outputText: "output",
    });

    // Clipboard should not have been touched
    expect(writeTextSpy).not.toHaveBeenCalled();
  });
});

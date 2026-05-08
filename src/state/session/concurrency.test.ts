// Phase: 10
// Concurrency tests for session slice (RC-2 through RC-7)
//
// Tests target real existing Zustand actions, attached to the session layer.

import { describe, it, expect, beforeEach } from "vitest";
import { create } from "zustand";
import { createSessionSlice, type SessionSlice } from "./sessionSlice";

function createTestStore() {
  return create<SessionSlice>()(createSessionSlice);
}

describe("session concurrency", () => {
  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    store = createTestStore();
  });

  // RC-2: Cancel after complete (cancel-wins)
  it("RC-2: cancel after complete is a no-op (no double state change)", () => {
    const tabId = store.getState().tabs[0]!.id;
    store.getState().updateInputText(tabId, "text");
    const token = store.getState().tabs[0]!.inputVersionToken;

    // Start generation
    store.getState().handleGenerationStarted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });

    // Complete
    store.getState().handleGenerationCompleted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
      inputVersionToken: token,
      acceptedOutputVersion: null,
      outputText: "completed output",
    });
    expect(store.getState().tabs[0]!.status).toBe("output_ready");
    expect(store.getState().tabs[0]!.acceptedOutput).toBe("completed output");

    // Late cancel arrives — should be ignored (no matching active job)
    store.getState().handleGenerationCanceled({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });

    // State should be unchanged (cancel-wins: complete already applied, cancel is no-op)
    expect(store.getState().tabs[0]!.status).toBe("output_ready");
    expect(store.getState().tabs[0]!.acceptedOutput).toBe("completed output");
    expect(store.getState().tabs[0]!.activeJob).toBeNull();
  });

  // RC-3: Tab switch during generation
  it("RC-3: switching tabs during generation does not affect generating tab", () => {
    const tabId = store.getState().tabs[0]!.id;
    store.getState().updateInputText(tabId, "text");
    const token = store.getState().tabs[0]!.inputVersionToken;

    // Start generation on tab 1
    store.getState().handleGenerationStarted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });
    expect(store.getState().tabs[0]!.status).toBe("generating");

    // Create and switch to another tab
    store.getState().createTab();
    const tab2Id = store.getState().tabs[1]!.id;
    store.getState().switchTab(tab2Id);
    expect(store.getState().activeTabId).toBe(tab2Id);

    // Tab 1 should still be generating
    expect(store.getState().tabs[0]!.status).toBe("generating");
    expect(store.getState().tabs[0]!.activeJob?.jobId).toBe("job-1");

    // Complete generation on tab 1
    store.getState().handleGenerationCompleted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
      inputVersionToken: token,
      acceptedOutputVersion: null,
      outputText: "result",
    });

    // Tab 1 should be output_ready, active tab is still tab 2
    expect(store.getState().tabs[0]!.status).toBe("output_ready");
    expect(store.getState().activeTabId).toBe(tab2Id);
  });

  // RC-4: Tab close during generation
  it("RC-4: requestCloseTab on tab with active job shows confirmation dialog", () => {
    const tabId = store.getState().tabs[0]!.id;
    store.getState().updateInputText(tabId, "text");

    // Start generation
    store.getState().handleGenerationStarted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });

    // Request close — should show dialog because activeJob is set
    store.getState().requestCloseTab(tabId);
    expect(store.getState().pendingCloseTabId).toBe(tabId);
    expect(store.getState().tabs).toHaveLength(1); // not removed yet

    // Confirm close destroys the tab
    store.getState().confirmCloseTab();
    const state = store.getState();
    expect(state.tabs).toHaveLength(1); // new empty tab created
    expect(state.tabs[0]!.status).toBe("empty");
    expect(state.tabs[0]!.id).not.toBe(tabId);
  });

  // RC-5: Accept during new generation (version guard)
  it("RC-5: acceptProposal rejects when base version mismatches accepted version", () => {
    const tabId = store.getState().tabs[0]!.id;

    // Set up: proposal where base version doesn't match accepted version
    store.setState((state) => ({
      tabs: state.tabs.map((t) =>
        t.id === tabId
          ? {
              ...t,
              inputText: "input",
              acceptedOutput: "output v2",
              acceptedOutputVersion: 2,
              proposedOutput: "stale proposal",
              proposedOutputBaseVersion: 1, // mismatch!
              status: "proposal_ready" as const,
            }
          : t,
      ),
    }));

    store.getState().acceptProposal(tabId);

    // Should NOT accept — version guard prevents it
    expect(store.getState().tabs[0]!.acceptedOutput).toBe("output v2");
    expect(store.getState().tabs[0]!.acceptedOutputVersion).toBe(2);
    expect(store.getState().tabs[0]!.proposedOutput).toBe("stale proposal");
  });

  // RC-6: Worker crash mid-job
  it("RC-6: handleGenerationFailed with crash reason transitions to error", () => {
    const tabId = store.getState().tabs[0]!.id;
    store.getState().updateInputText(tabId, "text");

    // Start generation
    store.getState().handleGenerationStarted({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
    });

    // Worker crash → generation fails
    store.getState().handleGenerationFailed({
      contractVersion: 1,
      jobId: "job-1",
      tabId,
      requestKind: "initial_rewrite",
      error: {
        code: "WORKER_CRASHED",
        message: "Worker process exited unexpectedly",
        subsystem: "inference",
      },
    });

    const tab = store.getState().tabs[0]!;
    expect(tab.status).toBe("error");
    expect(tab.activeJob).toBeNull();
    expect(tab.error).not.toBeNull();
    expect(tab.error!.cause).toBe("Worker process exited unexpectedly");

    // Dismiss error should return to editing (has input text, no accepted output)
    store.getState().dismissError(tabId);
    expect(store.getState().tabs[0]!.status).toBe("editing");
  });

  // RC-7: Rapid tag toggle
  it("RC-7: rapid tag toggles produce correct final state", () => {
    const tabId = store.getState().tabs[0]!.id;

    // Toggle same tag 10 times rapidly
    for (let i = 0; i < 10; i++) {
      store.getState().toggleTag(tabId, "tag-rapid");
    }

    // Even number of toggles → tag should NOT be present
    expect(store.getState().tabs[0]!.activeTagIds).not.toContain("tag-rapid");

    // Toggle once more (11 = odd) → tag should be present
    store.getState().toggleTag(tabId, "tag-rapid");
    expect(store.getState().tabs[0]!.activeTagIds).toContain("tag-rapid");
  });

  it("RC-7: rapid toggles of different tags all settle correctly", () => {
    const tabId = store.getState().tabs[0]!.id;

    // Toggle 10 different tags
    for (let i = 0; i < 10; i++) {
      store.getState().toggleTag(tabId, `tag-${i}`);
    }

    expect(store.getState().tabs[0]!.activeTagIds).toHaveLength(10);

    // Toggle even-numbered tags off
    for (let i = 0; i < 10; i += 2) {
      store.getState().toggleTag(tabId, `tag-${i}`);
    }

    expect(store.getState().tabs[0]!.activeTagIds).toHaveLength(5);
    expect(store.getState().tabs[0]!.activeTagIds).toContain("tag-1");
    expect(store.getState().tabs[0]!.activeTagIds).not.toContain("tag-0");
  });
});

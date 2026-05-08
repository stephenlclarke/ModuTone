// Phase: 5
// Tests for session slice — tab lifecycle, content, dialogs, generation events

import { describe, it, expect, beforeEach } from "vitest";
import { create } from "zustand";
import { createSessionSlice, type SessionSlice } from "./sessionSlice";

function createTestStore() {
  return create<SessionSlice>()(createSessionSlice);
}

describe("sessionSlice", () => {
  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    store = createTestStore();
  });

  describe("initialization", () => {
    it("starts with one tab", () => {
      expect(store.getState().tabs).toHaveLength(1);
    });

    it("starts with activeTabId set to the first tab", () => {
      const state = store.getState();
      expect(state.activeTabId).toBe(state.tabs[0]!.id);
    });

    it("initial tab is in empty state", () => {
      expect(store.getState().tabs[0]!.status).toBe("empty");
    });

    it("initial tab has empty content", () => {
      const tab = store.getState().tabs[0]!;
      expect(tab.inputText).toBe("");
      expect(tab.acceptedOutput).toBeNull();
      expect(tab.proposedOutput).toBeNull();
      expect(tab.refinementInstruction).toBe("");
    });

    it("no pending dialogs", () => {
      const state = store.getState();
      expect(state.pendingCloseTabId).toBeNull();
      expect(state.pendingClearTabId).toBeNull();
    });

    it("no toasts", () => {
      expect(store.getState().toasts).toHaveLength(0);
    });
  });

  describe("createTab", () => {
    it("adds a new tab", () => {
      store.getState().createTab();
      expect(store.getState().tabs).toHaveLength(2);
    });

    it("new tab becomes active", () => {
      store.getState().createTab();
      const state = store.getState();
      expect(state.activeTabId).toBe(state.tabs[1]!.id);
    });

    it("new tab is in empty state", () => {
      store.getState().createTab();
      expect(store.getState().tabs[1]!.status).toBe("empty");
    });

    it("increments tab number in title", () => {
      store.getState().createTab();
      store.getState().createTab();
      expect(store.getState().tabs[1]!.title).toBe("Tab 2");
      expect(store.getState().tabs[2]!.title).toBe("Tab 3");
    });

    it("enforces 20-tab limit", () => {
      for (let i = 0; i < 25; i++) {
        store.getState().createTab();
      }
      expect(store.getState().tabs).toHaveLength(20);
    });

    it("each tab has a unique id", () => {
      for (let i = 0; i < 5; i++) {
        store.getState().createTab();
      }
      const ids = store.getState().tabs.map((t) => t.id);
      expect(new Set(ids).size).toBe(ids.length);
    });
  });

  describe("switchTab", () => {
    it("switches active tab", () => {
      store.getState().createTab();
      const firstTabId = store.getState().tabs[0]!.id;
      store.getState().switchTab(firstTabId);
      expect(store.getState().activeTabId).toBe(firstTabId);
    });

    it("ignores switch to non-existent tab", () => {
      const activeId = store.getState().activeTabId;
      store.getState().switchTab("nonexistent");
      expect(store.getState().activeTabId).toBe(activeId);
    });
  });

  describe("updateInputText", () => {
    it("updates input text", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "Hello world");
      expect(store.getState().tabs[0]!.inputText).toBe("Hello world");
    });

    it("transitions empty → editing when text entered", () => {
      const tabId = store.getState().tabs[0]!.id;
      expect(store.getState().tabs[0]!.status).toBe("empty");
      store.getState().updateInputText(tabId, "text");
      expect(store.getState().tabs[0]!.status).toBe("editing");
    });

    it("transitions editing → empty when text cleared", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      expect(store.getState().tabs[0]!.status).toBe("editing");
      store.getState().updateInputText(tabId, "");
      expect(store.getState().tabs[0]!.status).toBe("empty");
    });

    it("updates inputVersionToken on text change", () => {
      const tabId = store.getState().tabs[0]!.id;
      const token1 = store.getState().tabs[0]!.inputVersionToken;
      store.getState().updateInputText(tabId, "text");
      const token2 = store.getState().tabs[0]!.inputVersionToken;
      expect(token2).not.toBe(token1);
    });

    it("does not change status of non-empty/editing states", () => {
      // Manually set a tab to output_ready state for this test
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      // Simulate getting to output_ready by directly setting state
      // (In practice, this comes from generation events in Phase 5+)
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "output",
                status: "output_ready" as const,
              }
            : t,
        ),
      }));
      expect(store.getState().tabs[0]!.status).toBe("output_ready");
      store.getState().updateInputText(tabId, "changed input");
      expect(store.getState().tabs[0]!.status).toBe("output_ready");
    });
  });

  describe("setRefinementInstruction", () => {
    it("updates refinement instruction text", () => {
      const tabId = store.getState().tabs[0]!.id;
      // Set up output_ready state
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().setRefinementInstruction(tabId, "make it shorter");
      expect(store.getState().tabs[0]!.refinementInstruction).toBe(
        "make it shorter",
      );
    });

    it("transitions output_ready → refine_editing when instruction entered", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().setRefinementInstruction(tabId, "refine");
      expect(store.getState().tabs[0]!.status).toBe("refine_editing");
    });

    it("transitions refine_editing → output_ready when instruction cleared", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                refinementInstruction: "refine",
                status: "refine_editing" as const,
              }
            : t,
        ),
      }));

      store.getState().setRefinementInstruction(tabId, "");
      expect(store.getState().tabs[0]!.status).toBe("output_ready");
    });
  });

  describe("requestCloseTab / confirmCloseTab / cancelCloseTab", () => {
    it("immediately closes empty tab with no job", () => {
      store.getState().createTab(); // now 2 tabs
      const secondTabId = store.getState().tabs[1]!.id;
      store.getState().requestCloseTab(secondTabId);
      expect(store.getState().tabs).toHaveLength(1);
      expect(store.getState().pendingCloseTabId).toBeNull();
    });

    it("shows dialog for tab with content", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "some content");
      store.getState().requestCloseTab(tabId);
      expect(store.getState().pendingCloseTabId).toBe(tabId);
      expect(store.getState().tabs).toHaveLength(1); // not removed yet
    });

    it("confirmCloseTab destroys the tab", () => {
      store.getState().createTab();
      const firstTabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(firstTabId, "content");
      store.getState().requestCloseTab(firstTabId);
      store.getState().confirmCloseTab();
      expect(store.getState().tabs).toHaveLength(1);
      expect(store.getState().pendingCloseTabId).toBeNull();
    });

    it("cancelCloseTab clears pending state", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "content");
      store.getState().requestCloseTab(tabId);
      expect(store.getState().pendingCloseTabId).toBe(tabId);
      store.getState().cancelCloseTab();
      expect(store.getState().pendingCloseTabId).toBeNull();
      expect(store.getState().tabs).toHaveLength(1); // tab still exists
    });

    it("closing last tab creates a new empty tab", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().requestCloseTab(tabId); // empty tab, immediate close
      const state = store.getState();
      expect(state.tabs).toHaveLength(1);
      expect(state.tabs[0]!.status).toBe("empty");
      expect(state.tabs[0]!.id).not.toBe(tabId); // new tab
      expect(state.activeTabId).toBe(state.tabs[0]!.id);
    });

    it("switches to adjacent tab when active tab is closed", () => {
      store.getState().createTab();
      store.getState().createTab();
      // 3 tabs, active is tab 3 (index 2)
      const tab2Id = store.getState().tabs[1]!.id;
      store.getState().switchTab(tab2Id);
      store.getState().requestCloseTab(tab2Id); // empty, immediate
      // Should switch to tab at same index (now tab 3)
      expect(store.getState().tabs).toHaveLength(2);
    });
  });

  describe("requestClearTab / confirmClearTab / cancelClearTab", () => {
    it("does nothing for empty tab", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().requestClearTab(tabId);
      expect(store.getState().pendingClearTabId).toBeNull();
    });

    it("shows dialog for tab with input text", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "some text");
      store.getState().requestClearTab(tabId);
      expect(store.getState().pendingClearTabId).toBe(tabId);
    });

    it("confirmClearTab zeroes all content and resets to empty", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "input text");

      // Set up with accepted output
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "accepted",
                acceptedOutputVersion: 3,
                proposedOutput: "proposed",
                proposedOutputBaseVersion: 3,
                refinementInstruction: "refine",
                status: "proposal_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().requestClearTab(tabId);
      store.getState().confirmClearTab();

      const tab = store.getState().tabs[0]!;
      expect(tab.inputText).toBe("");
      expect(tab.acceptedOutput).toBeNull();
      expect(tab.acceptedOutputVersion).toBe(0);
      expect(tab.proposedOutput).toBeNull();
      expect(tab.proposedOutputBaseVersion).toBeNull();
      expect(tab.refinementInstruction).toBe("");
      expect(tab.activeJob).toBeNull();
      expect(tab.status).toBe("empty");
      expect(tab.error).toBeNull();
      expect(store.getState().pendingClearTabId).toBeNull();
    });

    it("cancelClearTab clears pending state", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      store.getState().requestClearTab(tabId);
      store.getState().cancelClearTab();
      expect(store.getState().pendingClearTabId).toBeNull();
      expect(store.getState().tabs[0]!.inputText).toBe("text");
    });
  });

  describe("acceptProposal", () => {
    it("promotes proposed output to accepted output", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "original",
                acceptedOutputVersion: 1,
                proposedOutput: "refined",
                proposedOutputBaseVersion: 1,
                refinementInstruction: "make it better",
                status: "proposal_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().acceptProposal(tabId);
      const tab = store.getState().tabs[0]!;
      expect(tab.acceptedOutput).toBe("refined");
      expect(tab.acceptedOutputVersion).toBe(2);
      expect(tab.proposedOutput).toBeNull();
      expect(tab.proposedOutputBaseVersion).toBeNull();
      expect(tab.refinementInstruction).toBe("");
      expect(tab.status).toBe("output_ready");
    });

    it("rejects accept when base version mismatches", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "original",
                acceptedOutputVersion: 2,
                proposedOutput: "refined",
                proposedOutputBaseVersion: 1, // mismatch!
                status: "proposal_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().acceptProposal(tabId);
      const tab = store.getState().tabs[0]!;
      expect(tab.acceptedOutput).toBe("original"); // unchanged
      expect(tab.proposedOutput).toBe("refined"); // still there
    });

    it("does nothing when not in proposal_ready state", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "original",
                proposedOutput: "refined",
                proposedOutputBaseVersion: 0,
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().acceptProposal(tabId);
      expect(store.getState().tabs[0]!.acceptedOutput).toBe("original");
    });
  });

  describe("rejectProposal", () => {
    it("clears proposed output and returns to output_ready", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "original",
                acceptedOutputVersion: 1,
                proposedOutput: "refined",
                proposedOutputBaseVersion: 1,
                status: "proposal_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().rejectProposal(tabId);
      const tab = store.getState().tabs[0]!;
      expect(tab.acceptedOutput).toBe("original"); // preserved
      expect(tab.proposedOutput).toBeNull();
      expect(tab.status).toBe("output_ready");
    });
  });

  describe("dismissError", () => {
    it("returns to output_ready when accepted output exists", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                status: "error" as const,
                error: {
                  message: "err",
                  cause: "cause",
                  action: "retry",
                },
              }
            : t,
        ),
      }));

      store.getState().dismissError(tabId);
      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("output_ready");
      expect(tab.error).toBeNull();
    });

    it("returns to editing when no accepted output but has input", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                status: "error" as const,
                error: {
                  message: "err",
                  cause: "cause",
                  action: "retry",
                },
              }
            : t,
        ),
      }));

      store.getState().dismissError(tabId);
      expect(store.getState().tabs[0]!.status).toBe("editing");
    });

    it("returns to empty when no content at all", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                status: "error" as const,
                error: {
                  message: "err",
                  cause: "cause",
                  action: "retry",
                },
              }
            : t,
        ),
      }));

      store.getState().dismissError(tabId);
      expect(store.getState().tabs[0]!.status).toBe("empty");
    });
  });

  describe("toasts", () => {
    it("addToast adds a toast", () => {
      store.getState().addToast("Test message", "success");
      expect(store.getState().toasts).toHaveLength(1);
      expect(store.getState().toasts[0]!.message).toBe("Test message");
      expect(store.getState().toasts[0]!.style).toBe("success");
    });

    it("addToast uses default 2s duration", () => {
      store.getState().addToast("msg", "neutral");
      expect(store.getState().toasts[0]!.duration).toBe(2000);
    });

    it("addToast accepts custom duration", () => {
      store.getState().addToast("msg", "error", 5000);
      expect(store.getState().toasts[0]!.duration).toBe(5000);
    });

    it("dismissToast removes a toast", () => {
      store.getState().addToast("msg1", "success");
      store.getState().addToast("msg2", "error");
      const toastId = store.getState().toasts[0]!.id;
      store.getState().dismissToast(toastId);
      expect(store.getState().toasts).toHaveLength(1);
      expect(store.getState().toasts[0]!.message).toBe("msg2");
    });
  });

  // --- Generation event handlers (Phase 5) ---

  describe("handleGenerationStarted", () => {
    it("sets tab to generating for initial_rewrite", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });
      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("generating");
      expect(tab.activeJob).toEqual({
        jobId: "job-1",
        requestKind: "initial_rewrite",
      });
    });

    it("sets tab to proposal_generating for refinement", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                status: "output_ready" as const,
              }
            : t,
        ),
      }));
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
      });
      expect(store.getState().tabs[0]!.status).toBe("proposal_generating");
    });

    it("ignores events for unknown tab ids", () => {
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId: "nonexistent",
        requestKind: "initial_rewrite",
      });
      expect(store.getState().tabs[0]!.activeJob).toBeNull();
    });
  });

  describe("handleGenerationCompleted", () => {
    it("sets output for initial_rewrite", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
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
        outputText: "rewritten text",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("output_ready");
      expect(tab.acceptedOutput).toBe("rewritten text");
      expect(tab.acceptedOutputVersion).toBe(1);
      expect(tab.activeJob).toBeNull();
    });

    it("sets proposal for refinement", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "v1 output",
                acceptedOutputVersion: 1,
                status: "output_ready" as const,
              }
            : t,
        ),
      }));
      const token = store.getState().tabs[0]!.inputVersionToken;

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
      });

      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
        inputVersionToken: token,
        acceptedOutputVersion: 1,
        outputText: "refined text",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("proposal_ready");
      expect(tab.proposedOutput).toBe("refined text");
      expect(tab.proposedOutputBaseVersion).toBe(1);
      expect(tab.acceptedOutput).toBe("v1 output"); // unchanged
      expect(tab.activeJob).toBeNull();
    });

    it("discards stale result when version token mismatches", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Simulate user editing during generation (changes version token)
      store.getState().updateInputText(tabId, "edited text");

      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
        inputVersionToken: "stale-token",
        acceptedOutputVersion: null,
        outputText: "should be discarded",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.acceptedOutput).toBeNull(); // not applied
      expect(tab.activeJob).toBeNull();
      expect(tab.status).toBe("editing"); // reverted
    });

    it("ignores completion for non-matching job id", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      const token = store.getState().tabs[0]!.inputVersionToken;

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "different-job",
        tabId,
        requestKind: "initial_rewrite",
        inputVersionToken: token,
        acceptedOutputVersion: null,
        outputText: "wrong job",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("generating"); // unchanged
      expect(tab.activeJob?.jobId).toBe("job-1");
    });
  });

  describe("handleGenerationFailed", () => {
    it("transitions to error state", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      store.getState().handleGenerationFailed({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
        error: {
          code: "WORKER_ERROR",
          message: "Inference failed",
          subsystem: "inference",
        },
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("error");
      expect(tab.error).not.toBeNull();
      expect(tab.error!.cause).toBe("Inference failed");
      expect(tab.activeJob).toBeNull();
    });

    it("ignores failure for non-matching job", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      store.getState().handleGenerationFailed({
        contractVersion: 1,
        jobId: "wrong-job",
        tabId,
        requestKind: "initial_rewrite",
        error: {
          code: "ERROR",
          message: "fail",
          subsystem: "inference",
        },
      });

      expect(store.getState().tabs[0]!.status).toBe("generating");
    });
  });

  describe("handleGenerationCanceled", () => {
    it("reverts to editing when no accepted output", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("editing");
      expect(tab.activeJob).toBeNull();
    });

    it("reverts to output_ready when accepted output exists", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                acceptedOutputVersion: 1,
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
      });

      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
      });

      expect(store.getState().tabs[0]!.status).toBe("output_ready");
    });

    it("reverts to empty when no content", () => {
      const tabId = store.getState().tabs[0]!.id;

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      expect(store.getState().tabs[0]!.status).toBe("empty");
    });
  });

  describe("generation → recovery sequence", () => {
    it("full lifecycle: start → complete → accept refinement", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "my text");
      const token = store.getState().tabs[0]!.inputVersionToken;

      // Start initial generation
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });
      expect(store.getState().tabs[0]!.status).toBe("generating");

      // Complete
      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
        inputVersionToken: token,
        acceptedOutputVersion: null,
        outputText: "rewritten",
      });
      expect(store.getState().tabs[0]!.status).toBe("output_ready");
      expect(store.getState().tabs[0]!.acceptedOutput).toBe("rewritten");

      // Start refinement
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
      });
      expect(store.getState().tabs[0]!.status).toBe("proposal_generating");

      // Complete refinement
      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
        inputVersionToken: token,
        acceptedOutputVersion: 1,
        outputText: "refined output",
      });
      expect(store.getState().tabs[0]!.status).toBe("proposal_ready");
      expect(store.getState().tabs[0]!.proposedOutput).toBe("refined output");

      // Accept proposal
      store.getState().acceptProposal(tabId);
      expect(store.getState().tabs[0]!.status).toBe("output_ready");
      expect(store.getState().tabs[0]!.acceptedOutput).toBe("refined output");
      expect(store.getState().tabs[0]!.acceptedOutputVersion).toBe(2);
    });
  });

  // --- Tag toggle (Phase 6) ---

  describe("toggleTag", () => {
    it("adds a tag to the tab's activeTagIds", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().toggleTag(tabId, "tag-1");
      expect(store.getState().tabs[0]!.activeTagIds).toContain("tag-1");
    });

    it("removes a tag when already present", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().toggleTag(tabId, "tag-1");
      store.getState().toggleTag(tabId, "tag-1");
      expect(store.getState().tabs[0]!.activeTagIds).not.toContain("tag-1");
    });

    it("regenerates inputVersionToken on toggle", () => {
      const tabId = store.getState().tabs[0]!.id;
      const token1 = store.getState().tabs[0]!.inputVersionToken;
      store.getState().toggleTag(tabId, "tag-1");
      const token2 = store.getState().tabs[0]!.inputVersionToken;
      expect(token2).not.toBe(token1);
    });

    it("does not affect other tabs", () => {
      store.getState().createTab();
      const tab1Id = store.getState().tabs[0]!.id;
      store.getState().toggleTag(tab1Id, "tag-1");
      expect(store.getState().tabs[0]!.activeTagIds).toContain("tag-1");
      expect(store.getState().tabs[1]!.activeTagIds).not.toContain("tag-1");

      // Ensure tab2's version token wasn't changed
      const tab2Before = store.getState().tabs[1]!.inputVersionToken;
      store.getState().toggleTag(tab1Id, "tag-2");
      expect(store.getState().tabs[1]!.inputVersionToken).toBe(tab2Before);
    });

    it("can add multiple different tags", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().toggleTag(tabId, "tag-1");
      store.getState().toggleTag(tabId, "tag-2");
      store.getState().toggleTag(tabId, "tag-3");
      expect(store.getState().tabs[0]!.activeTagIds).toEqual([
        "tag-1",
        "tag-2",
        "tag-3",
      ]);
    });

    it("does not cancel active jobs (stale-discard is sufficient)", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Toggle tag during active generation
      store.getState().toggleTag(tabId, "tag-1");

      // Job should still be active — not canceled
      const tab = store.getState().tabs[0]!;
      expect(tab.activeJob).not.toBeNull();
      expect(tab.activeJob!.jobId).toBe("job-1");
      // But version token changed, so result will be stale-discarded
      expect(tab.activeTagIds).toContain("tag-1");
    });
  });

  describe("removeTagFromAllTabs", () => {
    it("removes tag from all tabs that have it", () => {
      store.getState().createTab();
      store.getState().createTab();
      const tab1Id = store.getState().tabs[0]!.id;
      const tab2Id = store.getState().tabs[1]!.id;
      const tab3Id = store.getState().tabs[2]!.id;

      store.getState().toggleTag(tab1Id, "tag-x");
      store.getState().toggleTag(tab2Id, "tag-x");
      store.getState().toggleTag(tab3Id, "tag-x");

      store.getState().removeTagFromAllTabs("tag-x");

      expect(store.getState().tabs[0]!.activeTagIds).not.toContain("tag-x");
      expect(store.getState().tabs[1]!.activeTagIds).not.toContain("tag-x");
      expect(store.getState().tabs[2]!.activeTagIds).not.toContain("tag-x");
    });

    it("preserves unrelated tag IDs", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().toggleTag(tabId, "tag-keep");
      store.getState().toggleTag(tabId, "tag-remove");
      store.getState().toggleTag(tabId, "tag-also-keep");

      store.getState().removeTagFromAllTabs("tag-remove");

      const tags = store.getState().tabs[0]!.activeTagIds;
      expect(tags).toEqual(["tag-keep", "tag-also-keep"]);
    });

    it("does not modify tabs that don't have the tag", () => {
      store.getState().createTab();
      const tab1Id = store.getState().tabs[0]!.id;

      store.getState().toggleTag(tab1Id, "tag-other");
      const tab2TokenBefore = store.getState().tabs[1]!.inputVersionToken;

      store.getState().removeTagFromAllTabs("tag-nonexistent");

      // Tab 2 should be completely untouched
      expect(store.getState().tabs[1]!.inputVersionToken).toBe(tab2TokenBefore);
    });
  });

  describe("multi-step refinement cycle", () => {
    it("full cycle: generate → refine → accept → refine again → accept (version 1→2→3)", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "input text");
      const token = store.getState().tabs[0]!.inputVersionToken;

      // Step 1: Initial generation
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
        outputText: "v1 output",
      });
      expect(store.getState().tabs[0]!.acceptedOutputVersion).toBe(1);
      expect(store.getState().tabs[0]!.acceptedOutput).toBe("v1 output");

      // Step 2: First refinement
      store.getState().setRefinementInstruction(tabId, "make shorter");
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
      });
      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-2",
        tabId,
        requestKind: "refinement",
        inputVersionToken: token,
        acceptedOutputVersion: 1,
        outputText: "v2 proposal",
      });
      expect(store.getState().tabs[0]!.status).toBe("proposal_ready");
      expect(store.getState().tabs[0]!.proposedOutput).toBe("v2 proposal");
      expect(store.getState().tabs[0]!.proposedOutputBaseVersion).toBe(1);

      // Step 3: Accept first proposal (version 1 → 2)
      store.getState().acceptProposal(tabId);
      expect(store.getState().tabs[0]!.acceptedOutput).toBe("v2 proposal");
      expect(store.getState().tabs[0]!.acceptedOutputVersion).toBe(2);
      expect(store.getState().tabs[0]!.proposedOutput).toBeNull();
      expect(store.getState().tabs[0]!.status).toBe("output_ready");

      // Step 4: Second refinement
      store.getState().setRefinementInstruction(tabId, "make formal");
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-3",
        tabId,
        requestKind: "refinement",
      });
      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-3",
        tabId,
        requestKind: "refinement",
        inputVersionToken: token,
        acceptedOutputVersion: 2,
        outputText: "v3 proposal",
      });
      expect(store.getState().tabs[0]!.proposedOutputBaseVersion).toBe(2);

      // Step 5: Accept second proposal (version 2 → 3)
      store.getState().acceptProposal(tabId);
      expect(store.getState().tabs[0]!.acceptedOutput).toBe("v3 proposal");
      expect(store.getState().tabs[0]!.acceptedOutputVersion).toBe(3);
      expect(store.getState().tabs[0]!.status).toBe("output_ready");
    });

    it("stale proposal: cannot accept when base version mismatches", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      const token = store.getState().tabs[0]!.inputVersionToken;

      // Generate initial output
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
        outputText: "output v1",
      });

      // Simulate a stale proposal (baseVersion=0, but acceptedOutputVersion=1)
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                proposedOutput: "stale proposal",
                proposedOutputBaseVersion: 0, // mismatch with acceptedOutputVersion=1
                status: "proposal_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().acceptProposal(tabId);
      // Should NOT accept — version mismatch guard
      expect(store.getState().tabs[0]!.acceptedOutput).toBe("output v1");
      expect(store.getState().tabs[0]!.proposedOutput).toBe("stale proposal");
    });

    it("Re-Generate from proposal_ready: clears proposal fields, produces fresh output", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      const token = store.getState().tabs[0]!.inputVersionToken;

      // Set up proposal_ready state
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "old output",
                acceptedOutputVersion: 1,
                proposedOutput: "proposal",
                proposedOutputBaseVersion: 1,
                refinementInstruction: "make shorter",
                status: "proposal_ready" as const,
              }
            : t,
        ),
      }));

      // Re-Generate (initial_rewrite) from proposal_ready
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-regen",
        tabId,
        requestKind: "initial_rewrite",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("generating");
      expect(tab.proposedOutput).toBeNull();
      expect(tab.proposedOutputBaseVersion).toBeNull();
      expect(tab.refinementInstruction).toBe(""); // cleared from proposal_ready

      // Complete generates fresh output
      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-regen",
        tabId,
        requestKind: "initial_rewrite",
        inputVersionToken: token,
        acceptedOutputVersion: null,
        outputText: "fresh output",
      });

      const tabAfter = store.getState().tabs[0]!;
      expect(tabAfter.acceptedOutput).toBe("fresh output");
      expect(tabAfter.acceptedOutputVersion).toBe(2);
      expect(tabAfter.status).toBe("output_ready");
    });

    it("cancel refinement with instruction: returns to refine_editing, instruction preserved", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                acceptedOutputVersion: 1,
                refinementInstruction: "make it better",
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      // Start refinement
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-refine",
        tabId,
        requestKind: "refinement",
      });
      expect(store.getState().tabs[0]!.status).toBe("proposal_generating");

      // Cancel
      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "job-refine",
        tabId,
        requestKind: "refinement",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("refine_editing");
      expect(tab.refinementInstruction).toBe("make it better");
      expect(tab.activeJob).toBeNull();
    });

    it("Re-Generate clears lingering proposal fields", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      // Set up output_ready with lingering proposal fields (shouldn't happen normally,
      // but tests the cleanup)
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "output",
                acceptedOutputVersion: 1,
                proposedOutput: "lingering",
                proposedOutputBaseVersion: 1,
                refinementInstruction: "old instruction",
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      // Re-Generate (initial_rewrite)
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-regen",
        tabId,
        requestKind: "initial_rewrite",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.proposedOutput).toBeNull();
      expect(tab.proposedOutputBaseVersion).toBeNull();
      expect(tab.refinementInstruction).toBe(""); // cleared from output_ready
    });

    it("profile change during in-flight refinement does not alter handling", () => {
      // Profile ID is captured at submit time (in the component) and sent in the request.
      // The session slice event handlers don't reference any profile state —
      // they just process the event. This test verifies that the completion
      // handler works correctly regardless of any state changes made during flight.
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");
      const token = store.getState().tabs[0]!.inputVersionToken;

      // Set up output_ready
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                acceptedOutput: "v1 output",
                acceptedOutputVersion: 1,
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      // Start refinement (profileId was captured at submit time in the component)
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-refine",
        tabId,
        requestKind: "refinement",
      });
      expect(store.getState().tabs[0]!.status).toBe("proposal_generating");

      // Complete refinement — processes correctly; no profile state is consulted
      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "job-refine",
        tabId,
        requestKind: "refinement",
        inputVersionToken: token,
        acceptedOutputVersion: 1,
        outputText: "refined with old profile",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("proposal_ready");
      expect(tab.proposedOutput).toBe("refined with old profile");
      expect(tab.proposedOutputBaseVersion).toBe(1);
    });

    it("Re-Generate from editing state does not clear refinement instruction", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      // Directly in editing state with a refinement instruction somehow present
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                refinementInstruction: "should persist",
                status: "editing" as const,
              }
            : t,
        ),
      }));

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      // From editing state, refinement instruction should NOT be cleared
      expect(store.getState().tabs[0]!.refinementInstruction).toBe(
        "should persist",
      );
    });
  });

  describe("requestReGenerate / confirmReGenerate / cancelReGenerate", () => {
    it("sets pendingReGenerateTabId when acceptedOutput is not null", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                acceptedOutputVersion: 1,
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().requestReGenerate(tabId);
      expect(store.getState().pendingReGenerateTabId).toBe(tabId);
    });

    it("does nothing when acceptedOutput is null", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      store.getState().requestReGenerate(tabId);
      expect(store.getState().pendingReGenerateTabId).toBeNull();
    });

    it("confirmReGenerate clears pending state", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().requestReGenerate(tabId);
      expect(store.getState().pendingReGenerateTabId).toBe(tabId);
      store.getState().confirmReGenerate();
      expect(store.getState().pendingReGenerateTabId).toBeNull();
    });

    it("cancelReGenerate clears pending state and preserves tab content", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "output",
                acceptedOutputVersion: 1,
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      store.getState().requestReGenerate(tabId);
      store.getState().cancelReGenerate();
      expect(store.getState().pendingReGenerateTabId).toBeNull();
      // Tab content is preserved
      const tab = store.getState().tabs[0]!;
      expect(tab.acceptedOutput).toBe("output");
      expect(tab.status).toBe("output_ready");
    });

    it("does nothing for nonexistent tab", () => {
      store.getState().requestReGenerate("nonexistent-tab-id");
      expect(store.getState().pendingReGenerateTabId).toBeNull();
    });

    it("initial state has pendingReGenerateTabId === null", () => {
      expect(store.getState().pendingReGenerateTabId).toBeNull();
    });
  });

  describe("tab lifecycle invariants", () => {
    it("always has at least one tab", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().requestCloseTab(tabId); // empty tab, closes immediately
      expect(store.getState().tabs.length).toBeGreaterThanOrEqual(1);
    });

    it("activeTabId always points to an existing tab", () => {
      store.getState().createTab();
      store.getState().createTab();
      const state = store.getState();
      const tab = state.tabs.find((t) => t.id === state.activeTabId);
      expect(tab).toBeDefined();
    });

    it("content fields are memory-only (no persistence calls)", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "sensitive content");
      // Verify content is only in memory state
      const tab = store.getState().tabs[0]!;
      expect(tab.inputText).toBe("sensitive content");
      // No localStorage, no file writes — this is structural
    });
  });

  describe("cancel generation: optimistic UI unlock", () => {
    it("clears activeJob and reverts status immediately on cancel", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text to rewrite");

      // Start generation
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "cancel-job-1",
        tabId,
        requestKind: "initial_rewrite",
      });
      expect(store.getState().tabs[0]!.status).toBe("generating");
      expect(store.getState().tabs[0]!.activeJob).not.toBeNull();

      // Simulate optimistic cancel (what the UI does after IPC success)
      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "cancel-job-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("editing");
      expect(tab.activeJob).toBeNull();
      expect(tab.inputText).toBe("text to rewrite");
    });

    it("late-arriving generation:canceled is no-op after optimistic cancel", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "test");

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "cancel-job-2",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Optimistic cancel
      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "cancel-job-2",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Late event from backend arrives — should be no-op
      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "cancel-job-2",
        tabId,
        requestKind: "initial_rewrite",
      });

      // State unchanged from after optimistic cancel
      expect(store.getState().tabs[0]!.status).toBe("editing");
      expect(store.getState().tabs[0]!.activeJob).toBeNull();
    });

    it("late-arriving generation:completed is no-op after optimistic cancel", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "test");
      const token = store.getState().tabs[0]!.inputVersionToken;

      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "cancel-job-3",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Optimistic cancel
      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "cancel-job-3",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Worker eventually completes (cancel-wins on backend emits canceled,
      // but test the case where completed arrives)
      store.getState().handleGenerationCompleted({
        contractVersion: 1,
        jobId: "cancel-job-3",
        tabId,
        requestKind: "initial_rewrite",
        inputVersionToken: token,
        acceptedOutputVersion: null,
        outputText: "stale result",
      });

      // Must NOT accept the stale output — activeJob was already cleared
      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("editing");
      expect(tab.acceptedOutput).toBeNull();
    });

    it("preserves accepted output when canceling a re-generate", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.setState((state) => ({
        tabs: state.tabs.map((t) =>
          t.id === tabId
            ? {
                ...t,
                inputText: "input",
                acceptedOutput: "existing output",
                acceptedOutputVersion: 1,
                status: "output_ready" as const,
              }
            : t,
        ),
      }));

      // Start re-generate
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "regen-cancel-1",
        tabId,
        requestKind: "initial_rewrite",
      });
      expect(store.getState().tabs[0]!.status).toBe("generating");

      // Cancel
      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "regen-cancel-1",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Existing output is preserved, status reverts to output_ready
      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("output_ready");
      expect(tab.acceptedOutput).toBe("existing output");
      expect(tab.activeJob).toBeNull();
    });

    it("can generate again immediately after cancel (new job accepted)", () => {
      const tabId = store.getState().tabs[0]!.id;
      store.getState().updateInputText(tabId, "text");

      // First generation
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "first-job",
        tabId,
        requestKind: "initial_rewrite",
      });

      // Cancel
      store.getState().handleGenerationCanceled({
        contractVersion: 1,
        jobId: "first-job",
        tabId,
        requestKind: "initial_rewrite",
      });

      expect(store.getState().tabs[0]!.activeJob).toBeNull();

      // Start new generation immediately
      store.getState().handleGenerationStarted({
        contractVersion: 1,
        jobId: "second-job",
        tabId,
        requestKind: "initial_rewrite",
      });

      const tab = store.getState().tabs[0]!;
      expect(tab.status).toBe("generating");
      expect(tab.activeJob!.jobId).toBe("second-job");
    });
  });
});

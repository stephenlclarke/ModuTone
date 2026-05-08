// Phase: 5
// Session slice — tabs, content, generation events, toasts (ephemeral, memory-only)

import type { StateCreator } from "zustand";
import type { SessionTab, TabStatus, ToastMessage } from "./types";
import type {
  GenerationStartedEvent,
  GenerationCompletedEvent,
  GenerationFailedEvent,
  GenerationCanceledEvent,
} from "../../ipc/types";
import { generateVersionToken } from "../../utils/versionToken";
import { v4 as uuidv4 } from "uuid";

const MAX_TABS = 20;

export interface SessionSlice {
  // State
  tabs: SessionTab[];
  activeTabId: string | null;
  nextTabNumber: number;
  pendingCloseTabId: string | null;
  pendingClearTabId: string | null;
  pendingReGenerateTabId: string | null;
  toasts: ToastMessage[];

  // Tab lifecycle
  createTab: () => void;
  switchTab: (tabId: string) => void;
  requestCloseTab: (tabId: string) => void;
  confirmCloseTab: () => void;
  cancelCloseTab: () => void;

  // Tab rename
  renameTab: (tabId: string, newTitle: string) => void;

  // Content
  updateInputText: (tabId: string, text: string) => void;
  setRefinementInstruction: (tabId: string, text: string) => void;

  // Clear
  requestClearTab: (tabId: string) => void;
  confirmClearTab: () => void;
  cancelClearTab: () => void;

  // Re-Generate confirmation
  requestReGenerate: (tabId: string) => void;
  confirmReGenerate: () => void;
  cancelReGenerate: () => void;

  // Proposals (state machine support — triggered in Phase 7)
  acceptProposal: (tabId: string) => void;
  rejectProposal: (tabId: string) => void;

  // Error (state machine support — triggered in Phase 5+)
  dismissError: (tabId: string) => void;

  // Toast
  addToast: (
    message: string,
    style: ToastMessage["style"],
    duration?: number,
  ) => void;
  dismissToast: (toastId: string) => void;

  // Generation events (driven by backend event listeners)
  handleGenerationStarted: (event: GenerationStartedEvent) => void;
  handleGenerationCompleted: (event: GenerationCompletedEvent) => void;
  handleGenerationFailed: (event: GenerationFailedEvent) => void;
  handleGenerationCanceled: (event: GenerationCanceledEvent) => void;

  // Tags
  toggleTag: (tabId: string, tagId: string) => void;
  removeTagFromAllTabs: (tagId: string) => void;

  // Clipboard
  copyAcceptedOutput: (tabId: string) => Promise<void>;
}

function createEmptyTab(tabNumber: number): SessionTab {
  return {
    id: uuidv4(),
    title: `Tab ${String(tabNumber)}`,
    inputText: "",
    activeTagIds: [],
    acceptedOutput: null,
    acceptedOutputVersion: 0,
    proposedOutput: null,
    proposedOutputBaseVersion: null,
    refinementInstruction: "",
    activeJob: null,
    status: "empty",
    error: null,
    inputVersionToken: generateVersionToken(),
  };
}

function tabHasContent(tab: SessionTab): boolean {
  return (
    tab.inputText.length > 0 ||
    tab.acceptedOutput !== null ||
    tab.proposedOutput !== null
  );
}

/**
 * Compute the state changes needed to destroy a tab.
 * If it's the last tab, creates a new empty tab.
 */
function computeTabDestruction(
  state: SessionSlice,
  tabId: string,
): Partial<SessionSlice> {
  const tabIndex = state.tabs.findIndex((t) => t.id === tabId);
  if (tabIndex === -1) return {};

  const newTabs = state.tabs.filter((t) => t.id !== tabId);

  if (newTabs.length === 0) {
    const newTab = createEmptyTab(state.nextTabNumber);
    return {
      tabs: [newTab],
      activeTabId: newTab.id,
      nextTabNumber: state.nextTabNumber + 1,
    };
  }

  let newActiveTabId = state.activeTabId;
  if (state.activeTabId === tabId) {
    const newIndex = Math.min(tabIndex, newTabs.length - 1);
    const targetTab = newTabs[newIndex];
    if (targetTab) {
      newActiveTabId = targetTab.id;
    }
  }

  return {
    tabs: newTabs,
    activeTabId: newActiveTabId,
  };
}

export const createSessionSlice: StateCreator<SessionSlice> = (set, get) => {
  const initialTab = createEmptyTab(1);

  return {
    tabs: [initialTab],
    activeTabId: initialTab.id,
    nextTabNumber: 2,
    pendingCloseTabId: null,
    pendingClearTabId: null,
    pendingReGenerateTabId: null,
    toasts: [],

    createTab: () => {
      const state = get();
      if (state.tabs.length >= MAX_TABS) return;

      const tab = createEmptyTab(state.nextTabNumber);
      set({
        tabs: [...state.tabs, tab],
        activeTabId: tab.id,
        nextTabNumber: state.nextTabNumber + 1,
      });
    },

    renameTab: (tabId: string, newTitle: string) => {
      const trimmed = newTitle.trim();
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;
          return {
            ...tab,
            title: trimmed || tab.title,
          };
        }),
      }));
    },

    switchTab: (tabId: string) => {
      const state = get();
      if (state.tabs.some((t) => t.id === tabId)) {
        set({ activeTabId: tabId });
      }
    },

    requestCloseTab: (tabId: string) => {
      const state = get();
      const tab = state.tabs.find((t) => t.id === tabId);
      if (!tab) return;

      if (tabHasContent(tab) || tab.activeJob !== null) {
        set({ pendingCloseTabId: tabId });
      } else {
        set(computeTabDestruction(state, tabId));
      }
    },

    confirmCloseTab: () => {
      const state = get();
      const tabId = state.pendingCloseTabId;
      if (!tabId) return;

      set({
        ...computeTabDestruction(state, tabId),
        pendingCloseTabId: null,
      });
    },

    cancelCloseTab: () => {
      set({ pendingCloseTabId: null });
    },

    updateInputText: (tabId: string, text: string) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;

          let newStatus: TabStatus = tab.status;
          if (tab.status === "empty" && text.length > 0) {
            newStatus = "editing";
          } else if (tab.status === "editing" && text.length === 0) {
            newStatus = "empty";
          }

          return {
            ...tab,
            inputText: text,
            status: newStatus,
            inputVersionToken: generateVersionToken(),
          };
        }),
      }));
    },

    setRefinementInstruction: (tabId: string, text: string) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;

          let newStatus: TabStatus = tab.status;
          if (tab.status === "output_ready" && text.length > 0) {
            newStatus = "refine_editing";
          } else if (tab.status === "refine_editing" && text.length === 0) {
            newStatus = "output_ready";
          }

          return {
            ...tab,
            refinementInstruction: text,
            status: newStatus,
          };
        }),
      }));
    },

    requestClearTab: (tabId: string) => {
      const state = get();
      const tab = state.tabs.find((t) => t.id === tabId);
      if (!tab || !tabHasContent(tab)) return;

      set({ pendingClearTabId: tabId });
    },

    confirmClearTab: () => {
      const state = get();
      const tabId = state.pendingClearTabId;
      if (!tabId) return;

      set({
        pendingClearTabId: null,
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;
          return {
            ...tab,
            inputText: "",
            acceptedOutput: null,
            acceptedOutputVersion: 0,
            proposedOutput: null,
            proposedOutputBaseVersion: null,
            refinementInstruction: "",
            activeJob: null,
            status: "empty" as TabStatus,
            error: null,
            inputVersionToken: generateVersionToken(),
          };
        }),
      });
    },

    cancelClearTab: () => {
      set({ pendingClearTabId: null });
    },

    requestReGenerate: (tabId: string) => {
      const state = get();
      const tab = state.tabs.find((t) => t.id === tabId);
      if (!tab) return;
      // Only prompt confirmation when there is accepted output to overwrite
      if (tab.acceptedOutput === null) return;
      set({ pendingReGenerateTabId: tabId });
    },

    confirmReGenerate: () => {
      set({ pendingReGenerateTabId: null });
    },

    cancelReGenerate: () => {
      set({ pendingReGenerateTabId: null });
    },

    acceptProposal: (tabId: string) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;
          if (tab.status !== "proposal_ready" || tab.proposedOutput === null)
            return tab;
          if (tab.proposedOutputBaseVersion !== tab.acceptedOutputVersion)
            return tab;

          return {
            ...tab,
            acceptedOutput: tab.proposedOutput,
            acceptedOutputVersion: tab.acceptedOutputVersion + 1,
            proposedOutput: null,
            proposedOutputBaseVersion: null,
            refinementInstruction: "",
            status: "output_ready" as TabStatus,
          };
        }),
      }));
    },

    rejectProposal: (tabId: string) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;
          if (tab.status !== "proposal_ready") return tab;

          return {
            ...tab,
            proposedOutput: null,
            proposedOutputBaseVersion: null,
            status: "output_ready" as TabStatus,
          };
        }),
      }));
    },

    dismissError: (tabId: string) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;
          if (tab.status !== "error") return tab;

          let newStatus: TabStatus;
          if (tab.acceptedOutput !== null) {
            newStatus = "output_ready";
          } else if (tab.inputText.length > 0) {
            newStatus = "editing";
          } else {
            newStatus = "empty";
          }

          return {
            ...tab,
            status: newStatus,
            error: null,
          };
        }),
      }));
    },

    addToast: (
      message: string,
      style: ToastMessage["style"],
      duration = 2000,
    ) => {
      const toast: ToastMessage = {
        id: uuidv4(),
        message,
        style,
        duration,
      };
      set((state) => ({
        toasts: [...state.toasts, toast],
      }));
    },

    dismissToast: (toastId: string) => {
      set((state) => ({
        toasts: state.toasts.filter((t) => t.id !== toastId),
      }));
    },

    // --- Generation event handlers ---

    handleGenerationStarted: (event: GenerationStartedEvent) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== event.tabId) return tab;

          if (event.requestKind === "initial_rewrite") {
            const clearInstruction =
              tab.status === "output_ready" ||
              tab.status === "refine_editing" ||
              tab.status === "proposal_ready" ||
              tab.status === "proposal_generating";

            return {
              ...tab,
              activeJob: {
                jobId: event.jobId,
                requestKind: event.requestKind,
              },
              proposedOutput: null,
              proposedOutputBaseVersion: null,
              refinementInstruction: clearInstruction
                ? ""
                : tab.refinementInstruction,
              status: "generating" as TabStatus,
            };
          }

          return {
            ...tab,
            activeJob: {
              jobId: event.jobId,
              requestKind: event.requestKind,
            },
            status: "proposal_generating" as TabStatus,
          };
        }),
      }));
    },

    handleGenerationCompleted: (event: GenerationCompletedEvent) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== event.tabId) return tab;
          // Must match active job
          if (!tab.activeJob || tab.activeJob.jobId !== event.jobId) return tab;

          // Stale version token check (P5-6)
          if (tab.inputVersionToken !== event.inputVersionToken) {
            // Result is for an older version of the input — discard
            return {
              ...tab,
              activeJob: null,
              status: (tab.inputText.length > 0
                ? "editing"
                : "empty") as TabStatus,
            };
          }

          if (event.requestKind === "initial_rewrite") {
            return {
              ...tab,
              activeJob: null,
              acceptedOutput: event.outputText,
              acceptedOutputVersion: tab.acceptedOutputVersion + 1,
              status: "output_ready" as TabStatus,
            };
          } else {
            // Refinement → proposal
            return {
              ...tab,
              activeJob: null,
              proposedOutput: event.outputText,
              proposedOutputBaseVersion: tab.acceptedOutputVersion,
              status: "proposal_ready" as TabStatus,
            };
          }
        }),
      }));
    },

    handleGenerationFailed: (event: GenerationFailedEvent) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== event.tabId) return tab;
          if (!tab.activeJob || tab.activeJob.jobId !== event.jobId) return tab;
          return {
            ...tab,
            activeJob: null,
            status: "error" as TabStatus,
            error: {
              message: event.error.message,
              cause: event.error.detail ?? event.error.message,
              action: "Try again or modify your input",
              source: event.error,
            },
          };
        }),
      }));
    },

    handleGenerationCanceled: (event: GenerationCanceledEvent) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== event.tabId) return tab;
          if (!tab.activeJob || tab.activeJob.jobId !== event.jobId) return tab;

          // Revert to pre-generation status
          let newStatus: TabStatus;
          if (tab.acceptedOutput !== null) {
            newStatus =
              tab.refinementInstruction.length > 0
                ? "refine_editing"
                : "output_ready";
          } else if (tab.inputText.length > 0) {
            newStatus = "editing";
          } else {
            newStatus = "empty";
          }

          return {
            ...tab,
            activeJob: null,
            status: newStatus,
          };
        }),
      }));
    },

    toggleTag: (tabId: string, tagId: string) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (tab.id !== tabId) return tab;

          const has = tab.activeTagIds.includes(tagId);
          const activeTagIds = has
            ? tab.activeTagIds.filter((id) => id !== tagId)
            : [...tab.activeTagIds, tagId];

          return {
            ...tab,
            activeTagIds,
            inputVersionToken: generateVersionToken(),
          };
        }),
      }));
    },

    removeTagFromAllTabs: (tagId: string) => {
      set((state) => ({
        tabs: state.tabs.map((tab) => {
          if (!tab.activeTagIds.includes(tagId)) return tab;
          return {
            ...tab,
            activeTagIds: tab.activeTagIds.filter((id) => id !== tagId),
          };
        }),
      }));
    },

    copyAcceptedOutput: async (tabId: string) => {
      const state = get();
      const tab = state.tabs.find((t) => t.id === tabId);
      if (!tab?.acceptedOutput) return;

      try {
        await navigator.clipboard.writeText(tab.acceptedOutput);
        get().addToast("Copied to clipboard", "success", 2000);
      } catch {
        get().addToast("Copy failed — clipboard unavailable", "error", 3000);
      }
    },
  };
};

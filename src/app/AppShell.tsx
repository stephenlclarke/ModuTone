// Phase: 8
// App shell — top-level layout with keyboard shortcuts, dialogs, toasts, and event listeners

import { useEffect, useCallback, useRef, useState } from "react";
import { TabStrip } from "../components/shell/TabStrip";
import { WorkspaceLayout } from "../components/shell/WorkspaceLayout";
import { StatusBar } from "../components/shell/StatusBar";
import { ConfirmDialog } from "../components/feedback/ConfirmDialog";
import { ToastContainer } from "../components/feedback/Toast";
import { Banner } from "../components/feedback/Banner";
import { ProfileSelector } from "../components/profiles/ProfileSelector";
import { ModelStatusChip } from "../components/shell/ModelStatusChip";
import { SettingsPanel } from "../components/settings/SettingsPanel";
import { useAppStore } from "../state/store";
import { subscribeToBackendEvents } from "../ipc/events";
import {
  appSetPrivacyBlackout,
  runtimeWarmModel,
  generationStartInitial,
} from "../ipc/commands";
import { RETRY_DELAY_MS } from "../state/modelLoading/types";
import type { IpcError } from "../ipc/types";

/**
 * Orchestrator hook — reacts to modelLoading phase changes and drives
 * the IPC calls for model warm-up, retry timers, and fallback toasts.
 */
function useModelLoadOrchestrator() {
  const phase = useAppStore((state) => state.modelLoading.phase);
  const targetModelId = useAppStore(
    (state) => state.modelLoading.targetModelId,
  );
  const retryCount = useAppStore((state) => state.modelLoading.retryCount);
  const fallbackModelId = useAppStore(
    (state) => state.modelLoading.fallbackModelId,
  );
  const originalTargetModelId = useAppStore(
    (state) => state.modelLoading.originalTargetModelId,
  );

  // Trigger IPC warm-model call when phase transitions to "loading" or "fallback_loading"
  useEffect(() => {
    if (
      (phase === "loading" || phase === "fallback_loading") &&
      targetModelId
    ) {
      runtimeWarmModel({
        contractVersion: 1,
        modelId: targetModelId,
      }).catch((err: IpcError) => {
        useAppStore.getState().handleLoadIpcError(err.code, err.message);
      });
    }
  }, [phase, targetModelId, retryCount]);

  // Retry timer — when waiting_retry, schedule a retry after RETRY_DELAY_MS
  useEffect(() => {
    if (phase === "waiting_retry" && targetModelId) {
      const timer = setTimeout(() => {
        useAppStore.getState().startRetryAttempt();
      }, RETRY_DELAY_MS);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [phase, targetModelId, retryCount]);

  // Toast on fallback activation
  const prevPhaseRef = useRef(phase);
  useEffect(() => {
    if (
      prevPhaseRef.current !== "fallback_active" &&
      phase === "fallback_active"
    ) {
      const models = useAppStore.getState().metadata.models;
      const targetName = originalTargetModelId
        ? (models.find((m) => m.id === originalTargetModelId)?.displayName ??
          originalTargetModelId)
        : "Selected model";
      const fbName = fallbackModelId
        ? (models.find((m) => m.id === fallbackModelId)?.displayName ??
          fallbackModelId)
        : "previous model";

      useAppStore
        .getState()
        .addToast(
          `Could not load ${targetName}. Using ${fbName} instead.`,
          "neutral",
          5000,
        );
    }
    prevPhaseRef.current = phase;
  }, [phase, fallbackModelId, originalTargetModelId]);
}

export function AppShell() {
  const tabs = useAppStore((state) => state.tabs);
  const activeTabId = useAppStore((state) => state.activeTabId);
  const pendingCloseTabId = useAppStore((state) => state.pendingCloseTabId);
  const pendingClearTabId = useAppStore((state) => state.pendingClearTabId);
  const pendingReGenerateTabId = useAppStore(
    (state) => state.pendingReGenerateTabId,
  );

  const createTab = useAppStore((state) => state.createTab);
  const switchTab = useAppStore((state) => state.switchTab);
  const requestCloseTab = useAppStore((state) => state.requestCloseTab);
  const confirmCloseTab = useAppStore((state) => state.confirmCloseTab);
  const cancelCloseTab = useAppStore((state) => state.cancelCloseTab);
  const confirmClearTab = useAppStore((state) => state.confirmClearTab);
  const cancelClearTab = useAppStore((state) => state.cancelClearTab);
  const confirmReGenerate = useAppStore((state) => state.confirmReGenerate);
  const cancelReGenerate = useAppStore((state) => state.cancelReGenerate);
  const loadMetadata = useAppStore((state) => state.loadMetadata);
  const loadRuntimeStatus = useAppStore((state) => state.loadRuntimeStatus);

  // Settings panel state
  const [settingsPanelOpen, setSettingsPanelOpen] = useState(false);

  // Check if pending close tab has an active job (for dialog extra text)
  const pendingCloseTab = pendingCloseTabId
    ? tabs.find((t) => t.id === pendingCloseTabId)
    : undefined;
  const pendingClearTab = pendingClearTabId
    ? tabs.find((t) => t.id === pendingClearTabId)
    : undefined;

  // Subscribe to backend events (runtime status, worker crashes)
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    subscribeToBackendEvents().then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  // Load metadata (settings, profiles, tags, models) from backend on mount
  useEffect(() => {
    loadMetadata();
  }, [loadMetadata]);

  // Load runtime status (platform capabilities) from backend on mount
  useEffect(() => {
    loadRuntimeStatus();
  }, [loadRuntimeStatus]);

  // Restore privacy blackout on startup if previously enabled
  const privacyBlackoutEnabled = useAppStore(
    (state) => state.metadata.settings?.privacyBlackoutEnabled ?? false,
  );
  const metadataLoadStatus = useAppStore((state) => state.metadata.loadStatus);
  const privacyRestoredRef = useRef(false);

  useEffect(() => {
    if (
      metadataLoadStatus === "loaded" &&
      privacyBlackoutEnabled &&
      !privacyRestoredRef.current
    ) {
      privacyRestoredRef.current = true;
      appSetPrivacyBlackout({ contractVersion: 1, enabled: true }).catch(() => {
        // Best effort — silently ignore failure
      });
    }
  }, [metadataLoadStatus, privacyBlackoutEnabled]);

  // Auto-load previously selected model on startup via the loading system
  const autoLoadAttemptedRef = useRef(false);

  useEffect(() => {
    if (metadataLoadStatus !== "loaded") return;
    if (autoLoadAttemptedRef.current) return;

    const settings = useAppStore.getState().metadata.settings;
    const runtime = useAppStore.getState().runtime;
    const models = useAppStore.getState().metadata.models;

    // Initialize lastKnownGoodModelId from persisted settings
    if (settings?.lastSuccessfulModelId) {
      useAppStore
        .getState()
        .initializeLastKnownGood(settings.lastSuccessfulModelId);
    }

    if (!settings?.selectedModelId) return;
    if (runtime.loadedModelId === settings.selectedModelId) return;

    const model = models.find((m) => m.id === settings.selectedModelId);
    if (!model?.isInstalled) return;

    autoLoadAttemptedRef.current = true;
    useAppStore.getState().initiateModelLoad(settings.selectedModelId);
  }, [metadataLoadStatus]);

  // Auto-reload model after worker crash recovery.
  // When the worker is force-killed (e.g. cancel timeout), it restarts
  // with no model loaded. This effect detects that and re-triggers the load.
  const workerState = useAppStore((state) => state.runtime.workerState);
  const loadedModelId = useAppStore((state) => state.runtime.loadedModelId);
  const loadingPhase = useAppStore((state) => state.modelLoading.phase);
  useEffect(() => {
    if (!autoLoadAttemptedRef.current) return; // Not until initial load attempted
    if (workerState !== "idle") return; // Worker must be ready
    if (loadedModelId) return; // Model already loaded
    if (loadingPhase !== "idle" && loadingPhase !== "failed") return; // Not if already loading

    const settings = useAppStore.getState().metadata.settings;
    if (!settings?.selectedModelId) return;

    const models = useAppStore.getState().metadata.models;
    const model = models.find((m) => m.id === settings.selectedModelId);
    if (!model?.isInstalled) return;

    useAppStore.getState().initiateModelLoad(settings.selectedModelId);
  }, [workerState, loadedModelId, loadingPhase]);

  // Model load orchestrator — drives IPC calls based on loading phase
  useModelLoadOrchestrator();

  // Focus management: after clear-tab confirm, focus input editor.
  const prevPendingClearTabId = useRef<string | null>(null);
  useEffect(() => {
    const previousId = prevPendingClearTabId.current;
    prevPendingClearTabId.current = pendingClearTabId;

    if (previousId !== null && pendingClearTabId === null) {
      const tab = tabs.find((t) => t.id === previousId);
      if (tab && tab.status === "empty") {
        const textarea = document.querySelector<HTMLTextAreaElement>(
          '[data-testid="input-editor"]',
        );
        textarea?.focus();
      }
    }
  }, [pendingClearTabId, tabs]);

  const activeTab = tabs.find((t) => t.id === activeTabId);

  // Generation progress bar visibility
  const isGenerating =
    activeTab?.status === "generating" ||
    activeTab?.status === "proposal_generating";

  // Focus management for refinement flow transitions
  const prevStatusRef = useRef<string | undefined>(undefined);
  const prevVersionRef = useRef<number>(0);

  useEffect(() => {
    const prevStatus = prevStatusRef.current;
    const prevVersion = prevVersionRef.current;
    const currentStatus = activeTab?.status;
    const currentVersion = activeTab?.acceptedOutputVersion ?? 0;

    prevStatusRef.current = currentStatus;
    prevVersionRef.current = currentVersion;

    if (!prevStatus || !currentStatus) return;

    // proposal_generating -> proposal_ready: focus proposed output
    if (
      prevStatus === "proposal_generating" &&
      currentStatus === "proposal_ready"
    ) {
      const textarea = document.querySelector<HTMLTextAreaElement>(
        '[data-testid="proposed-output-preview"]',
      );
      textarea?.focus();
      return;
    }

    // proposal_ready -> output_ready: distinguish accept vs reject
    if (prevStatus === "proposal_ready" && currentStatus === "output_ready") {
      if (currentVersion > prevVersion) {
        const textarea = document.querySelector<HTMLTextAreaElement>(
          '[data-testid="accepted-output-editor"]',
        );
        textarea?.focus();
      } else {
        const textarea = document.querySelector<HTMLTextAreaElement>(
          '[data-testid="refinement-instruction-box"]',
        );
        textarea?.focus();
      }
    }
  }, [activeTab?.status, activeTab?.acceptedOutputVersion]);

  // Re-Generate confirmation handler
  const handleConfirmReGenerate = useCallback(async () => {
    const state = useAppStore.getState();
    const tabId = state.pendingReGenerateTabId;
    if (!tabId) return;

    const tab = state.tabs.find((t) => t.id === tabId);
    if (!tab) return;

    const settings = state.metadata.settings;
    const loadedModelId = state.runtime.loadedModelId;
    if (!loadedModelId) {
      confirmReGenerate();
      useAppStore.getState().handleGenerationCommandFailed(tabId, {
        code: "MODEL_NOT_READY",
        message: "No model is loaded. Select and warm a model before generating.",
        subsystem: "inference",
      });
      return;
    }

    const selectedProfileId =
      settings?.lastSelectedProfileId ??
      state.metadata.profiles.find((p) => p.isFactoryDefault)?.id ??
      "factory-default";

    confirmReGenerate();

    try {
      await generationStartInitial({
        contractVersion: 1,
        tabId,
        modelId: loadedModelId,
        profileId: selectedProfileId,
        activeTagIds: tab.activeTagIds,
        sourceText: tab.inputText,
        inputVersionToken: tab.inputVersionToken,
      });
    } catch (err) {
      useAppStore.getState().handleGenerationCommandFailed(tabId, err);
    }
  }, [confirmReGenerate]);

  // Global keyboard shortcuts
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      const isCtrlOrCmd = e.ctrlKey || e.metaKey;

      // Ctrl+, / Cmd+, — open settings
      if (isCtrlOrCmd && e.key === ",") {
        e.preventDefault();
        setSettingsPanelOpen((prev) => !prev);
        return;
      }

      // Ctrl+T / Cmd+T — new tab
      if (isCtrlOrCmd && e.key === "t") {
        e.preventDefault();
        createTab();
        return;
      }

      // Ctrl+W / Cmd+W — close active tab
      if (isCtrlOrCmd && e.key === "w") {
        e.preventDefault();
        if (activeTabId) {
          requestCloseTab(activeTabId);
        }
        return;
      }

      // Ctrl+Tab — next tab
      if (e.ctrlKey && e.key === "Tab" && !e.shiftKey) {
        e.preventDefault();
        const currentIndex = tabs.findIndex((t) => t.id === activeTabId);
        if (currentIndex !== -1 && tabs.length > 1) {
          const nextIndex = (currentIndex + 1) % tabs.length;
          const nextTab = tabs[nextIndex];
          if (nextTab) {
            switchTab(nextTab.id);
          }
        }
        return;
      }

      // Ctrl+Shift+Tab — previous tab
      if (e.ctrlKey && e.key === "Tab" && e.shiftKey) {
        e.preventDefault();
        const currentIndex = tabs.findIndex((t) => t.id === activeTabId);
        if (currentIndex !== -1 && tabs.length > 1) {
          const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
          const prevTab = tabs[prevIndex];
          if (prevTab) {
            switchTab(prevTab.id);
          }
        }
        return;
      }

      // Ctrl+1 through Ctrl+9 — switch to tab N
      if (isCtrlOrCmd && e.key >= "1" && e.key <= "9") {
        e.preventDefault();
        const tabIndex = parseInt(e.key, 10) - 1;
        const targetTab = tabs[tabIndex];
        if (targetTab) {
          switchTab(targetTab.id);
        }
        return;
      }

      // Ctrl+Enter / Cmd+Enter — trigger Refine or Generate
      if (isCtrlOrCmd && e.key === "Enter") {
        e.preventDefault();

        const activeEl = document.activeElement;
        if (
          activeEl &&
          activeEl.matches('[data-testid="refinement-instruction-box"]')
        ) {
          const refineBtn = document.querySelector<HTMLButtonElement>(
            '[data-testid="refine-button"]',
          );
          if (refineBtn && !refineBtn.disabled) {
            refineBtn.click();
            return;
          }
        }

        const generateBtn = document.querySelector<HTMLButtonElement>(
          '[data-testid="generate-button"]',
        );
        if (generateBtn && !generateBtn.disabled) {
          generateBtn.click();
        }
        return;
      }
    },
    [tabs, activeTabId, createTab, switchTab, requestCloseTab],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="app-shell">
      <TabStrip />
      <div className="toolbar">
        <ProfileSelector />
        <div className="toolbar-spacer" />
        <ModelStatusChip onOpenSettings={() => setSettingsPanelOpen(true)} />
        <button
          className="settings-button"
          onClick={() => setSettingsPanelOpen(true)}
          title="Settings (Ctrl+,)"
          data-testid="settings-button"
          aria-label="Open settings"
        >
          &#9881;
        </button>
      </div>
      <Banner />
      <WorkspaceLayout onOpenSettings={() => setSettingsPanelOpen(true)} />
      <StatusBar />
      {isGenerating && <div className="generation-progress-bar" />}

      {/* Close tab confirmation dialog */}
      {pendingCloseTabId && pendingCloseTab && (
        <ConfirmDialog
          title="Close this tab?"
          body="All text in this tab will be lost."
          confirmLabel="Close Tab"
          onConfirm={confirmCloseTab}
          onCancel={cancelCloseTab}
        />
      )}

      {/* Clear tab confirmation dialog */}
      {pendingClearTabId && pendingClearTab && (
        <ConfirmDialog
          title="Clear this tab?"
          body="All text in this tab will be permanently removed. This cannot be undone."
          extraBody={
            pendingClearTab.activeJob
              ? "An active generation will also be canceled."
              : undefined
          }
          confirmLabel="Clear"
          confirmDestructive
          onConfirm={confirmClearTab}
          onCancel={cancelClearTab}
        />
      )}

      {/* Re-Generate confirmation dialog */}
      {pendingReGenerateTabId && (
        <ConfirmDialog
          title="Overwrite existing output?"
          body="Re-generating will replace your current output with a new result."
          confirmLabel="Re-Generate"
          confirmDestructive
          onConfirm={handleConfirmReGenerate}
          onCancel={cancelReGenerate}
        />
      )}

      <SettingsPanel
        open={settingsPanelOpen}
        onClose={() => setSettingsPanelOpen(false)}
      />
      <ToastContainer />
    </div>
  );
}

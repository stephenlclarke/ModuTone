// Phase: 8
// Persistent notification banners for system-level conditions

import { useAppStore } from "../../state/store";

interface BannerItem {
  key: string;
  message: string;
  style: "warning" | "info";
}

export function Banner() {
  const metadataStoreWritable = useAppStore(
    (state) => state.runtime.metadataStoreWritable,
  );
  const workerState = useAppStore((state) => state.runtime.workerState);
  const appState = useAppStore((state) => state.runtime.appState);
  const models = useAppStore((state) => state.metadata.models);
  const selectedModelId = useAppStore(
    (state) => state.metadata.settings?.selectedModelId ?? null,
  );
  const loadingPhase = useAppStore((state) => state.modelLoading.phase);
  const lastErrorClassification = useAppStore(
    (state) => state.modelLoading.lastError?.classification ?? null,
  );

  const banners: BannerItem[] = [];

  if (!metadataStoreWritable) {
    banners.push({
      key: "readonly",
      message: "Settings and profiles are read-only. Changes won't be saved.",
      style: "warning",
    });
  }

  const anyInstalled = models.some((m) => m.isInstalled);

  // Priority chain — first match wins (after read-only check above)
  if (appState === "degraded") {
    banners.push({
      key: "worker-unavailable",
      message: "Text processing is unavailable. Restart the app to try again.",
      style: "warning",
    });
  } else if (models.length === 0 || !anyInstalled) {
    banners.push({
      key: "no-models",
      message:
        "Download a model in Settings or place a local model in the models folder.",
      style: "info",
    });
  } else if (anyInstalled && !selectedModelId) {
    banners.push({
      key: "no-model-selected",
      message: "Select a model in Settings to get started.",
      style: "info",
    });
  } else if (loadingPhase === "failed" && selectedModelId) {
    if (lastErrorClassification === "model_invalid") {
      banners.push({
        key: "model-invalid",
        message:
          "Selected model file appears incomplete or corrupt. Choose a different model in Settings.",
        style: "warning",
      });
    } else if (lastErrorClassification === "runtime_missing") {
      banners.push({
        key: "runtime-missing",
        message:
          "The selected MLX model needs Python with mlx-lm and turboquant-mlx-full installed.",
        style: "warning",
      });
    } else if (lastErrorClassification === "insufficient_memory") {
      banners.push({
        key: "model-memory",
        message:
          "Not enough memory to load the selected model. Choose a smaller model in Settings.",
        style: "warning",
      });
    } else {
      banners.push({
        key: "model-failed",
        message:
          "Model failed to load. Try selecting a different model in Settings.",
        style: "warning",
      });
    }
  } else if (selectedModelId && workerState === "warming") {
    banners.push({
      key: "model-loading",
      message: "Loading model\u2026",
      style: "info",
    });
  }

  if (banners.length === 0) return null;

  return (
    <div className="banner-container" data-testid="banner-container">
      {banners.map((banner) => (
        <div key={banner.key} className={`banner banner-${banner.style}`}>
          <span className="banner-message">{banner.message}</span>
        </div>
      ))}
    </div>
  );
}

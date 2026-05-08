// Model loading state machine types

export type ModelLoadErrorClass =
  | "transient"
  | "model_invalid"
  | "insufficient_memory";

export type ModelLoadingPhase =
  | "idle"
  | "loading"
  | "waiting_retry"
  | "failed"
  | "fallback_loading"
  | "fallback_active";

export interface ModelLoadError {
  message: string;
  classification: ModelLoadErrorClass;
}

export interface ModelLoadingState {
  phase: ModelLoadingPhase;
  targetModelId: string | null;
  originalTargetModelId: string | null;
  retryCount: number;
  lastError: ModelLoadError | null;
  lastKnownGoodModelId: string | null;
  fallbackModelId: string | null;
}

export const MAX_RETRIES = 20;
export const RETRY_DELAY_MS = 10_000;

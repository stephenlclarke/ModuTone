import type { ModelEntry } from "../ipc/types";

/**
 * Resolve the display name for a model.
 * Priority: user alias > backend display_name > raw model ID
 */
export function resolveModelDisplayName(
  modelId: string,
  models: ModelEntry[],
  aliases: Record<string, string>,
): string {
  const alias = aliases[modelId];
  if (alias) return alias;

  const model = models.find((m) => m.id === modelId);
  return model?.displayName ?? modelId;
}

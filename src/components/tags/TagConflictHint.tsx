// Phase: 6

import type { BuiltInTagEntry, CustomTagEntry } from "../../ipc/types";

interface TagConflictHintProps {
  activeTagIds: string[];
  builtInTags: BuiltInTagEntry[];
  customTags: CustomTagEntry[];
}

/**
 * Informational hint shown when multiple tags in the same balancing group
 * are active. Backend prompt composition remains authoritative; this UI
 * hint may under-report balancing for custom tags (which have no
 * balancingGroup field on the frontend).
 */
export function TagConflictHint({
  activeTagIds,
  builtInTags,
}: TagConflictHintProps) {
  if (activeTagIds.length === 0) return null;

  // Count active built-in tags per balancing group
  const groupCounts = new Map<string, number>();
  for (const tagId of activeTagIds) {
    const tag = builtInTags.find((t) => t.id === tagId);
    if (tag?.balancingGroup) {
      groupCounts.set(
        tag.balancingGroup,
        (groupCounts.get(tag.balancingGroup) ?? 0) + 1,
      );
    }
  }

  // Check if any group has 2+ active tags
  let hasConflict = false;
  for (const count of groupCounts.values()) {
    if (count >= 2) {
      hasConflict = true;
      break;
    }
  }

  if (!hasConflict) return null;

  return (
    <p className="tag-conflict-hint" data-testid="tag-conflict-hint">
      These tags will be balanced together.
    </p>
  );
}

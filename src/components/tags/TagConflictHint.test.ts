// Phase: 6
// Tests for TagConflictHint — balancing group detection

import { describe, it, expect } from "vitest";
import type { BuiltInTagEntry } from "../../ipc/types";

// We test the conflict detection logic directly rather than rendering,
// since this is pure computation.
// Replicate the logic from TagConflictHint for unit testing.

function detectConflict(
  activeTagIds: string[],
  builtInTags: BuiltInTagEntry[],
): boolean {
  if (activeTagIds.length === 0) return false;

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

  for (const count of groupCounts.values()) {
    if (count >= 2) return true;
  }

  return false;
}

// --- Fixtures ---

function makeBuiltInTag(id: string, balancingGroup?: string): BuiltInTagEntry {
  const tag: BuiltInTagEntry = {
    id,
    name: `Tag ${id}`,
    category: "tone",
    instructionBody: "instructions",
    isBuiltIn: true,
  };
  if (balancingGroup !== undefined) {
    tag.balancingGroup = balancingGroup;
  }
  return tag;
}

describe("TagConflictHint logic", () => {
  it("returns false when no active tags", () => {
    expect(detectConflict([], [])).toBe(false);
  });

  it("returns false when one tag per group", () => {
    const tags = [
      makeBuiltInTag("t1", "formality"),
      makeBuiltInTag("t2", "length"),
    ];
    expect(detectConflict(["t1", "t2"], tags)).toBe(false);
  });

  it("returns true when two tags in same group", () => {
    const tags = [
      makeBuiltInTag("formal", "formality"),
      makeBuiltInTag("casual", "formality"),
    ];
    expect(detectConflict(["formal", "casual"], tags)).toBe(true);
  });

  it("returns true when three tags in same group", () => {
    const tags = [
      makeBuiltInTag("t1", "length"),
      makeBuiltInTag("t2", "length"),
      makeBuiltInTag("t3", "length"),
    ];
    expect(detectConflict(["t1", "t2", "t3"], tags)).toBe(true);
  });

  it("returns false when tags have no balancing group", () => {
    const tags = [
      makeBuiltInTag("t1", undefined),
      makeBuiltInTag("t2", undefined),
    ];
    expect(detectConflict(["t1", "t2"], tags)).toBe(false);
  });

  it("returns false when only one active tag even if duplicates exist", () => {
    const tags = [
      makeBuiltInTag("formal", "formality"),
      makeBuiltInTag("casual", "formality"),
    ];
    // Only one is active
    expect(detectConflict(["formal"], tags)).toBe(false);
  });

  it("detects conflict in one group while other groups are fine", () => {
    const tags = [
      makeBuiltInTag("formal", "formality"),
      makeBuiltInTag("casual", "formality"),
      makeBuiltInTag("short", "length"),
    ];
    expect(detectConflict(["formal", "casual", "short"], tags)).toBe(true);
  });

  it("ignores custom tags (no balancingGroup field)", () => {
    const builtInTags = [makeBuiltInTag("t1", "formality")];
    // custom tag IDs in activeTagIds won't match any built-in tag
    expect(detectConflict(["t1", "custom-1", "custom-2"], builtInTags)).toBe(
      false,
    );
  });
});

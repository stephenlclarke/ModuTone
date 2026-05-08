// Phase: 3
// Tests for tab state transition validation

import { describe, it, expect } from "vitest";
import { isValidTransition } from "./tabStateMachine";
import type { TabStatus } from "./types";

const ALL_STATUSES: TabStatus[] = [
  "empty",
  "editing",
  "generating",
  "output_ready",
  "refine_editing",
  "proposal_generating",
  "proposal_ready",
  "error",
];

describe("tabStateMachine", () => {
  describe("valid transitions", () => {
    // From state_machines.md §2
    const validTransitions: [TabStatus, TabStatus][] = [
      // empty
      ["empty", "editing"],
      // editing
      ["editing", "empty"],
      ["editing", "generating"],
      // generating
      ["generating", "output_ready"],
      ["generating", "error"],
      ["generating", "editing"],
      // output_ready
      ["output_ready", "refine_editing"],
      ["output_ready", "generating"],
      // refine_editing
      ["refine_editing", "generating"],
      ["refine_editing", "output_ready"],
      ["refine_editing", "proposal_generating"],
      // proposal_generating
      ["proposal_generating", "proposal_ready"],
      ["proposal_generating", "refine_editing"],
      // proposal_ready
      ["proposal_ready", "output_ready"],
      ["proposal_ready", "proposal_generating"],
      ["proposal_ready", "generating"],
      // error
      ["error", "output_ready"],
      ["error", "editing"],
      ["error", "empty"],
    ];

    for (const [from, to] of validTransitions) {
      it(`allows ${from} → ${to}`, () => {
        expect(isValidTransition(from, to)).toBe(true);
      });
    }
  });

  describe("forbidden transitions", () => {
    const forbiddenTransitions: [TabStatus, TabStatus][] = [
      // empty → cannot go to output_ready, generating, etc. directly
      ["empty", "generating"],
      ["empty", "output_ready"],
      ["empty", "error"],
      ["empty", "proposal_ready"],
      // editing → cannot go directly to output_ready
      ["editing", "output_ready"],
      ["editing", "refine_editing"],
      ["editing", "proposal_generating"],
      // generating → cannot go to editing via direct transition (cancel is separate)
      ["generating", "refine_editing"],
      ["generating", "proposal_generating"],
      ["generating", "empty"],
      // output_ready → cannot skip to proposal_ready
      ["output_ready", "proposal_ready"],
      ["output_ready", "empty"],
      ["output_ready", "editing"],
      // refine_editing → cannot go to proposal_ready directly
      ["refine_editing", "proposal_ready"],
      ["refine_editing", "empty"],
      // proposal_generating → cannot go to output_ready directly
      ["proposal_generating", "output_ready"],
      ["proposal_generating", "empty"],
      // proposal_ready → cannot go to editing
      ["proposal_ready", "editing"],
      ["proposal_ready", "empty"],
      // error → cannot go to generating directly
      ["error", "generating"],
      ["error", "refine_editing"],
      // self-transitions are forbidden
      ["empty", "empty"],
      ["editing", "editing"],
      ["generating", "generating"],
      ["output_ready", "output_ready"],
    ];

    for (const [from, to] of forbiddenTransitions) {
      it(`forbids ${from} → ${to}`, () => {
        expect(isValidTransition(from, to)).toBe(false);
      });
    }
  });

  describe("completeness", () => {
    it("covers all statuses as source", () => {
      for (const status of ALL_STATUSES) {
        // Every status should have at least one valid outgoing transition
        const hasTransition = ALL_STATUSES.some((to) =>
          isValidTransition(status, to),
        );
        expect(hasTransition).toBe(true);
      }
    });
  });
});

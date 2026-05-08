// Phase: 3
// Tab state transition logic per state_machines.md §2
// All transitions not listed are FORBIDDEN.

import type { TabStatus } from "./types";

/**
 * Valid transitions for the tab lifecycle state machine.
 * The universal clear transition (ANY → empty via confirmClearTab)
 * bypasses this table and is handled separately.
 */
const VALID_TRANSITIONS: Record<TabStatus, readonly TabStatus[]> = {
  empty: ["editing"],
  editing: ["empty", "generating"],
  generating: ["output_ready", "error", "editing"],
  output_ready: ["refine_editing", "generating"],
  refine_editing: ["generating", "output_ready", "proposal_generating"],
  proposal_generating: ["proposal_ready", "refine_editing"],
  proposal_ready: ["output_ready", "proposal_generating", "generating"],
  error: ["output_ready", "editing", "empty"],
};

/**
 * Check whether a normal tab state transition is allowed.
 */
export function isValidTransition(from: TabStatus, to: TabStatus): boolean {
  return VALID_TRANSITIONS[from].includes(to);
}

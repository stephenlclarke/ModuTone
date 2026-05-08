// Phase: 1
// inputVersionToken generator (UUID v4)

import { v4 as uuidv4 } from "uuid";

/**
 * Generate a new input version token.
 * Called whenever the input state changes in a way that would
 * invalidate a pending generation result.
 */
export function generateVersionToken(): string {
  return uuidv4();
}

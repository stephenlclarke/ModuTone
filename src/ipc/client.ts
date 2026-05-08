// Phase: 2
// Tauri invoke wrapper — sole point of contact with @tauri-apps/api

import { invoke } from "@tauri-apps/api/core";
import type { IpcError } from "./types";

export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw error as IpcError;
  }
}

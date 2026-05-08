// Phase: 3
// Session state types — ephemeral, memory-only

import type { IpcError } from "../../ipc/types";

export type TabStatus =
  | "empty"
  | "editing"
  | "generating"
  | "output_ready"
  | "refine_editing"
  | "proposal_generating"
  | "proposal_ready"
  | "error";

export interface JobRef {
  jobId: string;
  requestKind: "initial_rewrite" | "refinement";
}

export interface UserSafeError {
  message: string;
  cause: string;
  action: string;
  source?: IpcError;
}

export interface SessionTab {
  id: string;
  title: string;
  inputText: string;
  activeTagIds: string[];
  acceptedOutput: string | null;
  acceptedOutputVersion: number;
  proposedOutput: string | null;
  proposedOutputBaseVersion: number | null;
  refinementInstruction: string;
  activeJob: JobRef | null;
  status: TabStatus;
  error: UserSafeError | null;
  inputVersionToken: string;
}

export interface ToastMessage {
  id: string;
  message: string;
  style: "success" | "error" | "neutral";
  duration: number;
}

import { describe, expect, it } from "vitest";
import { CONTRACT_VERSION } from "../../src/ipc/types";
import type {
  ModelDownloadStartRequest,
  StartInitialRequest,
  WarmModelRequest,
} from "../../src/ipc/types";

describe("IPC contract types", () => {
  it("uses the backend-supported contract version", () => {
    expect(CONTRACT_VERSION).toBe(1);
  });

  it("keeps versioned command payloads in camelCase shape", () => {
    const warmModel = {
      contractVersion: CONTRACT_VERSION,
      modelId: "qwen2.5-3b-instruct",
    } satisfies WarmModelRequest;

    const modelDownload = {
      contractVersion: CONTRACT_VERSION,
      modelId: "gpt-oss-20b-tq3",
    } satisfies ModelDownloadStartRequest;

    const initialGeneration = {
      contractVersion: CONTRACT_VERSION,
      tabId: "tab-1",
      modelId: "qwen2.5-3b-instruct",
      profileId: "default",
      activeTagIds: ["plain"],
      sourceText: "Rewrite this.",
      inputVersionToken: "input-1",
    } satisfies StartInitialRequest;

    expect(Object.keys(warmModel)).toEqual(["contractVersion", "modelId"]);
    expect(Object.keys(modelDownload)).toEqual(["contractVersion", "modelId"]);
    expect(Object.keys(initialGeneration)).toEqual([
      "contractVersion",
      "tabId",
      "modelId",
      "profileId",
      "activeTagIds",
      "sourceText",
      "inputVersionToken",
    ]);
  });
});

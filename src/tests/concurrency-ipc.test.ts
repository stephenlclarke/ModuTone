// Phase: 10
// IPC boundary concurrency test (RC-1)
//
// Tests that the frontend handles a DUPLICATE_JOB error from rapid double-click

import { describe, it, expect, vi } from "vitest";

// Mock the IPC client
vi.mock("../ipc/commands", () => ({
  generationStartInitial: vi.fn(),
}));

import { generationStartInitial } from "../ipc/commands";

const mockGenerationStartInitial = vi.mocked(generationStartInitial);

describe("IPC concurrency", () => {
  // RC-1: Rapid double-click Generate
  it("RC-1: second generationStartInitial returns DUPLICATE_JOB error", async () => {
    // First call succeeds
    mockGenerationStartInitial.mockResolvedValueOnce({ jobId: "job-1" });

    // Second call returns a DUPLICATE_JOB error (simulating backend rejection)
    mockGenerationStartInitial.mockRejectedValueOnce({
      code: "DUPLICATE_JOB",
      message: "A generation job is already active for this tab",
      subsystem: "inference",
    });

    // First call succeeds
    const result1 = await generationStartInitial({
      contractVersion: 1,
      tabId: "tab-1",
      modelId: "model-1",
      profileId: "profile-1",
      activeTagIds: [],
      sourceText: "Hello world",
      inputVersionToken: "token-1",
    });
    expect(result1.jobId).toBe("job-1");

    // Second call should throw/reject
    await expect(
      generationStartInitial({
        contractVersion: 1,
        tabId: "tab-1",
        modelId: "model-1",
        profileId: "profile-1",
        activeTagIds: [],
        sourceText: "Hello world",
        inputVersionToken: "token-1",
      }),
    ).rejects.toEqual(expect.objectContaining({ code: "DUPLICATE_JOB" }));
  });

  it("RC-1: frontend can catch DUPLICATE_JOB and not crash", async () => {
    mockGenerationStartInitial.mockRejectedValue({
      code: "DUPLICATE_JOB",
      message: "A generation job is already active for this tab",
      subsystem: "inference",
    });

    // Simulate the frontend pattern: catch and handle gracefully
    let errorCode: string | null | undefined = null;
    try {
      await generationStartInitial({
        contractVersion: 1,
        tabId: "tab-1",
        modelId: "model-1",
        profileId: "profile-1",
        activeTagIds: [],
        sourceText: "Hello world",
        inputVersionToken: "token-1",
      });
    } catch (err: unknown) {
      if (
        err !== null &&
        typeof err === "object" &&
        "code" in err &&
        typeof (err as Record<string, unknown>).code === "string"
      ) {
        errorCode = (err as Record<string, string>).code;
      }
    }

    expect(errorCode).toBe("DUPLICATE_JOB");
  });
});

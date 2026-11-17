import { describe, expect, it } from "vitest";
import type { ScanCounters, ScanStatus } from "../services/scan";
import {
  IDLE_VIEW,
  ZERO_COUNTERS,
  applyEvent,
  applyStatus,
  completenessNote,
  isActive,
  parseRoots,
  type ScanView,
} from "./scanView";

const counters: ScanCounters = { ...ZERO_COUNTERS, files: 7, bytes: 70, directories: 2 };

const starting: ScanView = { ...IDLE_VIEW, phase: "starting" };
const running: ScanView = { ...IDLE_VIEW, phase: "running", jobId: "job-1" };

function progress(jobId: string, seq: number, filesSeen = 1) {
  return {
    kind: "progress" as const,
    payload: { jobId, seq, phase: "traverse", filesSeen, bytesSeen: 5, skipped: 0, elapsedMs: 9 },
  };
}

describe("applyEvent", () => {
  it("adopts the first job only while starting", () => {
    expect(applyEvent(starting, progress("job-1", 1)).jobId).toBe("job-1");
    expect(applyEvent(IDLE_VIEW, progress("job-1", 1))).toBe(IDLE_VIEW);
  });

  it("ignores events for another job", () => {
    expect(applyEvent(running, progress("job-2", 1))).toBe(running);
  });

  it("drops progress that is not newer than the last applied", () => {
    const first = applyEvent(running, progress("job-1", 3, 30));
    expect(first.counters.files).toBe(30);

    expect(applyEvent(first, progress("job-1", 3, 99))).toBe(first);
    expect(applyEvent(first, progress("job-1", 2, 99))).toBe(first);
    expect(applyEvent(first, progress("job-1", 4, 40)).counters.files).toBe(40);
  });

  it("keeps the cancelling phase while progress still arrives", () => {
    const cancelling: ScanView = { ...running, phase: "cancelling" };
    expect(applyEvent(cancelling, progress("job-1", 1)).phase).toBe("cancelling");
  });

  it("finishes on a terminal event and ignores later progress", () => {
    const done = applyEvent(running, {
      kind: "completed",
      payload: {
        jobId: "job-1",
        summary: counters,
        completeness: "partial",
        partial: true,
        elapsedMs: 50,
      },
    });

    expect(done.phase).toBe("completed");
    expect(done.completeness).toBe("partial");
    expect(done.counters.directories).toBe(2);
    expect(applyEvent(done, progress("job-1", 99))).toBe(done);
  });

  it("records a cancelled scan with its counts and a failed one with its code", () => {
    const cancelled = applyEvent(running, {
      kind: "cancelled",
      payload: { jobId: "job-1", reason: "user_requested", summary: counters, elapsedMs: 4 },
    });
    expect(cancelled.phase).toBe("cancelled");
    expect(cancelled.completeness).toBe("cancelled");

    const failed = applyEvent(running, {
      kind: "failed",
      payload: { jobId: "job-1", errorCode: "internal", message: "x" },
    });
    expect(failed.phase).toBe("failed");
    expect(failed.errorCode).toBe("internal");
  });
});

describe("applyStatus", () => {
  const status = (overrides: Partial<ScanStatus>): ScanStatus => ({
    jobId: "job-1",
    state: "running",
    seq: 5,
    elapsedMs: 100,
    counters,
    ...overrides,
  });

  it("recovers a running scan on an idle panel", () => {
    const view = applyStatus(IDLE_VIEW, status({}));
    expect(view.phase).toBe("running");
    expect(view.jobId).toBe("job-1");
    expect(view.seq).toBe(5);
  });

  it("maps queued to running and never lowers the sequence", () => {
    const view = applyStatus({ ...running, seq: 9 }, status({ state: "queued", seq: 2 }));
    expect(view.phase).toBe("running");
    expect(view.seq).toBe(9);
  });

  it("does not overwrite a terminal view or a different job", () => {
    const done: ScanView = { ...running, phase: "completed" };
    expect(applyStatus(done, status({}))).toBe(done);
    expect(applyStatus(running, status({ jobId: "job-2" }))).toBe(running);
  });

  it("adopts the authoritative terminal state", () => {
    const view = applyStatus(running, status({ state: "completed", completeness: "complete" }));
    expect(view.phase).toBe("completed");
    expect(view.completeness).toBe("complete");
  });
});

describe("helpers", () => {
  it("parses one root per line and drops blanks", () => {
    expect(parseRoots("  C:\\a \r\n\r\n D:\\b\n  \n")).toEqual(["C:\\a", "D:\\b"]);
    expect(parseRoots("   \n")).toEqual([]);
  });

  it("knows which phases hold the scan slot", () => {
    expect(isActive("starting")).toBe(true);
    expect(isActive("cancelling")).toBe(true);
    expect(isActive("completed")).toBe(false);
    expect(isActive("idle")).toBe(false);
  });

  it("never describes a partial or truncated scan as the whole tree", () => {
    expect(completenessNote("partial")).toMatch(/understate/i);
    expect(completenessNote("truncated")).toMatch(/only part/i);
    expect(completenessNote("complete")).toMatch(/not followed/i);
  });
});

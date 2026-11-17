import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { cancelScan, getScanStatus, startScan, subscribeToScan } from "./scan";

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

describe("scan service", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedListen.mockReset();
  });

  it("sends roots in a request object and unwraps the envelope", async () => {
    mockedInvoke.mockResolvedValue({ status: "ok", data: { jobId: "scan-1" } });

    await expect(startScan(["C:\\data"])).resolves.toEqual({ jobId: "scan-1" });
    expect(mockedInvoke).toHaveBeenCalledWith("start_scan", {
      request: { roots: ["C:\\data"] },
    });
  });

  it("passes the optional job id to cancel and status", async () => {
    mockedInvoke.mockResolvedValue({ status: "ok", data: { accepted: true } });
    await cancelScan("scan-1");
    expect(mockedInvoke).toHaveBeenCalledWith("cancel_scan", { jobId: "scan-1" });

    mockedInvoke.mockResolvedValue({ status: "err", error: errorPayload("not_found") });
    await expect(getScanStatus()).rejects.toMatchObject({ code: "not_found" });
    expect(mockedInvoke).toHaveBeenLastCalledWith("get_scan_status", { jobId: undefined });
  });

  it("subscribes to all four events, maps them to kinds, and unsubscribes together", async () => {
    const handlers = new Map<string, (event: { payload: unknown }) => void>();
    const unlisten = vi.fn();
    mockedListen.mockImplementation(async (name, handler) => {
      handlers.set(name, handler as (event: { payload: unknown }) => void);
      return unlisten;
    });
    const seen: unknown[] = [];

    const stop = await subscribeToScan((event) => seen.push(event));

    expect([...handlers.keys()]).toEqual([
      "scan:progress",
      "scan:completed",
      "scan:failed",
      "scan:cancelled",
    ]);
    handlers.get("scan:failed")?.({ payload: { jobId: "j", errorCode: "internal", message: "m" } });
    expect(seen).toEqual([
      { kind: "failed", payload: { jobId: "j", errorCode: "internal", message: "m" } },
    ]);

    stop();
    expect(unlisten).toHaveBeenCalledTimes(4);
  });

  it("removes listeners already made when a later subscription fails", async () => {
    const unlisten = vi.fn();
    mockedListen
      .mockResolvedValueOnce(unlisten)
      .mockResolvedValueOnce(unlisten)
      .mockRejectedValueOnce(new Error("no runtime"));

    await expect(subscribeToScan(() => undefined)).rejects.toThrow("no runtime");
    expect(unlisten).toHaveBeenCalledTimes(2);
  });
});

function errorPayload(code: string) {
  return { code, message: "m", retryable: false, correlationId: "c" };
}

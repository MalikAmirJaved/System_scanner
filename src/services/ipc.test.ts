import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { IpcError, invokeCommand } from "./ipc";

describe("invokeCommand", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("unwraps an ok envelope", async () => {
    invoke.mockResolvedValue({ status: "ok", data: { appVersion: "0.1.0" } });

    await expect(invokeCommand("get_app_info")).resolves.toEqual({ appVersion: "0.1.0" });
    expect(invoke).toHaveBeenCalledWith("get_app_info", undefined);
  });

  it("throws IpcError with the backend code for an err envelope", async () => {
    invoke.mockResolvedValue({
      status: "err",
      error: {
        code: "access_denied",
        message: "Windows denied access.",
        retryable: true,
        correlationId: "cid-1",
      },
    });

    const call = invokeCommand("get_system_snapshot");

    await expect(call).rejects.toBeInstanceOf(IpcError);
    await expect(call).rejects.toMatchObject({
      code: "access_denied",
      retryable: true,
      correlationId: "cid-1",
    });
  });

  it("passes arguments through unchanged", async () => {
    invoke.mockResolvedValue({ status: "ok", data: [] });

    await invokeCommand("list_processes", { limit: 500 });

    expect(invoke).toHaveBeenCalledWith("list_processes", { limit: 500 });
  });
});

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { IpcContractError, IpcError, invokeCommand } from "./ipc";
import { at, object, string, type Validator } from "./validate";

interface Thing {
  name: string;
}

const parseThing: Validator<Thing> = (value, path) => {
  const raw = object(value, path);
  return { name: string(raw.name, at(path, "name")) };
};

describe("invokeCommand", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("validates and unwraps an ok envelope", async () => {
    invoke.mockResolvedValue({ status: "ok", data: { name: "x", ignored: true } });

    await expect(invokeCommand("get_thing", parseThing)).resolves.toEqual({ name: "x" });
    expect(invoke).toHaveBeenCalledWith("get_thing", undefined);
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

    const call = invokeCommand("get_thing", parseThing);

    await expect(call).rejects.toBeInstanceOf(IpcError);
    await expect(call).rejects.toMatchObject({
      code: "access_denied",
      retryable: true,
      correlationId: "cid-1",
    });
  });

  it("passes arguments through unchanged", async () => {
    invoke.mockResolvedValue({ status: "ok", data: { name: "x" } });

    await invokeCommand("get_thing", parseThing, { limit: 500 });

    expect(invoke).toHaveBeenCalledWith("get_thing", { limit: 500 });
  });

  it("rejects an unknown envelope status instead of trusting the payload", async () => {
    invoke.mockResolvedValue({ status: "OK", data: { name: "x" } });

    const call = invokeCommand("get_thing", parseThing);

    await expect(call).rejects.toBeInstanceOf(IpcContractError);
    await expect(call).rejects.toMatchObject({ source: "get_thing" });
    await expect(call).rejects.toThrow(/status is not one of ok, err/);

    const error = await invokeCommand("get_thing", parseThing).catch((cause: unknown) => cause);
    expect(error).toBeInstanceOf(IpcContractError);
    expect(error).not.toBeInstanceOf(IpcError);
  });

  it("rejects a missing data payload", async () => {
    invoke.mockResolvedValue({ status: "ok" });

    await expect(invokeCommand("get_thing", parseThing)).rejects.toThrow(
      /the payload is not an object/,
    );
  });

  it("rejects a renamed DTO field and names the path", async () => {
    invoke.mockResolvedValue({ status: "ok", data: { app_name: "system-scanner" } });

    const call = invokeCommand("get_thing", parseThing);

    await expect(call).rejects.toBeInstanceOf(IpcContractError);
    await expect(call).rejects.toThrow(/name is not a string/);
  });

  it("rejects an undocumented error code", async () => {
    invoke.mockResolvedValue({
      status: "err",
      error: {
        code: "exploded",
        message: "?",
        retryable: false,
        correlationId: "cid-1",
      },
    });

    await expect(invokeCommand("get_thing", parseThing)).rejects.toThrow(
      /error\.code is not one of/,
    );
  });

  it("rejects an error payload with a missing correlation id", async () => {
    invoke.mockResolvedValue({
      status: "err",
      error: { code: "internal", message: "?", retryable: false },
    });

    await expect(invokeCommand("get_thing", parseThing)).rejects.toThrow(
      /error\.correlationId is not a string/,
    );
  });
});

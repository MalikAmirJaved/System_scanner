import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getAppInfo, type AppInfo } from "../services/app";
import { IpcError } from "../services/ipc";
import { BackendStatus } from "./BackendStatus";

vi.mock("../services/app", () => ({ getAppInfo: vi.fn() }));

const mockedGetAppInfo = vi.mocked(getAppInfo);

const appInfo: AppInfo = {
  appName: "system-scanner",
  appVersion: "0.1.0",
  tauriVersion: "2.12.5",
  os: "windows",
  arch: "x86_64",
  debug: true,
  schemaVersion: 1,
};

describe("BackendStatus", () => {
  beforeEach(() => {
    mockedGetAppInfo.mockReset();
  });

  it("shows the backend version once the command resolves", async () => {
    mockedGetAppInfo.mockResolvedValue(appInfo);

    render(<BackendStatus />);

    expect(await screen.findByText(/backend 0\.1\.0/i)).toBeInTheDocument();
  });

  it("reports the error code and recovers on retry", async () => {
    mockedGetAppInfo
      .mockRejectedValueOnce(
        new IpcError({
          code: "access_denied",
          message: "Windows denied access.",
          retryable: true,
          correlationId: "cid-1",
        }),
      )
      .mockResolvedValueOnce(appInfo);

    render(<BackendStatus />);

    expect(await screen.findByText(/backend unavailable \(access_denied\)/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));

    expect(await screen.findByText(/backend 0\.1\.0/i)).toBeInTheDocument();
  });

  it("hides internal error details behind the status title", async () => {
    mockedGetAppInfo.mockRejectedValue(
      new IpcError({
        code: "internal",
        message: "Safe message.",
        detail: "diagnostic detail",
        retryable: false,
        correlationId: "cid-2",
      }),
    );

    render(<BackendStatus />);

    const label = await screen.findByText(/backend unavailable \(internal\)/i);
    expect(label).toHaveAttribute("title", "Safe message.");
  });
});

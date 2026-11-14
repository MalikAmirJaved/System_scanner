import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getDiagnostics, type DiagnosticsReport } from "../services/diagnostics";
import { IpcError } from "../services/ipc";
import { Diagnostics } from "./Diagnostics";

vi.mock("../services/diagnostics", () => ({ getDiagnostics: vi.fn() }));

const mockedGetDiagnostics = vi.mocked(getDiagnostics);

const report: DiagnosticsReport = {
  generatedAt: 1_700_000_000_000,
  schemaVersion: 1,
  appVersion: "0.1.0",
  tauriVersion: "2.12.5",
  os: {
    name: "Windows 11 Pro",
    version: "10.0.26100",
    build: "26100",
    arch: "x64",
    elevated: false,
  },
  log: { sizeBytes: 1_234, fileCount: 2, modifiedAt: 1_700_000_000_000 },
  bundle: {
    preview: "System Scanner support bundle\nLog tail: <path>",
    redactions: 2,
    logTruncated: true,
    includedLogBytes: 262_144,
  },
  unavailable: [],
};

describe("Diagnostics", () => {
  beforeEach(() => {
    mockedGetDiagnostics.mockReset();
  });

  it("waits for the user before reading logs and then shows the preview", async () => {
    mockedGetDiagnostics.mockResolvedValue(report);

    render(<Diagnostics />);

    expect(screen.queryByLabelText(/redacted support bundle preview/i)).not.toBeInTheDocument();
    expect(mockedGetDiagnostics).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /generate bundle/i }));

    expect(
      await screen.findByText(/windows 11 pro 10\.0\.26100 build 26100 · x64/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/1\.2 KB across 2 files/i)).toBeInTheDocument();
    expect(screen.getByText("Unelevated")).toBeInTheDocument();
    expect(screen.getByLabelText(/redacted support bundle preview/i)).toHaveValue(
      "System Scanner support bundle\nLog tail: <path>",
    );
    expect(screen.getByText(/2 replacements made before preview/i)).toBeInTheDocument();
    expect(screen.getByText(/preview limited to the last 256\.0 KB/i)).toBeInTheDocument();
    expect(mockedGetDiagnostics).toHaveBeenCalledTimes(1);
  });

  it("reports the error code and retries", async () => {
    mockedGetDiagnostics
      .mockRejectedValueOnce(
        new IpcError({
          code: "internal",
          message: "The report could not be built.",
          retryable: false,
          correlationId: "cid-9",
        }),
      )
      .mockResolvedValueOnce(report);

    render(<Diagnostics />);

    fireEvent.click(screen.getByRole("button", { name: /generate bundle/i }));

    expect(
      await screen.findByText(/diagnostics are unavailable \(internal\)/i),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /try again/i }));

    expect(await screen.findByText(/windows 11 pro/i)).toBeInTheDocument();
  });

  it("copies the preview and confirms it", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    mockedGetDiagnostics.mockResolvedValue(report);

    render(<Diagnostics />);

    fireEvent.click(screen.getByRole("button", { name: /generate bundle/i }));
    fireEvent.click(await screen.findByRole("button", { name: /copy preview/i }));

    expect(await screen.findByText(/^copied$/i)).toBeInTheDocument();
    expect(writeText).toHaveBeenCalledWith(report.bundle.preview);
  });
});

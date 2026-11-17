import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { IpcError } from "../services/ipc";
import {
  cancelScan,
  getScanStatus,
  startScan,
  subscribeToScan,
  type ScanCounters,
  type ScanEvent,
  type ScanStatus,
} from "../services/scan";
import { ScanPanel } from "./ScanPanel";
import { IpcContractError } from "../services/validate";

vi.mock("../services/scan", () => ({
  startScan: vi.fn(),
  cancelScan: vi.fn(),
  getScanStatus: vi.fn(),
  subscribeToScan: vi.fn(),
}));

const mockedStart = vi.mocked(startScan);
const mockedCancel = vi.mocked(cancelScan);
const mockedStatus = vi.mocked(getScanStatus);
const mockedSubscribe = vi.mocked(subscribeToScan);

const counters: ScanCounters = {
  entries: 12,
  files: 9,
  directories: 3,
  reparsePoints: 0,
  bytes: 2048,
  skipped: 1,
  denied: 2,
  disappeared: 0,
  unsupported: 0,
  failed: 1,
};

let emit: (event: ScanEvent) => void;
let reportInvalid: (error: IpcContractError) => void;
const stop = vi.fn();

function notFound() {
  return new IpcError({
    code: "not_found",
    message: "The requested item no longer exists.",
    retryable: true,
    correlationId: "cid",
  });
}

function status(overrides: Partial<ScanStatus>): ScanStatus {
  return {
    jobId: "job-1",
    state: "running",
    seq: 0,
    elapsedMs: 0,
    counters,
    ...overrides,
  };
}

async function renderPanel() {
  render(<ScanPanel />);
  await waitFor(() => expect(mockedSubscribe).toHaveBeenCalled());
}

function typeRoots(text: string) {
  fireEvent.change(screen.getByLabelText(/folders to scan/i), { target: { value: text } });
}

describe("ScanPanel", () => {
  beforeEach(() => {
    stop.mockReset();
    mockedStart.mockReset();
    mockedCancel.mockReset();
    mockedStatus.mockReset();
    mockedSubscribe.mockReset();
    mockedSubscribe.mockImplementation(async (handler, onInvalid) => {
      emit = (event) => act(() => handler(event));
      reportInvalid = (error) => act(() => onInvalid?.(error));
      return stop;
    });
    mockedStatus.mockRejectedValue(notFound());
  });

  it("starts idle and only enables Start once a folder is entered", async () => {
    await renderPanel();

    const start = screen.getByRole("button", { name: /start scan/i });
    expect(start).toBeDisabled();
    expect(screen.queryByRole("button", { name: /cancel/i })).not.toBeInTheDocument();

    typeRoots("  C:\\data  \n\n");
    expect(start).toBeEnabled();
    expect(screen.getByText("1 folder selected")).toBeInTheDocument();
  });

  it("sends trimmed roots, shows live counts, and ends with a partial verdict", async () => {
    mockedStart.mockResolvedValue({ jobId: "job-1" });
    mockedStatus.mockReset();
    mockedStatus
      .mockRejectedValueOnce(notFound())
      .mockResolvedValueOnce(status({ state: "running", seq: 0 }));
    await renderPanel();

    typeRoots("C:\\data\n D:\\more ");
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    expect(await screen.findByText("Scanning…")).toBeInTheDocument();
    expect(mockedStart).toHaveBeenCalledWith(["C:\\data", "D:\\more"]);

    emit({
      kind: "progress",
      payload: {
        jobId: "job-1",
        seq: 1,
        phase: "traverse",
        filesSeen: 1500,
        bytesSeen: 5_242_880,
        skipped: 0,
        elapsedMs: 2500,
      },
    });
    expect(await screen.findByText("1,500")).toBeInTheDocument();
    expect(screen.getByText("5.0 MB")).toBeInTheDocument();
    expect(screen.getByText("2.5 s")).toBeInTheDocument();

    emit({
      kind: "completed",
      payload: {
        jobId: "job-1",
        summary: counters,
        completeness: "partial",
        partial: true,
        elapsedMs: 4000,
      },
    });

    expect(await screen.findByText("Partial scan finished.")).toBeInTheDocument();
    expect(screen.getByText(/understate the tree/i)).toBeInTheDocument();
    expect(screen.getByText("Denied")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /start scan/i })).toBeEnabled();
  });

  it("cancels the current job and shows the cancelled counts", async () => {
    mockedStatus.mockReset();
    mockedStatus.mockResolvedValue(status({ state: "running" }));
    mockedCancel.mockResolvedValue({ accepted: true });
    await renderPanel();

    fireEvent.click(await screen.findByRole("button", { name: /^cancel$/i }));

    await waitFor(() => expect(mockedCancel).toHaveBeenCalledWith("job-1"));
    emit({
      kind: "cancelled",
      payload: { jobId: "job-1", reason: "user_requested", summary: counters, elapsedMs: 900 },
    });

    expect(await screen.findByText("Cancelled scan finished.")).toBeInTheDocument();
    expect(screen.getByText(/counts reached so far/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /cancel/i })).not.toBeInTheDocument();
  });

  it("recovers a running scan from the authoritative status on mount", async () => {
    mockedStatus.mockReset();
    mockedStatus.mockResolvedValue(status({ state: "running", seq: 4 }));
    await renderPanel();

    expect(await screen.findByText("Scanning…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /start scan/i })).toBeDisabled();
    expect(screen.getByLabelText(/folders to scan/i)).toBeDisabled();
  });

  it("reports busy with its code and leaves the panel idle", async () => {
    mockedStart.mockRejectedValue(
      new IpcError({
        code: "busy",
        message: "Another operation is already using this resource.",
        retryable: true,
        correlationId: "cid",
      }),
    );
    await renderPanel();

    typeRoots("C:\\data");
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/scan unavailable \(busy\)/i);
    expect(screen.queryByText("Scanning…")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /start scan/i })).toBeEnabled();
  });

  it("shows the backend's validation detail for a refused root", async () => {
    mockedStart.mockRejectedValue(
      new IpcError({
        code: "invalid_input",
        message: "The request was invalid.",
        detail: "device and NT-namespace paths are not allowed",
        retryable: false,
        correlationId: "cid",
      }),
    );
    await renderPanel();

    typeRoots("\\\\.\\PhysicalDrive0");
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /device and nt-namespace paths are not allowed/i,
    );
  });

  it("ignores events that belong to another job", async () => {
    mockedStatus.mockReset();
    mockedStatus.mockResolvedValue(status({ jobId: "job-1", state: "running" }));
    await renderPanel();
    expect(await screen.findByText("Scanning…")).toBeInTheDocument();

    emit({
      kind: "completed",
      payload: {
        jobId: "other-job",
        summary: counters,
        completeness: "complete",
        partial: false,
        elapsedMs: 1,
      },
    });

    expect(screen.getByText("Scanning…")).toBeInTheDocument();
    expect(screen.queryByText(/scan finished/i)).not.toBeInTheDocument();
  });

  it("reports a contract failure and re-reads the authoritative status", async () => {
    mockedStatus.mockReset();
    mockedStatus.mockResolvedValue(status({ state: "running" }));
    await renderPanel();
    expect(await screen.findByText("Scanning…")).toBeInTheDocument();
    expect(mockedStatus).toHaveBeenCalledTimes(1);

    reportInvalid(new IpcContractError("scan:completed", "completeness is not one of complete"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/contract_violation/i);
    expect(screen.getByRole("alert")).toHaveTextContent(/scan:completed/);
    await waitFor(() => expect(mockedStatus).toHaveBeenCalledTimes(2));
    // The scan is still running as far as the backend is concerned.
    expect(screen.getByText("Scanning…")).toBeInTheDocument();
  });

  it("shows a failed scan as having no result and unsubscribes on unmount", async () => {
    mockedStatus.mockReset();
    mockedStatus.mockResolvedValue(status({ state: "running" }));
    const { unmount } = render(<ScanPanel />);
    await screen.findByText("Scanning…");

    emit({ kind: "failed", payload: { jobId: "job-1", errorCode: "internal", message: "m" } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/failed \(internal\)/i);
    expect(screen.queryByText("Size seen")).not.toBeInTheDocument();

    unmount();
    expect(stop).toHaveBeenCalledTimes(1);
  });
});

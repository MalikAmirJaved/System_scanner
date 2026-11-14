import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { IpcError } from "../services/ipc";
import { listProcesses, type ProcessList as ProcessPage } from "../services/processes";
import { ProcessList as ProcessPanel } from "./ProcessList";

vi.mock("../services/processes", () => ({ listProcesses: vi.fn() }));

const mockedListProcesses = vi.mocked(listProcesses);

const page: ProcessPage = {
  capturedAt: 1762387200000,
  schemaVersion: 1,
  total: 312,
  offset: 0,
  limit: 100,
  truncated: false,
  items: [
    {
      key: "4-1700000000000",
      pid: 4,
      parentPid: 0,
      name: "System",
      threadCount: 120,
      startTime: 1_700_000_000_000,
      workingSetBytes: 1_048_576,
      unavailable: [],
    },
    {
      key: "812-1700000001000",
      pid: 812,
      parentPid: 4,
      name: "example.exe",
      threadCount: 8,
      startTime: 1_700_000_001_000,
      workingSetBytes: 2_097_152,
      unavailable: [],
    },
  ],
};

describe("ProcessList", () => {
  beforeEach(() => {
    mockedListProcesses.mockReset();
  });

  it("renders rows, the paged range, and requests the next offset", async () => {
    mockedListProcesses.mockResolvedValue(page);

    render(<ProcessPanel />);

    expect(await screen.findByText("example.exe")).toBeInTheDocument();
    expect(screen.getByText("812")).toBeInTheDocument();
    expect(screen.getByRole("caption")).toHaveTextContent(/1–2 of 312/);
    expect(screen.getByRole("button", { name: /previous/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /next/i })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: /next/i }));

    expect(mockedListProcesses).toHaveBeenCalledWith({ offset: 100, limit: 100 });
  });

  it("marks denied fields instead of dropping the row", async () => {
    mockedListProcesses.mockResolvedValue({
      ...page,
      total: 1,
      items: [
        {
          key: "pid:4",
          pid: 4,
          parentPid: 0,
          name: "System",
          unavailable: [
            { field: "items[0].startTime", reason: "access_denied" },
            { field: "items[0].workingSetBytes", reason: "access_denied" },
          ],
        },
      ],
    });

    render(<ProcessPanel />);

    expect(await screen.findByText("System")).toBeInTheDocument();
    expect(screen.getAllByText(/unavailable \(access_denied\)/i)).toHaveLength(2);
    expect(screen.getByText(/rows marked unavailable were denied by windows/i)).toBeInTheDocument();
  });

  it("reports a failure and recovers on refresh", async () => {
    mockedListProcesses
      .mockRejectedValueOnce(
        new IpcError({
          code: "access_denied",
          message: "Windows denied access to the requested resource.",
          retryable: true,
          correlationId: "cid-1",
        }),
      )
      .mockResolvedValueOnce(page);

    render(<ProcessPanel />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /process list is unavailable \(access_denied\)/i,
    );

    fireEvent.click(screen.getByRole("button", { name: /refresh/i }));

    expect(await screen.findByText("example.exe")).toBeInTheDocument();
    expect(mockedListProcesses).toHaveBeenCalledTimes(2);
  });

  it("says so when the list was truncated", async () => {
    mockedListProcesses.mockResolvedValue({ ...page, total: 8192, truncated: true });

    render(<ProcessPanel />);

    expect(await screen.findByText(/the list hit its safety cap/i)).toBeInTheDocument();
    expect(screen.getByRole("caption")).toHaveTextContent(/of 8,192\+/);
  });
});

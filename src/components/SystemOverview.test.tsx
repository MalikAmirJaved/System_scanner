import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getSystemSnapshot, type SystemSnapshot } from "../services/system";
import { IpcError } from "../services/ipc";
import { SystemOverview } from "./SystemOverview";

vi.mock("../services/system", () => ({ getSystemSnapshot: vi.fn() }));

const mockedGetSystemSnapshot = vi.mocked(getSystemSnapshot);

const snapshot: SystemSnapshot = {
  capturedAt: 1762387200000,
  schemaVersion: 1,
  os: {
    name: "Windows 11 Pro",
    edition: "Professional",
    version: "10.0.26100",
    arch: "x64",
  },
  cpu: { model: "Example CPU", logicalCores: 16, physicalCores: 8 },
  memory: { totalBytes: 34_359_738_368, availableBytes: 12_884_901_888 },
  disks: [
    {
      mount: "C:\\",
      kind: "fixed",
      freeBytes: 536_870_912_000,
      totalBytes: 1_073_741_824_000,
    },
  ],
  network: [{ name: "Ethernet", isUp: true, addresses: ["192.0.2.10"] }],
  unavailable: [],
};

describe("SystemOverview", () => {
  beforeEach(() => {
    mockedGetSystemSnapshot.mockReset();
  });

  it("renders collected OS, CPU, memory, disk, and adapter data", async () => {
    mockedGetSystemSnapshot.mockResolvedValue(snapshot);

    render(<SystemOverview />);

    expect(await screen.findByText(/windows 11 pro/i)).toBeInTheDocument();
    expect(screen.getByText(/example cpu/i)).toBeInTheDocument();
    expect(screen.getByText(/12\.0 GB free of 32\.0 GB/i)).toBeInTheDocument();
    expect(screen.getByText("C:\\")).toBeInTheDocument();
    expect(screen.getByText(/192\.0\.2\.10/)).toBeInTheDocument();
    expect(screen.getByText(/read-only snapshot · updated/i)).toBeInTheDocument();
  });

  it("shows why a group is missing instead of an empty value", async () => {
    mockedGetSystemSnapshot.mockResolvedValue({
      ...snapshot,
      memory: undefined,
      unavailable: [{ field: "memory", reason: "access_denied" }],
    });

    render(<SystemOverview />);

    expect(await screen.findByText(/unavailable \(access_denied\)/i)).toBeInTheDocument();
  });

  it("reports a failure and recovers on refresh", async () => {
    mockedGetSystemSnapshot
      .mockRejectedValueOnce(
        new IpcError({
          code: "access_denied",
          message: "Windows denied access.",
          retryable: true,
          correlationId: "cid-1",
        }),
      )
      .mockResolvedValueOnce(snapshot);

    render(<SystemOverview />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /system information is unavailable \(access_denied\)/i,
    );

    fireEvent.click(screen.getByRole("button", { name: /refresh/i }));

    expect(await screen.findByText(/windows 11 pro/i)).toBeInTheDocument();
    expect(mockedGetSystemSnapshot).toHaveBeenCalledTimes(2);
  });
});

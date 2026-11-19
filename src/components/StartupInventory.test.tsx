import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { IpcError } from "../services/ipc";
import { getStartupInventory, type StartupInventory as StartupData } from "../services/startup";
import { StartupInventory as StartupPanel } from "./StartupInventory";

vi.mock("../services/startup", () => ({ getStartupInventory: vi.fn() }));

const mockedGetStartupInventory = vi.mocked(getStartupInventory);

const inventory: StartupData = {
  capturedAt: 1762387200000,
  schemaVersion: 1,
  services: [
    {
      name: "Dhcp",
      displayName: "DHCP Client",
      state: "running",
      startType: "automatic",
      binaryPath: "C:\\Windows\\System32\\svchost.exe -k LocalServiceNetworkRestricted",
      unavailable: [],
    },
    {
      name: "SuspiciousService",
      state: "stopped",
      unavailable: [
        { field: "services[1].startType", reason: "access_denied" },
        { field: "services[1].binaryPath", reason: "access_denied" },
      ],
    },
  ],
  startup: [
    {
      scope: "current_user",
      source: "registry_run",
      entries: [{ name: "Update", command: "C:\\bin\\update.exe --tray" }],
    },
    {
      scope: "all_users",
      source: "startup_folder",
      entries: [],
    },
  ],
  servicesTruncated: false,
  startupTruncated: false,
  unavailable: [
    { field: "startup[2].entries", reason: "access_denied" },
    { field: "services[1].startType", reason: "access_denied" },
    { field: "services[1].binaryPath", reason: "access_denied" },
  ],
};

describe("StartupInventory", () => {
  beforeEach(() => {
    mockedGetStartupInventory.mockReset();
  });

  it("renders services and startup entries with their sources", async () => {
    mockedGetStartupInventory.mockResolvedValue(inventory);

    render(<StartupPanel />);

    expect(await screen.findByText("Dhcp")).toBeInTheDocument();
    expect(screen.getByText("DHCP Client")).toBeInTheDocument();
    expect(screen.getByText("running")).toBeInTheDocument();
    expect(
      screen.getByText("C:\\Windows\\System32\\svchost.exe -k LocalServiceNetworkRestricted"),
    ).toBeInTheDocument();

    expect(screen.getByText("Current user · Registry · Run")).toBeInTheDocument();
    expect(screen.getByText("Update")).toBeInTheDocument();
    expect(screen.getByText("C:\\bin\\update.exe --tray")).toBeInTheDocument();
    expect(screen.getByText("No entries.")).toBeInTheDocument();
    expect(screen.getByText(/2 services/)).toBeInTheDocument();
    expect(screen.getByText(/1 entries/)).toBeInTheDocument();
  });

  it("marks a denied service configuration instead of hiding the row", async () => {
    mockedGetStartupInventory.mockResolvedValue(inventory);

    render(<StartupPanel />);

    expect(await screen.findByText("SuspiciousService")).toBeInTheDocument();
    // Start type and binary path, both denied.
    expect(screen.getAllByText(/unavailable \(access_denied\)/i)).toHaveLength(2);
    expect(screen.getByText(/rows marked unavailable were denied by windows/i)).toBeInTheDocument();
  });

  it("does not draw a source the backend never sent", async () => {
    mockedGetStartupInventory.mockResolvedValue({
      ...inventory,
      startup: [inventory.startup[0]],
    });

    render(<StartupPanel />);

    expect(await screen.findByText("Registry · Run", { exact: false })).toBeInTheDocument();
    expect(screen.queryByText(/startup folder/i)).not.toBeInTheDocument();
  });

  it("reports a failure and recovers on refresh", async () => {
    mockedGetStartupInventory
      .mockRejectedValueOnce(
        new IpcError({
          code: "access_denied",
          message: "Windows denied access to the requested resource.",
          retryable: true,
          correlationId: "cid-1",
        }),
      )
      .mockResolvedValueOnce(inventory);

    render(<StartupPanel />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /startup and service inventory is unavailable \(access_denied\)/i,
    );

    fireEvent.click(screen.getByRole("button", { name: /refresh/i }));

    expect(await screen.findByText("Dhcp")).toBeInTheDocument();
    expect(mockedGetStartupInventory).toHaveBeenCalledTimes(2);
  });

  it("says so when the service database itself is denied", async () => {
    mockedGetStartupInventory.mockResolvedValue({
      ...inventory,
      services: undefined,
      unavailable: [{ field: "services", reason: "access_denied" }],
    });

    render(<StartupPanel />);

    expect(
      await screen.findByText(/the service database is unavailable \(access_denied\)/i),
    ).toBeInTheDocument();
  });
});

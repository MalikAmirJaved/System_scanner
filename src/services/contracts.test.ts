import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { getAppInfo } from "./app";
import { getDiagnostics } from "./diagnostics";
import { listProcesses } from "./processes";
import { cancelScan, getScanStatus, startScan } from "./scan";
import { getStartupInventory } from "./startup";
import { getSystemSnapshot } from "./system";
import { IpcContractError } from "./validate";

/**
 * End-to-end shape checks for every implemented command: one payload that
 * matches `API.md`, and one drifted payload per command that must fail loudly.
 */

const ok = (data: unknown) => ({ status: "ok", data });

const appInfo = {
  appName: "system-scanner",
  appVersion: "0.1.0",
  tauriVersion: "2.12.5",
  os: "windows",
  arch: "x86_64",
  debug: false,
  schemaVersion: 1,
};

const counters = {
  entries: 12,
  files: 9,
  directories: 3,
  reparsePoints: 0,
  bytes: 2_048,
  skipped: 1,
  denied: 2,
  disappeared: 0,
  unsupported: 0,
  failed: 1,
};

const snapshot = {
  capturedAt: 1_762_387_200_000,
  schemaVersion: 1,
  os: { name: "Windows 11 Pro", edition: "Professional", arch: "x64" },
  cpu: { model: "Example CPU", logicalCores: 16, physicalCores: 8 },
  memory: { totalBytes: 34_359_738_368, availableBytes: 12_884_901_888 },
  disks: [{ mount: "C:\\", kind: "fixed", label: "System", totalBytes: 0, freeBytes: 0 }],
  network: [{ name: "Ethernet", isUp: true, addresses: ["192.0.2.10"] }],
  unavailable: [{ field: "cpu.model", reason: "unsupported" }],
};

const processList = {
  capturedAt: 1_762_387_200_000,
  schemaVersion: 2,
  total: 1,
  offset: 0,
  limit: 100,
  truncated: false,
  items: [
    {
      key: "4-1762380000000",
      pid: 4,
      parentPid: 0,
      depth: 0,
      name: "System",
      imagePath: "C:\\Windows\\System32\\ntoskrnl.exe",
      threadCount: 210,
      startTime: 1_762_380_000_000,
      workingSetBytes: 1_048_576,
      unavailable: [{ field: "items[0].startTime", reason: "access_denied" }],
    },
  ],
};

const startupInventory = {
  capturedAt: 1_762_387_200_000,
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
  ],
  startup: [
    {
      scope: "current_user",
      source: "registry_run",
      entries: [{ name: "Update", command: "C:\\bin\\update.exe --tray" }],
    },
  ],
  servicesTruncated: false,
  startupTruncated: false,
  unavailable: [],
};

const diagnostics = {
  generatedAt: 1_763_000_000_000,
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
  log: { sizeBytes: 1_234, fileCount: 2, modifiedAt: 1_763_000_000_000 },
  bundle: { preview: "bundle text", redactions: 4, logTruncated: true, includedLogBytes: 262_144 },
  unavailable: [],
};

const scanStatus = {
  jobId: "scan-1",
  state: "completed",
  seq: 7,
  elapsedMs: 4_000,
  counters,
  completeness: "partial",
};

describe("command response contracts", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("accepts the documented shape of every command and copies only known fields", async () => {
    invoke.mockResolvedValueOnce(ok({ ...appInfo, unexpected: "ignored" }));
    await expect(getAppInfo()).resolves.toEqual(appInfo);

    invoke.mockResolvedValueOnce(ok(snapshot));
    await expect(getSystemSnapshot()).resolves.toEqual(snapshot);

    invoke.mockResolvedValueOnce(ok(processList));
    await expect(listProcesses()).resolves.toEqual(processList);

    invoke.mockResolvedValueOnce(ok(startupInventory));
    await expect(getStartupInventory()).resolves.toEqual(startupInventory);

    invoke.mockResolvedValueOnce(ok(diagnostics));
    await expect(getDiagnostics()).resolves.toEqual(diagnostics);

    invoke.mockResolvedValueOnce(ok({ jobId: "scan-1" }));
    await expect(startScan(["C:\\data"])).resolves.toEqual({ jobId: "scan-1" });

    invoke.mockResolvedValueOnce(ok({ accepted: true }));
    await expect(cancelScan("scan-1")).resolves.toEqual({ accepted: true });

    invoke.mockResolvedValueOnce(ok(scanStatus));
    await expect(getScanStatus("scan-1")).resolves.toEqual(scanStatus);
  });

  it("accepts a status with its optional fields omitted", async () => {
    const withoutCompleteness = {
      jobId: scanStatus.jobId,
      state: scanStatus.state,
      seq: scanStatus.seq,
      elapsedMs: scanStatus.elapsedMs,
      counters: scanStatus.counters,
    };
    invoke.mockResolvedValue(ok(withoutCompleteness));

    await expect(getScanStatus()).resolves.toEqual(withoutCompleteness);
  });

  it.each([
    {
      command: "get_app_info",
      call: () => getAppInfo(),
      drifted: { ...appInfo, appVersion: undefined, app_version: "0.1.0" },
      problem: /appVersion is not a string/,
    },
    {
      command: "get_system_snapshot",
      call: () => getSystemSnapshot(),
      drifted: { ...snapshot, disks: { mount: "C:\\" } },
      problem: /disks is not an array/,
    },
    {
      command: "list_processes",
      call: () => listProcesses(),
      drifted: { ...processList, items: [{ ...processList.items[0], pid: -1 }] },
      problem: /items\[0\]\.pid is not a non-negative integer/,
    },
    {
      command: "get_startup_inventory",
      call: () => getStartupInventory(),
      drifted: {
        ...startupInventory,
        services: [{ ...startupInventory.services[0], state: "crashed" }],
      },
      problem: /services\[0\]\.state is not one of/,
    },
    {
      command: "get_diagnostics",
      call: () => getDiagnostics(),
      drifted: { ...diagnostics, bundle: { ...diagnostics.bundle, preview: 42 } },
      problem: /bundle\.preview is not a string/,
    },
    {
      command: "start_scan",
      call: () => startScan(["C:\\data"]),
      drifted: { job_id: "scan-1" },
      problem: /jobId is not a string/,
    },
    {
      command: "cancel_scan",
      call: () => cancelScan(),
      drifted: { accepted: "yes" },
      problem: /accepted is not a boolean/,
    },
    {
      command: "get_scan_status",
      call: () => getScanStatus(),
      drifted: { ...scanStatus, state: "stopped" },
      problem: /state is not one of/,
    },
  ])("$command rejects a drifted payload", async ({ command, call, drifted, problem }) => {
    invoke.mockResolvedValue(ok(drifted));

    const error = await call().catch((cause: unknown) => cause);

    expect(error).toBeInstanceOf(IpcContractError);
    expect(error).toMatchObject({ source: command });
    expect((error as IpcContractError).problem).toMatch(problem);
  });

  it("rejects a schema version this build does not read", async () => {
    invoke.mockResolvedValue(ok({ ...snapshot, schemaVersion: 2 }));

    await expect(getSystemSnapshot()).rejects.toThrow(
      /schemaVersion is version 2, but this build reads version 1/,
    );
  });

  it("rejects a page with more rows than the backend can return", async () => {
    const items = Array.from({ length: 501 }, () => processList.items[0]);
    invoke.mockResolvedValue(ok({ ...processList, total: 501, items }));

    await expect(listProcesses({ limit: 500 })).rejects.toThrow(/at most 500 items/);
  });

  it("rejects a process row without its unavailable list", async () => {
    const row = {
      key: "4-1762380000000",
      pid: 4,
      parentPid: 0,
      depth: 0,
      name: "System",
    };
    invoke.mockResolvedValue(ok({ ...processList, items: [row] }));

    await expect(listProcesses()).rejects.toThrow(/items\[0\]\.unavailable is not an array/);
  });
});

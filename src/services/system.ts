import { invokeCommand } from "./ipc";

/**
 * Mirrors the `get_system_snapshot` DTO in
 * `src-tauri/src/dto/system_snapshot.rs` (`SNAPSHOT_SCHEMA_VERSION = 1`).
 *
 * A missing group or field is always paired with an entry in `unavailable`;
 * `undefined` alone never means "could not read".
 */
export type UnavailableReason = "access_denied" | "unsupported" | "failed";

export interface UnavailableField {
  field: string;
  reason: UnavailableReason;
}

export interface OsInfo {
  name?: string;
  edition?: string;
  version?: string;
  build?: string;
  arch: string;
}

export interface CpuInfo {
  model?: string;
  logicalCores?: number;
  physicalCores?: number;
}

export interface MemoryInfo {
  totalBytes: number;
  availableBytes: number;
}

export interface DiskInfo {
  mount: string;
  kind: string;
  label?: string;
  filesystem?: string;
  totalBytes?: number;
  freeBytes?: number;
}

export interface NetworkAdapterInfo {
  name: string;
  description?: string;
  isUp: boolean;
  addresses: string[];
}

export interface SystemSnapshot {
  capturedAt: number;
  schemaVersion: number;
  os?: OsInfo;
  cpu?: CpuInfo;
  memory?: MemoryInfo;
  disks?: DiskInfo[];
  network?: NetworkAdapterInfo[];
  unavailable: UnavailableField[];
}

export function getSystemSnapshot(): Promise<SystemSnapshot> {
  return invokeCommand<SystemSnapshot>("get_system_snapshot");
}

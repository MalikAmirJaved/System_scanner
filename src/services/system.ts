import { invokeCommand } from "./ipc";
import {
  at,
  boolean,
  count,
  listOf,
  object,
  oneOf,
  optional,
  optionalCount,
  optionalListOf,
  optionalString,
  string,
  version,
  type Validator,
} from "./validate";

/**
 * Mirrors the `get_system_snapshot` DTO in
 * `src-tauri/src/dto/system_snapshot.rs` (`SNAPSHOT_SCHEMA_VERSION = 1`).
 *
 * A missing group or field is always paired with an entry in `unavailable`;
 * `undefined` alone never means "could not read".
 */
export const SNAPSHOT_SCHEMA_VERSION = 1;

export type UnavailableReason = "access_denied" | "unsupported" | "failed";

const UNAVAILABLE_REASONS = ["access_denied", "unsupported", "failed"] as const;

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

/**
 * Caps on collections the backend builds from OS state. They are far above any
 * real machine, so exceeding one means a broken payload, not a big machine.
 */
const MAX_DISKS = 256;
const MAX_ADAPTERS = 256;
const MAX_ADDRESSES_PER_ADAPTER = 128;
const MAX_UNAVAILABLE = 2_048;

const parseUnavailableField: Validator<UnavailableField> = (value, path) => {
  const raw = object(value, path);

  return {
    field: string(raw.field, at(path, "field")),
    reason: oneOf(UNAVAILABLE_REASONS, raw.reason, at(path, "reason")),
  };
};

const parseOs: Validator<OsInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    name: optionalString(raw.name, at(path, "name")),
    edition: optionalString(raw.edition, at(path, "edition")),
    version: optionalString(raw.version, at(path, "version")),
    build: optionalString(raw.build, at(path, "build")),
    arch: string(raw.arch, at(path, "arch")),
  };
};

const parseCpu: Validator<CpuInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    model: optionalString(raw.model, at(path, "model")),
    logicalCores: optionalCount(raw.logicalCores, at(path, "logicalCores")),
    physicalCores: optionalCount(raw.physicalCores, at(path, "physicalCores")),
  };
};

const parseMemory: Validator<MemoryInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    totalBytes: count(raw.totalBytes, at(path, "totalBytes")),
    availableBytes: count(raw.availableBytes, at(path, "availableBytes")),
  };
};

const parseDisk: Validator<DiskInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    mount: string(raw.mount, at(path, "mount")),
    kind: string(raw.kind, at(path, "kind")),
    label: optionalString(raw.label, at(path, "label")),
    filesystem: optionalString(raw.filesystem, at(path, "filesystem")),
    totalBytes: optionalCount(raw.totalBytes, at(path, "totalBytes")),
    freeBytes: optionalCount(raw.freeBytes, at(path, "freeBytes")),
  };
};

const parseAdapter: Validator<NetworkAdapterInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    name: string(raw.name, at(path, "name")),
    description: optionalString(raw.description, at(path, "description")),
    isUp: boolean(raw.isUp, at(path, "isUp")),
    addresses: listOf(raw.addresses, at(path, "addresses"), MAX_ADDRESSES_PER_ADAPTER, string),
  };
};

const parseSystemSnapshot: Validator<SystemSnapshot> = (value, path) => {
  const raw = object(value, path);

  return {
    capturedAt: count(raw.capturedAt, at(path, "capturedAt")),
    schemaVersion: version(SNAPSHOT_SCHEMA_VERSION, raw.schemaVersion, at(path, "schemaVersion")),
    os: optional(raw.os, at(path, "os"), parseOs),
    cpu: optional(raw.cpu, at(path, "cpu"), parseCpu),
    memory: optional(raw.memory, at(path, "memory"), parseMemory),
    disks: optionalListOf(raw.disks, at(path, "disks"), MAX_DISKS, parseDisk),
    network: optionalListOf(raw.network, at(path, "network"), MAX_ADAPTERS, parseAdapter),
    unavailable: listOf(
      raw.unavailable,
      at(path, "unavailable"),
      MAX_UNAVAILABLE,
      parseUnavailableField,
    ),
  };
};

export function getSystemSnapshot(): Promise<SystemSnapshot> {
  return invokeCommand("get_system_snapshot", parseSystemSnapshot);
}

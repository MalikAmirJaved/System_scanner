import { invokeCommand } from "./ipc";
import {
  at,
  boolean,
  count,
  listOf,
  object,
  oneOf,
  optional,
  optionalListOf,
  optionalString,
  string,
  version,
  type Validator,
} from "./validate";
import { parseUnavailableField, type UnavailableField } from "./unavailable";

/**
 * Mirrors the `get_startup_inventory` DTO in
 * `src-tauri/src/dto/startup_inventory.rs`
 * (`STARTUP_INVENTORY_SCHEMA_VERSION = 1`).
 *
 * A missing group or field is always paired with an entry in `unavailable`;
 * `undefined` alone never means "could not read".
 */
export const STARTUP_INVENTORY_SCHEMA_VERSION = 1;

export type ServiceState =
  | "running"
  | "stopped"
  | "start_pending"
  | "stop_pending"
  | "continue_pending"
  | "pause_pending"
  | "paused"
  | "unknown";

export type ServiceStartType = "boot" | "system" | "automatic" | "manual" | "disabled" | "unknown";

export type StartupScope = "current_user" | "all_users";

export type StartupSource = "registry_run" | "registry_run_once" | "startup_folder";

const SERVICE_STATES = [
  "running",
  "stopped",
  "start_pending",
  "stop_pending",
  "continue_pending",
  "pause_pending",
  "paused",
  "unknown",
] as const;

const SERVICE_START_TYPES = [
  "boot",
  "system",
  "automatic",
  "manual",
  "disabled",
  "unknown",
] as const;

const STARTUP_SCOPES = ["current_user", "all_users"] as const;

const STARTUP_SOURCES = ["registry_run", "registry_run_once", "startup_folder"] as const;

export interface ServiceInfo {
  /** SCM key name; the identity of the row. */
  name: string;
  displayName?: string;
  state: ServiceState;
  startType?: ServiceStartType;
  /** Configured binary path. Local to this machine; never logged or bundled. */
  binaryPath?: string;
  unavailable: UnavailableField[];
}

export interface StartupEntryInfo {
  /** Registry value name, or the file name of a startup-folder item. */
  name: string;
  /** Registry value data, or the item's own path. Local to this machine. */
  command?: string;
}

export interface StartupSourceInfo {
  scope: StartupScope;
  source: StartupSource;
  /** Absent when this source was refused; the reason is in `unavailable`. */
  entries?: StartupEntryInfo[];
}

export interface StartupInventory {
  capturedAt: number;
  schemaVersion: number;
  services?: ServiceInfo[];
  startup: StartupSourceInfo[];
  servicesTruncated: boolean;
  startupTruncated: boolean;
  unavailable: UnavailableField[];
}

/**
 * Caps on collections the backend builds from OS state. They are far above any
 * real machine, so exceeding one means a broken payload, not a big machine.
 */
const MAX_SERVICES = 4_096;
const MAX_STARTUP_SOURCES = 8;
const MAX_ENTRIES_PER_SOURCE = 512;
const MAX_ROW_UNAVAILABLE = 64;
const MAX_UNAVAILABLE = 8_192;

const parseService: Validator<ServiceInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    name: string(raw.name, at(path, "name")),
    displayName: optionalString(raw.displayName, at(path, "displayName")),
    state: oneOf(SERVICE_STATES, raw.state, at(path, "state")),
    startType: optional(raw.startType, at(path, "startType"), (item, itemPath) =>
      oneOf(SERVICE_START_TYPES, item, itemPath),
    ),
    binaryPath: optionalString(raw.binaryPath, at(path, "binaryPath")),
    unavailable: listOf(
      raw.unavailable,
      at(path, "unavailable"),
      MAX_ROW_UNAVAILABLE,
      parseUnavailableField,
    ),
  };
};

const parseStartupEntry: Validator<StartupEntryInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    name: string(raw.name, at(path, "name")),
    command: optionalString(raw.command, at(path, "command")),
  };
};

const parseStartupSource: Validator<StartupSourceInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    scope: oneOf(STARTUP_SCOPES, raw.scope, at(path, "scope")),
    source: oneOf(STARTUP_SOURCES, raw.source, at(path, "source")),
    entries: optionalListOf(
      raw.entries,
      at(path, "entries"),
      MAX_ENTRIES_PER_SOURCE,
      parseStartupEntry,
    ),
  };
};

const parseStartupInventory: Validator<StartupInventory> = (value, path) => {
  const raw = object(value, path);

  return {
    capturedAt: count(raw.capturedAt, at(path, "capturedAt")),
    schemaVersion: version(
      STARTUP_INVENTORY_SCHEMA_VERSION,
      raw.schemaVersion,
      at(path, "schemaVersion"),
    ),
    services: optionalListOf(raw.services, at(path, "services"), MAX_SERVICES, parseService),
    startup: listOf(raw.startup, at(path, "startup"), MAX_STARTUP_SOURCES, parseStartupSource),
    servicesTruncated: boolean(raw.servicesTruncated, at(path, "servicesTruncated")),
    startupTruncated: boolean(raw.startupTruncated, at(path, "startupTruncated")),
    unavailable: listOf(
      raw.unavailable,
      at(path, "unavailable"),
      MAX_UNAVAILABLE,
      parseUnavailableField,
    ),
  };
};

export function getStartupInventory(): Promise<StartupInventory> {
  return invokeCommand("get_startup_inventory", parseStartupInventory);
}

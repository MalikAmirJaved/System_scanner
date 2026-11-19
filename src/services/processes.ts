import { invokeCommand } from "./ipc";
import {
  at,
  boolean,
  count,
  listOf,
  object,
  optionalCount,
  optionalString,
  string,
  version,
  type Validator,
} from "./validate";
import { parseUnavailableField, type UnavailableField } from "./unavailable";

/**
 * Mirrors the `list_processes` DTO in
 * `src-tauri/src/dto/process_list.rs` (`PROCESS_LIST_SCHEMA_VERSION = 2`).
 *
 * A missing field is always paired with an entry in the row's `unavailable`
 * list; `undefined` alone never means "could not read".
 */
export const PROCESS_LIST_SCHEMA_VERSION = 2;

export interface ProcessInfo {
  /** Stable across refreshes unless the start time was unreadable (`pid:<pid>`). */
  key: string;
  pid: number;
  parentPid: number;
  /** Ancestors above this process; `0` for a root or an unresolved parent. */
  depth: number;
  name?: string;
  /** Full image path. Local to this machine; never logged or bundled. */
  imagePath?: string;
  threadCount?: number;
  startTime?: number;
  workingSetBytes?: number;
  unavailable: UnavailableField[];
}

export interface ProcessList {
  capturedAt: number;
  schemaVersion: number;
  /** Processes enumerated, before paging. A lower bound when `truncated`. */
  total: number;
  offset: number;
  limit: number;
  truncated: boolean;
  items: ProcessInfo[];
}

export interface ProcessPageOptions {
  offset?: number;
  limit?: number;
}

/**
 * Rust caps `limit` at 500 and its adapter at 8,192 processes, so one page can
 * never legitimately hold more than 500 rows.
 */
const MAX_PAGE_ITEMS = 500;
const MAX_ROW_UNAVAILABLE = 64;

const parseProcess: Validator<ProcessInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    key: string(raw.key, at(path, "key")),
    pid: count(raw.pid, at(path, "pid")),
    parentPid: count(raw.parentPid, at(path, "parentPid")),
    depth: count(raw.depth, at(path, "depth")),
    name: optionalString(raw.name, at(path, "name")),
    imagePath: optionalString(raw.imagePath, at(path, "imagePath")),
    threadCount: optionalCount(raw.threadCount, at(path, "threadCount")),
    startTime: optionalCount(raw.startTime, at(path, "startTime")),
    workingSetBytes: optionalCount(raw.workingSetBytes, at(path, "workingSetBytes")),
    unavailable: listOf(
      raw.unavailable,
      at(path, "unavailable"),
      MAX_ROW_UNAVAILABLE,
      parseUnavailableField,
    ),
  };
};

const parseProcessList: Validator<ProcessList> = (value, path) => {
  const raw = object(value, path);

  return {
    capturedAt: count(raw.capturedAt, at(path, "capturedAt")),
    schemaVersion: version(
      PROCESS_LIST_SCHEMA_VERSION,
      raw.schemaVersion,
      at(path, "schemaVersion"),
    ),
    total: count(raw.total, at(path, "total")),
    offset: count(raw.offset, at(path, "offset")),
    limit: count(raw.limit, at(path, "limit")),
    truncated: boolean(raw.truncated, at(path, "truncated")),
    items: listOf(raw.items, at(path, "items"), MAX_PAGE_ITEMS, parseProcess),
  };
};

export function listProcesses(options: ProcessPageOptions = {}): Promise<ProcessList> {
  const args: Record<string, number> = {};
  if (options.offset !== undefined) args.offset = options.offset;
  if (options.limit !== undefined) args.limit = options.limit;

  return invokeCommand("list_processes", parseProcessList, args);
}

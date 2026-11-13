import { invokeCommand } from "./ipc";
import type { UnavailableField } from "./system";

/**
 * Mirrors the `list_processes` DTO in
 * `src-tauri/src/dto/process_list.rs` (`PROCESS_LIST_SCHEMA_VERSION = 1`).
 *
 * A missing field is always paired with an entry in the row's `unavailable`
 * list; `undefined` alone never means "could not read".
 */
export interface ProcessInfo {
  /** Stable across refreshes unless the start time was unreadable (`pid:<pid>`). */
  key: string;
  pid: number;
  parentPid: number;
  name?: string;
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

export function listProcesses(options: ProcessPageOptions = {}): Promise<ProcessList> {
  const args: Record<string, number> = {};
  if (options.offset !== undefined) args.offset = options.offset;
  if (options.limit !== undefined) args.limit = options.limit;

  return invokeCommand<ProcessList>("list_processes", args);
}

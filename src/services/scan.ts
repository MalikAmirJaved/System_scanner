import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeCommand } from "./ipc";
import {
  at,
  boolean,
  count,
  object,
  oneOf,
  optional,
  optionalString,
  parsePayload,
  string,
  type Validator,
} from "./validate";
import { IpcContractError } from "./validate";

/**
 * Mirrors `src-tauri/src/dto/scan.rs` (`API.md` §4.1).
 *
 * Only aggregate counters cross the boundary; no path or file name is ever
 * returned. Every payload is validated before a component sees it (B-090).
 */
const SCAN_STATES = [
  "queued",
  "running",
  "cancelling",
  "completed",
  "failed",
  "cancelled",
] as const;

export type ScanState = (typeof SCAN_STATES)[number];

const COMPLETENESS_VALUES = ["complete", "partial", "truncated", "cancelled"] as const;

export type Completeness = (typeof COMPLETENESS_VALUES)[number];

export interface ScanCounters {
  entries: number;
  files: number;
  directories: number;
  reparsePoints: number;
  bytes: number;
  skipped: number;
  denied: number;
  disappeared: number;
  unsupported: number;
  failed: number;
}

export interface ScanStatus {
  jobId: string;
  state: ScanState;
  seq: number;
  elapsedMs: number;
  counters: ScanCounters;
  completeness?: Completeness;
  errorCode?: string;
}

export interface ScanProgress {
  jobId: string;
  seq: number;
  phase: string;
  filesSeen: number;
  bytesSeen: number;
  skipped: number;
  elapsedMs: number;
}

export interface ScanCompleted {
  jobId: string;
  summary: ScanCounters;
  completeness: Completeness;
  partial: boolean;
  elapsedMs: number;
}

export interface ScanFailed {
  jobId: string;
  errorCode: string;
  message: string;
}

export interface ScanCancelled {
  jobId: string;
  reason: string;
  summary: ScanCounters;
  elapsedMs: number;
}

export type ScanEvent =
  | { kind: "progress"; payload: ScanProgress }
  | { kind: "completed"; payload: ScanCompleted }
  | { kind: "failed"; payload: ScanFailed }
  | { kind: "cancelled"; payload: ScanCancelled };

const parseCounters: Validator<ScanCounters> = (value, path) => {
  const raw = object(value, path);

  return {
    entries: count(raw.entries, at(path, "entries")),
    files: count(raw.files, at(path, "files")),
    directories: count(raw.directories, at(path, "directories")),
    reparsePoints: count(raw.reparsePoints, at(path, "reparsePoints")),
    bytes: count(raw.bytes, at(path, "bytes")),
    skipped: count(raw.skipped, at(path, "skipped")),
    denied: count(raw.denied, at(path, "denied")),
    disappeared: count(raw.disappeared, at(path, "disappeared")),
    unsupported: count(raw.unsupported, at(path, "unsupported")),
    failed: count(raw.failed, at(path, "failed")),
  };
};

const parseCompleteness: Validator<Completeness> = (value, path) =>
  oneOf(COMPLETENESS_VALUES, value, path);

const parseScanStatus: Validator<ScanStatus> = (value, path) => {
  const raw = object(value, path);

  return {
    jobId: string(raw.jobId, at(path, "jobId")),
    state: oneOf(SCAN_STATES, raw.state, at(path, "state")),
    seq: count(raw.seq, at(path, "seq")),
    elapsedMs: count(raw.elapsedMs, at(path, "elapsedMs")),
    counters: parseCounters(raw.counters, at(path, "counters")),
    completeness: optional(raw.completeness, at(path, "completeness"), parseCompleteness),
    errorCode: optionalString(raw.errorCode, at(path, "errorCode")),
  };
};

const parseStartResult: Validator<{ jobId: string }> = (value, path) => {
  const raw = object(value, path);

  return { jobId: string(raw.jobId, at(path, "jobId")) };
};

const parseCancelResult: Validator<{ accepted: boolean }> = (value, path) => {
  const raw = object(value, path);

  return { accepted: boolean(raw.accepted, at(path, "accepted")) };
};

const parseScanProgress: Validator<ScanProgress> = (value, path) => {
  const raw = object(value, path);

  return {
    jobId: string(raw.jobId, at(path, "jobId")),
    seq: count(raw.seq, at(path, "seq")),
    phase: string(raw.phase, at(path, "phase")),
    filesSeen: count(raw.filesSeen, at(path, "filesSeen")),
    bytesSeen: count(raw.bytesSeen, at(path, "bytesSeen")),
    skipped: count(raw.skipped, at(path, "skipped")),
    elapsedMs: count(raw.elapsedMs, at(path, "elapsedMs")),
  };
};

const parseScanCompleted: Validator<ScanCompleted> = (value, path) => {
  const raw = object(value, path);

  return {
    jobId: string(raw.jobId, at(path, "jobId")),
    summary: parseCounters(raw.summary, at(path, "summary")),
    completeness: parseCompleteness(raw.completeness, at(path, "completeness")),
    partial: boolean(raw.partial, at(path, "partial")),
    elapsedMs: count(raw.elapsedMs, at(path, "elapsedMs")),
  };
};

const parseScanFailed: Validator<ScanFailed> = (value, path) => {
  const raw = object(value, path);

  return {
    jobId: string(raw.jobId, at(path, "jobId")),
    errorCode: string(raw.errorCode, at(path, "errorCode")),
    message: string(raw.message, at(path, "message")),
  };
};

const parseScanCancelled: Validator<ScanCancelled> = (value, path) => {
  const raw = object(value, path);

  return {
    jobId: string(raw.jobId, at(path, "jobId")),
    reason: string(raw.reason, at(path, "reason")),
    summary: parseCounters(raw.summary, at(path, "summary")),
    elapsedMs: count(raw.elapsedMs, at(path, "elapsedMs")),
  };
};

/** Starts one read-only scan. Rejects with `busy` while another is active. */
export function startScan(roots: string[]): Promise<{ jobId: string }> {
  return invokeCommand("start_scan", parseStartResult, { request: { roots } });
}

/** Idempotent. `accepted: false` means there was nothing to cancel. */
export function cancelScan(jobId?: string): Promise<{ accepted: boolean }> {
  return invokeCommand("cancel_scan", parseCancelResult, { jobId });
}

/** Authoritative state of the current or most recent scan, or `not_found`. */
export function getScanStatus(jobId?: string): Promise<ScanStatus> {
  return invokeCommand("get_scan_status", parseScanStatus, { jobId });
}

/**
 * Event names and their payload readers. The name is used as the error source,
 * so a contract failure always says which event was malformed.
 */
const EVENT_PARSERS: ReadonlyArray<{
  name: string;
  parse: (payload: unknown) => ScanEvent;
}> = [
  {
    name: "scan:progress",
    parse: (payload) => ({
      kind: "progress",
      payload: parsePayload("scan:progress", parseScanProgress, payload),
    }),
  },
  {
    name: "scan:completed",
    parse: (payload) => ({
      kind: "completed",
      payload: parsePayload("scan:completed", parseScanCompleted, payload),
    }),
  },
  {
    name: "scan:failed",
    parse: (payload) => ({
      kind: "failed",
      payload: parsePayload("scan:failed", parseScanFailed, payload),
    }),
  },
  {
    name: "scan:cancelled",
    parse: (payload) => ({
      kind: "cancelled",
      payload: parsePayload("scan:cancelled", parseScanCancelled, payload),
    }),
  },
];

/**
 * Subscribes to the four `scan:*` events and resolves to one unsubscribe
 * function. Events are a convenience; `getScanStatus` is the source of truth.
 *
 * A payload that does not match the contract is reported to `onInvalid` and
 * dropped instead of reaching the view: `listen` callbacks cannot surface an
 * exception, and one bad event must not break the other three. Without
 * `onInvalid` the event is dropped silently, which is safe only because status
 * polling is authoritative.
 *
 * If any subscription fails, the ones already made are removed and the error
 * is rethrown, so a half-subscribed panel cannot leak listeners.
 */
export async function subscribeToScan(
  handler: (event: ScanEvent) => void,
  onInvalid?: (error: IpcContractError) => void,
): Promise<() => void> {
  const unlisteners: UnlistenFn[] = [];

  try {
    for (const parser of EVENT_PARSERS) {
      unlisteners.push(
        await listen<unknown>(parser.name, (event) => {
          let parsed: ScanEvent;

          try {
            parsed = parser.parse(event.payload);
          } catch (error) {
            if (error instanceof IpcContractError) {
              onInvalid?.(error);
              return;
            }

            throw error;
          }

          handler(parsed);
        }),
      );
    }
  } catch (error) {
    unlisteners.forEach((unlisten) => unlisten());
    throw error;
  }

  return () => unlisteners.forEach((unlisten) => unlisten());
}

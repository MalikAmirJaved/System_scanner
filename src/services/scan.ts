import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeCommand } from "./ipc";

/**
 * Mirrors `src-tauri/src/dto/scan.rs` (`API.md` §4.1).
 *
 * Only aggregate counters cross the boundary; no path or file name is ever
 * returned. Payloads are not validated at runtime (KI-011).
 */
export type ScanState = "queued" | "running" | "cancelling" | "completed" | "failed" | "cancelled";

export type Completeness = "complete" | "partial" | "truncated" | "cancelled";

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

/** Starts one read-only scan. Rejects with `busy` while another is active. */
export function startScan(roots: string[]): Promise<{ jobId: string }> {
  return invokeCommand<{ jobId: string }>("start_scan", { request: { roots } });
}

/** Idempotent. `accepted: false` means there was nothing to cancel. */
export function cancelScan(jobId?: string): Promise<{ accepted: boolean }> {
  return invokeCommand<{ accepted: boolean }>("cancel_scan", { jobId });
}

/** Authoritative state of the current or most recent scan, or `not_found`. */
export function getScanStatus(jobId?: string): Promise<ScanStatus> {
  return invokeCommand<ScanStatus>("get_scan_status", { jobId });
}

const EVENT_KINDS = [
  ["scan:progress", "progress"],
  ["scan:completed", "completed"],
  ["scan:failed", "failed"],
  ["scan:cancelled", "cancelled"],
] as const;

/**
 * Subscribes to the four `scan:*` events and resolves to one unsubscribe
 * function. Events are a convenience; `getScanStatus` is the source of truth.
 *
 * If any subscription fails, the ones already made are removed and the error
 * is rethrown, so a half-subscribed panel cannot leak listeners.
 */
export async function subscribeToScan(handler: (event: ScanEvent) => void): Promise<() => void> {
  const unlisteners: UnlistenFn[] = [];

  try {
    for (const [name, kind] of EVENT_KINDS) {
      unlisteners.push(
        await listen<ScanEvent["payload"]>(name, (event) =>
          handler({ kind, payload: event.payload } as ScanEvent),
        ),
      );
    }
  } catch (error) {
    unlisteners.forEach((unlisten) => unlisten());
    throw error;
  }

  return () => unlisteners.forEach((unlisten) => unlisten());
}

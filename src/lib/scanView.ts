import type { Completeness, ScanCounters, ScanEvent, ScanStatus } from "../services/scan";

export type ScanPhase =
  "idle" | "starting" | "running" | "cancelling" | "completed" | "cancelled" | "failed";

export interface ScanView {
  phase: ScanPhase;
  jobId?: string;
  /** Last progress sequence applied; older or repeated events are dropped. */
  seq: number;
  counters: ScanCounters;
  completeness?: Completeness;
  elapsedMs: number;
  errorCode?: string;
}

export const ZERO_COUNTERS: ScanCounters = {
  entries: 0,
  files: 0,
  directories: 0,
  reparsePoints: 0,
  bytes: 0,
  skipped: 0,
  denied: 0,
  disappeared: 0,
  unsupported: 0,
  failed: 0,
};

export const IDLE_VIEW: ScanView = {
  phase: "idle",
  seq: 0,
  counters: ZERO_COUNTERS,
  elapsedMs: 0,
};

export function isActive(phase: ScanPhase): boolean {
  return phase === "starting" || phase === "running" || phase === "cancelling";
}

function isTerminal(phase: ScanPhase): boolean {
  return phase === "completed" || phase === "cancelled" || phase === "failed";
}

/**
 * Applies one event. Pure.
 *
 * A job is adopted from its first event only while the panel is `starting`
 * (the start call has not returned its ID yet). After that, events for any
 * other job are ignored, as are progress events that are not newer.
 */
export function applyEvent(view: ScanView, event: ScanEvent): ScanView {
  const { jobId } = event.payload;

  if (view.jobId === undefined) {
    if (view.phase !== "starting") return view;
  } else if (view.jobId !== jobId) {
    return view;
  }

  const base = { ...view, jobId };

  switch (event.kind) {
    case "progress": {
      if (isTerminal(view.phase) || event.payload.seq <= view.seq) return view;
      return {
        ...base,
        phase: view.phase === "cancelling" ? "cancelling" : "running",
        seq: event.payload.seq,
        counters: {
          ...view.counters,
          files: event.payload.filesSeen,
          bytes: event.payload.bytesSeen,
          skipped: event.payload.skipped,
        },
        elapsedMs: event.payload.elapsedMs,
      };
    }
    case "completed":
      return {
        ...base,
        phase: "completed",
        counters: event.payload.summary,
        completeness: event.payload.completeness,
        elapsedMs: event.payload.elapsedMs,
      };
    case "cancelled":
      return {
        ...base,
        phase: "cancelled",
        counters: event.payload.summary,
        completeness: "cancelled",
        elapsedMs: event.payload.elapsedMs,
      };
    case "failed":
      return { ...base, phase: "failed", errorCode: event.payload.errorCode };
  }
}

/**
 * Applies an authoritative status. Used after start, after cancel, and on
 * mount to recover a scan that began before this panel existed.
 *
 * It never replaces a terminal view of the same job (the event already said it)
 * and never moves the progress sequence backwards.
 */
export function applyStatus(view: ScanView, status: ScanStatus): ScanView {
  if (view.jobId !== undefined && view.jobId !== status.jobId) return view;
  if (view.jobId === undefined && view.phase !== "idle" && view.phase !== "starting") {
    return view;
  }
  if (isTerminal(view.phase)) return view;

  const phase: ScanPhase = status.state === "queued" ? "running" : (status.state as ScanPhase);

  return {
    phase,
    jobId: status.jobId,
    seq: Math.max(view.seq, status.seq),
    counters: status.counters,
    completeness: status.completeness,
    elapsedMs: status.elapsedMs,
    errorCode: status.errorCode,
  };
}

/** What a finished scan does and does not tell the user. */
export function completenessNote(completeness: Completeness): string {
  switch (completeness) {
    case "complete":
      return "Every observed entry was read. Reparse points (junctions, symlinks) were still not followed.";
    case "partial":
      return "Some entries were skipped, denied, missing, or unreadable, so these totals understate the tree.";
    case "truncated":
      return "The scan stopped at its entry limit, so these totals cover only part of the tree.";
    case "cancelled":
      return "The scan was stopped early. These are the counts reached so far.";
  }
}

export const COMPLETENESS_LABEL: Record<Completeness, string> = {
  complete: "Complete",
  partial: "Partial",
  truncated: "Truncated",
  cancelled: "Cancelled",
};

/** Roots from a one-per-line text box: trimmed, blank lines dropped. */
export function parseRoots(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

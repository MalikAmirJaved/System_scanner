import { useEffect, useState } from "react";
import { Play, Square } from "lucide-react";
import { summarizeError } from "../lib/errors";
import { formatBytes } from "../lib/format";
import {
  COMPLETENESS_LABEL,
  IDLE_VIEW,
  ZERO_COUNTERS,
  applyEvent,
  applyStatus,
  completenessNote,
  isActive,
  parseRoots,
  type ScanView,
} from "../lib/scanView";
import { cn } from "../lib/utils";
import { IpcError } from "../services/ipc";
import { cancelScan, getScanStatus, startScan, subscribeToScan } from "../services/scan";
import { Button } from "./ui/button";

interface PanelError {
  code: string;
  message: string;
}

/**
 * Starts, watches, and cancels one read-only storage scan.
 *
 * The backend returns aggregate counters only, so this panel never shows a file
 * name or path. Events drive the live numbers; `get_scan_status` is the source
 * of truth after start, after cancel, and on mount, so a missed event cannot
 * leave the panel wrong.
 */
export function ScanPanel({ className }: { className?: string }) {
  const [rootsText, setRootsText] = useState("");
  const [view, setView] = useState<ScanView>(IDLE_VIEW);
  const [error, setError] = useState<PanelError | null>(null);

  const reportError = (cause: unknown) => {
    const { code, message } = summarizeError(cause);
    const detail = cause instanceof IpcError ? cause.detail : undefined;
    setError({ code, message: detail ? `${message} ${detail}.` : message });
  };

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;

    subscribeToScan(
      (event) => setView((current) => applyEvent(current, event)),
      // An unreadable event is a contract failure, not a scan failure. Report it
      // and re-read the authoritative status so the panel cannot stall on a
      // dropped terminal event.
      (cause) => {
        reportError(cause);
        getScanStatus()
          .then((status) => {
            if (!disposed) setView((current) => applyStatus(current, status));
          })
          .catch(() => undefined);
      },
    )
      .then((stop) => {
        if (disposed) stop();
        else unsubscribe = stop;
      })
      .catch(() => {
        // No Tauri runtime (browser dev): starting a scan reports it instead.
      });

    // Recover a scan that began before this panel mounted. `not_found` is the
    // normal "no scan yet" answer and leaves the panel idle.
    getScanStatus()
      .then((status) => {
        if (!disposed) setView((current) => applyStatus(current, status));
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  const roots = parseRoots(rootsText);
  const active = isActive(view.phase);

  const start = () => {
    setError(null);
    setView({ ...IDLE_VIEW, phase: "starting", counters: ZERO_COUNTERS });

    startScan(roots)
      .then(({ jobId }) => {
        setView((current) =>
          current.phase === "starting" ? { ...current, jobId, phase: "running" } : current,
        );
        return getScanStatus(jobId);
      })
      .then((status) => setView((current) => applyStatus(current, status)))
      .catch((cause: unknown) => {
        reportError(cause);
        // A rejected start never created a job, so there is nothing to show.
        setView((current) => (current.jobId === undefined ? IDLE_VIEW : current));
      });
  };

  const cancel = () => {
    setError(null);
    setView((current) =>
      current.phase === "running" ? { ...current, phase: "cancelling" } : current,
    );

    cancelScan(view.jobId)
      .then(() => getScanStatus(view.jobId))
      .then((status) => setView((current) => applyStatus(current, status)))
      .catch(reportError);
  };

  return (
    <section
      aria-labelledby="scan-panel-title"
      className={cn("rounded-lg border border-border bg-card p-4 shadow-sm sm:p-6", className)}
    >
      <header className="space-y-1">
        <h2 id="scan-panel-title" className="text-lg font-semibold tracking-tight">
          Storage scan
        </h2>
        <p className="text-xs text-muted-foreground">
          Read-only · counts only, no file names are shown · junctions and symlinks are not followed
        </p>
      </header>

      <div className="mt-4 space-y-2">
        <label
          htmlFor="scan-roots"
          className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
        >
          Folders to scan (one absolute path per line)
        </label>
        <textarea
          id="scan-roots"
          rows={3}
          value={rootsText}
          onChange={(event) => setRootsText(event.target.value)}
          disabled={active}
          spellCheck={false}
          placeholder={"C:\\Users\\you\\Downloads"}
          className="w-full resize-y rounded-md border border-border bg-background p-3 font-mono text-xs text-foreground disabled:opacity-60"
        />

        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" onClick={start} disabled={active || roots.length === 0}>
            <Play aria-hidden="true" className="size-3.5" />
            {view.phase === "starting" ? "Starting…" : "Start scan"}
          </Button>
          {(view.phase === "running" || view.phase === "cancelling") && (
            <Button size="sm" onClick={cancel} disabled={view.phase === "cancelling"}>
              <Square aria-hidden="true" className="size-3.5" />
              {view.phase === "cancelling" ? "Stopping…" : "Cancel"}
            </Button>
          )}
          <span className="text-xs text-muted-foreground">
            {roots.length === 0
              ? "Enter at least one folder."
              : `${roots.length} ${roots.length === 1 ? "folder" : "folders"} selected`}
          </span>
        </div>
      </div>

      {error && (
        <p className="mt-4 text-sm font-medium text-destructive" role="alert">
          Scan unavailable ({error.code}). {error.message}
        </p>
      )}

      <Result view={view} />
    </section>
  );
}

function Result({ view }: { view: ScanView }) {
  if (view.phase === "idle") return null;

  if (view.phase === "failed") {
    return (
      <p className="mt-4 text-sm font-medium text-destructive" role="alert">
        The scan failed ({view.errorCode ?? "unknown"}). No result is available.
      </p>
    );
  }

  const { counters } = view;
  const finished = view.phase === "completed" || view.phase === "cancelled";

  return (
    <div className="mt-4 space-y-3">
      <p className="text-sm" role="status" aria-live="polite">
        {headline(view)}
      </p>

      <dl className="grid grid-cols-2 gap-x-6 gap-y-3 text-sm sm:grid-cols-3 lg:grid-cols-4">
        <Fact label="Files seen" value={counters.files.toLocaleString()} />
        <Fact label="Size seen" value={formatBytes(counters.bytes)} />
        <Fact label="Skipped" value={counters.skipped.toLocaleString()} />
        <Fact label="Elapsed" value={`${(view.elapsedMs / 1000).toFixed(1)} s`} />
        {finished && (
          <>
            <Fact label="Folders" value={counters.directories.toLocaleString()} />
            <Fact label="Denied" value={counters.denied.toLocaleString()} />
            <Fact label="Missing" value={counters.disappeared.toLocaleString()} />
            <Fact
              label="Unreadable"
              value={(counters.unsupported + counters.failed).toLocaleString()}
            />
          </>
        )}
      </dl>

      {finished && view.completeness && (
        <p className="text-xs text-muted-foreground">{completenessNote(view.completeness)}</p>
      )}
    </div>
  );
}

function headline(view: ScanView): string {
  switch (view.phase) {
    case "starting":
      return "Starting the scan…";
    case "running":
      return "Scanning…";
    case "cancelling":
      return "Stopping the scan…";
    default:
      return view.completeness
        ? `${COMPLETENESS_LABEL[view.completeness]} scan finished.`
        : "Scan finished.";
  }
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-2xs uppercase tracking-widest text-muted-foreground">{label}</dt>
      <dd className="mt-0.5 break-words">{value}</dd>
    </div>
  );
}

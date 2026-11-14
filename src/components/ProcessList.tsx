import { useEffect, useState } from "react";
import { ChevronLeft, ChevronRight, RefreshCw } from "lucide-react";
import { summarizeError } from "../lib/errors";
import { formatBytes, formatDateTime, formatTime } from "../lib/format";
import { cn } from "../lib/utils";
import {
  listProcesses,
  type ProcessInfo,
  type ProcessList as ProcessPage,
} from "../services/processes";
import { Button } from "./ui/button";

/** Mirrors the default page size in `src-tauri/src/domain/process.rs`. */
const PAGE_SIZE = 100;

type ListState =
  | { status: "loading" }
  | { status: "ready"; list: ProcessPage }
  | { status: "error"; code: string; message: string };

/**
 * Read-only process inventory with explicit paging.
 *
 * The page is requested from Rust, which validates and caps the arguments; this
 * component never sends a limit it invented. Rows Windows refused to open keep
 * their identifiers and state which field is missing.
 */
export function ProcessList({ className }: { className?: string }) {
  const [offset, setOffset] = useState(0);
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<ListState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    listProcesses({ offset, limit: PAGE_SIZE })
      .then((list) => {
        if (!active) return;

        // The inventory shrank between refreshes; go back to a page that exists.
        if (list.total > 0 && list.offset >= list.total) {
          setOffset(0);
          return;
        }

        setState({ status: "ready", list });
      })
      .catch((error: unknown) => {
        if (!active) return;
        const { code, message } = summarizeError(error);
        setState({ status: "error", code, message });
      });

    return () => {
      active = false;
    };
  }, [offset, attempt]);

  const loading = state.status === "loading";

  const refresh = () => {
    setState({ status: "loading" });
    setAttempt((count) => count + 1);
  };

  const goToPage = (next: number) => {
    setState({ status: "loading" });
    setOffset(next);
  };

  return (
    <section
      aria-labelledby="process-panel-title"
      className={cn("rounded-lg border border-border bg-card p-4 shadow-sm sm:p-6", className)}
    >
      <header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <h2 id="process-panel-title" className="text-lg font-semibold tracking-tight">
            Processes
          </h2>
          {state.status === "ready" && (
            <p className="text-xs text-muted-foreground">
              Read-only · updated {formatTime(state.list.capturedAt)}
            </p>
          )}
        </div>

        <Button size="sm" onClick={refresh} disabled={loading} className="self-start">
          <RefreshCw aria-hidden="true" className={cn("size-3.5", loading && "animate-spin")} />
          {loading ? "Reading…" : "Refresh"}
        </Button>
      </header>

      {state.status === "loading" && (
        <p className="mt-4 text-sm text-muted-foreground" role="status">
          Reading the process list…
        </p>
      )}

      {state.status === "error" && (
        <p className="mt-4 text-sm font-medium text-destructive" role="alert">
          The process list is unavailable ({state.code}). {state.message}
        </p>
      )}

      {state.status === "ready" && <ListBody list={state.list} />}

      {state.status === "ready" && state.list.total > 0 && (
        <div className="mt-4 flex flex-wrap items-center justify-between gap-3">
          <Button
            size="sm"
            onClick={() => goToPage(Math.max(0, offset - PAGE_SIZE))}
            disabled={offset === 0 || loading}
          >
            <ChevronLeft aria-hidden="true" className="size-3.5" />
            Previous
          </Button>
          <span className="text-xs text-muted-foreground">
            Page {Math.floor(offset / PAGE_SIZE) + 1}
          </span>
          <Button
            size="sm"
            onClick={() => goToPage(offset + PAGE_SIZE)}
            disabled={offset + state.list.items.length >= state.list.total}
          >
            Next
            <ChevronRight aria-hidden="true" className="size-3.5" />
          </Button>
        </div>
      )}
    </section>
  );
}

function caption(list: ProcessPage): string {
  const first = list.offset + 1;
  const last = list.offset + list.items.length;
  const range = list.items.length === 0 ? "0" : `${first}–${last}`;
  const total = `${list.total.toLocaleString()}${list.truncated ? "+" : ""}`;
  const page = Math.floor(list.offset / PAGE_SIZE) + 1;

  return `Page ${page} · Read-only · ${range} of ${total} · updated ${formatTime(list.capturedAt)}`;
}

function ListBody({ list }: { list: ProcessPage }) {
  if (list.items.length === 0) {
    return (
      <p className="mt-4 text-sm text-muted-foreground" role="status">
        No processes were reported.
      </p>
    );
  }

  const anyGaps = list.items.some((process) => process.unavailable.length > 0);

  return (
    <>
      <div className="mt-4 overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <caption className="pb-2 text-left text-xs text-muted-foreground">
            {caption(list)}
          </caption>
          <thead>
            <tr className="border-b border-border text-left">
              <th
                scope="col"
                className="py-2 pr-4 text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
              >
                Process
              </th>
              <th
                scope="col"
                className="py-2 pr-4 text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
              >
                PID
              </th>
              <th
                scope="col"
                className="py-2 pr-4 text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
              >
                Parent
              </th>
              <th
                scope="col"
                className="py-2 pr-4 text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
              >
                Threads
              </th>
              <th
                scope="col"
                className="py-2 pr-4 text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
              >
                Working set
              </th>
              <th
                scope="col"
                className="py-2 text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
              >
                Started
              </th>
            </tr>
          </thead>
          <tbody>
            {list.items.map((process, index) => (
              <tr key={process.key} className="border-b border-border/60 align-top">
                <td className="py-2 pr-4 font-medium">
                  {process.name ? process.name : <Gap reason={gapReason(process, index, "name")} />}
                </td>
                <td className="py-2 pr-4 font-mono text-xs">{process.pid}</td>
                <td className="py-2 pr-4 font-mono text-xs">{process.parentPid}</td>
                <td className="py-2 pr-4">
                  <Cell
                    value={process.threadCount}
                    reason={gapReason(process, index, "threadCount")}
                  />
                </td>
                <td className="py-2 pr-4">
                  <Cell
                    value={
                      process.workingSetBytes === undefined
                        ? undefined
                        : formatBytes(process.workingSetBytes)
                    }
                    reason={gapReason(process, index, "workingSetBytes")}
                  />
                </td>
                <td className="py-2">
                  <Cell
                    value={
                      process.startTime === undefined
                        ? undefined
                        : formatDateTime(process.startTime)
                    }
                    reason={gapReason(process, index, "startTime")}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {list.truncated && (
        <p className="mt-3 text-xs text-muted-foreground">
          The list hit its safety cap, so it is a prefix of this machine.
        </p>
      )}

      {anyGaps && (
        <p className="mt-3 text-xs text-muted-foreground">
          Rows marked unavailable were denied by Windows; the rest of the row is still reported.
        </p>
      )}
    </>
  );
}

function Cell({ value, reason }: { value?: number | string; reason: string }) {
  if (value === undefined) {
    return <Gap reason={reason} />;
  }
  return <>{value}</>;
}

function Gap({ reason }: { reason: string }) {
  return <span className="text-xs font-medium text-destructive">Unavailable ({reason})</span>;
}

function gapReason(process: ProcessInfo, index: number, field: string): string {
  return (
    process.unavailable.find((item) => item.field === `items[${index}].${field}`)?.reason ??
    "reason not reported"
  );
}

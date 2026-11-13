import { useEffect, useState } from "react";
import { summarizeError } from "../lib/errors";
import { formatBytes, formatDateTime, formatTime } from "../lib/format";
import {
  listProcesses,
  type ProcessInfo,
  type ProcessList as ProcessPage,
} from "../services/processes";

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
export function ProcessList() {
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
    <section className="process-panel" aria-labelledby="process-panel-title">
      <header className="process-panel-header">
        <div>
          <h2 id="process-panel-title">Processes</h2>
          {state.status === "ready" && <p className="process-caption">{caption(state.list)}</p>}
        </div>
        <button type="button" className="process-refresh" onClick={refresh} disabled={loading}>
          {loading ? "Reading…" : "Refresh"}
        </button>
      </header>

      {state.status === "loading" && (
        <p className="process-message" role="status">
          Reading the process list…
        </p>
      )}

      {state.status === "error" && (
        <p className="process-message process-message--error" role="alert">
          The process list is unavailable ({state.code}). {state.message}
        </p>
      )}

      {state.status === "ready" && <ListBody list={state.list} />}

      {state.status === "ready" && state.list.total > 0 && (
        <div className="process-pager">
          <button
            type="button"
            className="process-page-button"
            onClick={() => goToPage(Math.max(0, offset - PAGE_SIZE))}
            disabled={offset === 0 || loading}
          >
            Previous
          </button>
          <span className="process-caption">Page {Math.floor(offset / PAGE_SIZE) + 1}</span>
          <button
            type="button"
            className="process-page-button"
            onClick={() => goToPage(offset + PAGE_SIZE)}
            disabled={offset + state.list.items.length >= state.list.total}
          >
            Next
          </button>
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

  return `Read-only · ${range} of ${total} · updated ${formatTime(list.capturedAt)}`;
}

function ListBody({ list }: { list: ProcessPage }) {
  if (list.items.length === 0) {
    return (
      <p className="process-message" role="status">
        No processes were reported.
      </p>
    );
  }

  const anyGaps = list.items.some((process) => process.unavailable.length > 0);

  return (
    <>
      <div className="process-scroll">
        <table className="process-table">
          <thead>
            <tr>
              <th scope="col">Process</th>
              <th scope="col">PID</th>
              <th scope="col">Parent</th>
              <th scope="col">Threads</th>
              <th scope="col">Working set</th>
              <th scope="col">Started</th>
            </tr>
          </thead>
          <tbody>
            {list.items.map((process, index) => (
              <tr key={process.key}>
                <td>
                  {process.name ? (
                    <span className="process-name">{process.name}</span>
                  ) : (
                    <Gap reason={gapReason(process, index, "name")} />
                  )}
                </td>
                <td className="process-num">{process.pid}</td>
                <td className="process-num">{process.parentPid}</td>
                <td className="process-num">
                  <Cell
                    value={process.threadCount}
                    reason={gapReason(process, index, "threadCount")}
                  />
                </td>
                <td className="process-num">
                  <Cell
                    value={
                      process.workingSetBytes === undefined
                        ? undefined
                        : formatBytes(process.workingSetBytes)
                    }
                    reason={gapReason(process, index, "workingSetBytes")}
                  />
                </td>
                <td className="process-num">
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
        <p className="process-note">
          The list hit its safety cap, so it is a prefix of this machine.
        </p>
      )}

      {anyGaps && (
        <p className="process-note">
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
  return <span className="process-gap">Unavailable ({reason})</span>;
}

function gapReason(process: ProcessInfo, index: number, field: string): string {
  return (
    process.unavailable.find((item) => item.field === `items[${index}].${field}`)?.reason ??
    "reason not reported"
  );
}

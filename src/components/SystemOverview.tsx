import { useEffect, useState } from "react";
import { summarizeError } from "../lib/errors";
import { formatBytes, formatTime } from "../lib/format";
import { getSystemSnapshot, type SystemSnapshot } from "../services/system";

type OverviewState =
  | { status: "loading" }
  | { status: "ready"; snapshot: SystemSnapshot }
  | { status: "error"; code: string; message: string };

function toErrorState(error: unknown): OverviewState {
  const { code, message } = summarizeError(error);
  return { status: "error", code, message };
}

function reasonFor(snapshot: SystemSnapshot, field: string): string {
  return snapshot.unavailable.find((item) => item.field === field)?.reason ?? "reason not reported";
}

function Gap({ reason }: { reason: string }) {
  return <span className="system-gap">Unavailable ({reason})</span>;
}

function Value({ value, reason }: { value?: string; reason: string }) {
  return value ? <>{value}</> : <Gap reason={reason} />;
}

/**
 * Read-only view of the current system snapshot.
 *
 * Every section states why data is missing instead of rendering an empty or
 * zero-filled value, and a refresh never lets an older response win.
 */
export function SystemOverview() {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<OverviewState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    getSystemSnapshot()
      .then((snapshot) => {
        if (active) setState({ status: "ready", snapshot });
      })
      .catch((error: unknown) => {
        if (active) setState(toErrorState(error));
      });

    return () => {
      active = false;
    };
  }, [attempt]);

  const refresh = () => {
    setState({ status: "loading" });
    setAttempt((count) => count + 1);
  };

  const loading = state.status === "loading";

  return (
    <section className="system-panel" aria-labelledby="system-panel-title">
      <header className="system-panel-header">
        <div>
          <h2 id="system-panel-title">This machine</h2>
          {state.status === "ready" && (
            <p className="system-caption">
              Read-only snapshot · updated {formatTime(state.snapshot.capturedAt)}
            </p>
          )}
        </div>
        <button type="button" className="system-refresh" onClick={refresh} disabled={loading}>
          {loading ? "Reading…" : "Refresh"}
        </button>
      </header>

      {state.status === "loading" && (
        <p className="system-message" role="status">
          Reading system information…
        </p>
      )}

      {state.status === "error" && (
        <p className="system-message system-message--error" role="alert">
          System information is unavailable ({state.code}). {state.message}
        </p>
      )}

      {state.status === "ready" && <SnapshotBody snapshot={state.snapshot} />}
    </section>
  );
}

function SnapshotBody({ snapshot }: { snapshot: SystemSnapshot }) {
  const { os, cpu, memory, disks, network } = snapshot;
  const empty = !os && !cpu && !memory && !disks && !network;

  if (empty) {
    return (
      <p className="system-message" role="status">
        No system data could be collected on this machine.
      </p>
    );
  }

  return (
    <dl className="system-grid">
      <div className="system-row">
        <dt>Operating system</dt>
        <dd>
          {os ? (
            <>
              <Value value={os.name} reason={reasonFor(snapshot, "os.name")} />
              {os.edition && <span className="system-muted"> · {os.edition}</span>}
              <span className="system-muted">
                {" "}
                · <Value value={os.version} reason={reasonFor(snapshot, "os.version")} />
              </span>
              <span className="system-muted"> · {os.arch}</span>
            </>
          ) : (
            <Gap reason={reasonFor(snapshot, "os")} />
          )}
        </dd>
      </div>

      <div className="system-row">
        <dt>Processor</dt>
        <dd>
          {cpu ? (
            <>
              <Value value={cpu.model} reason={reasonFor(snapshot, "cpu.model")} />
              <span className="system-muted">
                {" "}
                ·{" "}
                <Value
                  value={cpu.logicalCores?.toString()}
                  reason={reasonFor(snapshot, "cpu.logicalCores")}
                />{" "}
                logical
              </span>
              <span className="system-muted">
                {" "}
                ·{" "}
                <Value
                  value={cpu.physicalCores?.toString()}
                  reason={reasonFor(snapshot, "cpu.physicalCores")}
                />{" "}
                physical
              </span>
            </>
          ) : (
            <Gap reason={reasonFor(snapshot, "cpu")} />
          )}
        </dd>
      </div>

      <div className="system-row">
        <dt>Memory</dt>
        <dd>
          {memory ? (
            <>
              {formatBytes(memory.availableBytes)} free of {formatBytes(memory.totalBytes)}
            </>
          ) : (
            <Gap reason={reasonFor(snapshot, "memory")} />
          )}
        </dd>
      </div>

      <div className="system-row">
        <dt>Disks</dt>
        <dd>
          {disks ? (
            disks.length === 0 ? (
              <span className="system-muted">No volumes reported.</span>
            ) : (
              <ul className="system-list">
                {disks.map((disk, index) => (
                  <li key={`${disk.mount}-${index}`}>
                    <span className="system-mono">{disk.mount}</span>{" "}
                    <span className="system-muted">{disk.kind}</span>
                    {" · "}
                    <Value
                      value={disk.freeBytes === undefined ? undefined : formatBytes(disk.freeBytes)}
                      reason={reasonFor(snapshot, `disks[${index}].freeBytes`)}
                    />{" "}
                    free of{" "}
                    <Value
                      value={
                        disk.totalBytes === undefined ? undefined : formatBytes(disk.totalBytes)
                      }
                      reason={reasonFor(snapshot, `disks[${index}].totalBytes`)}
                    />
                  </li>
                ))}
              </ul>
            )
          ) : (
            <Gap reason={reasonFor(snapshot, "disks")} />
          )}
        </dd>
      </div>

      <div className="system-row">
        <dt>Network</dt>
        <dd>
          {network ? (
            network.length === 0 ? (
              <span className="system-muted">No adapters reported.</span>
            ) : (
              <ul className="system-list">
                {network.map((adapter, index) => (
                  <li key={`${adapter.name}-${index}`}>
                    {adapter.name}{" "}
                    <span className="system-muted">{adapter.isUp ? "up" : "down"}</span>
                    {adapter.addresses.length > 0 && (
                      <>
                        {" · "}
                        <span className="system-mono">{adapter.addresses.join(", ")}</span>
                      </>
                    )}
                  </li>
                ))}
              </ul>
            )
          ) : (
            <Gap reason={reasonFor(snapshot, "network")} />
          )}
        </dd>
      </div>
    </dl>
  );
}

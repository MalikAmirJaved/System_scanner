import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { summarizeError } from "../lib/errors";
import { formatBytes, formatTime } from "../lib/format";
import { cn } from "../lib/utils";
import { getSystemSnapshot, type SystemSnapshot } from "../services/system";
import { Button } from "./ui/button";

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
  return <span className="text-xs font-medium text-destructive">Unavailable ({reason})</span>;
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
export function SystemOverview({ className }: { className?: string }) {
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
    <section
      aria-labelledby="system-panel-title"
      className={cn("rounded-lg border border-border bg-card p-4 shadow-sm sm:p-6", className)}
    >
      <header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <h2 id="system-panel-title" className="text-lg font-semibold tracking-tight">
            This machine
          </h2>
          {state.status === "ready" && (
            <p className="text-xs text-muted-foreground">
              Read-only snapshot · updated {formatTime(state.snapshot.capturedAt)}
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
          Reading system information…
        </p>
      )}

      {state.status === "error" && (
        <p className="mt-4 text-sm font-medium text-destructive" role="alert">
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
      <p className="mt-4 text-sm text-muted-foreground" role="status">
        No system data could be collected on this machine.
      </p>
    );
  }

  return (
    <dl className="mt-4 grid grid-cols-1 gap-x-6 gap-y-5 sm:grid-cols-2 xl:grid-cols-3">
      <div className="space-y-1">
        <dt className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground">
          Operating system
        </dt>
        <dd className="text-sm leading-relaxed">
          {os ? (
            <>
              <Value value={os.name} reason={reasonFor(snapshot, "os.name")} />
              {os.edition && <span className="text-muted-foreground"> · {os.edition}</span>}
              <span className="text-muted-foreground">
                {" "}
                · <Value value={os.version} reason={reasonFor(snapshot, "os.version")} />
              </span>
              <span className="text-muted-foreground"> · {os.arch}</span>
            </>
          ) : (
            <Gap reason={reasonFor(snapshot, "os")} />
          )}
        </dd>
      </div>

      <div className="space-y-1">
        <dt className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground">
          Processor
        </dt>
        <dd className="text-sm leading-relaxed">
          {cpu ? (
            <>
              <Value value={cpu.model} reason={reasonFor(snapshot, "cpu.model")} />
              <span className="text-muted-foreground">
                {" "}
                ·{" "}
                <Value
                  value={cpu.logicalCores?.toString()}
                  reason={reasonFor(snapshot, "cpu.logicalCores")}
                />{" "}
                logical
              </span>
              <span className="text-muted-foreground">
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

      <div className="space-y-1">
        <dt className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground">
          Memory
        </dt>
        <dd className="text-sm leading-relaxed">
          {memory ? (
            <>
              {formatBytes(memory.availableBytes)} free of {formatBytes(memory.totalBytes)}
            </>
          ) : (
            <Gap reason={reasonFor(snapshot, "memory")} />
          )}
        </dd>
      </div>

      <div className="space-y-1 sm:col-span-2 xl:col-span-3">
        <dt className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground">
          Disks
        </dt>
        <dd className="text-sm leading-relaxed">
          {disks ? (
            disks.length === 0 ? (
              <span className="text-muted-foreground">No volumes reported.</span>
            ) : (
              <ul className="grid grid-cols-1 gap-x-6 gap-y-1 sm:grid-cols-2">
                {disks.map((disk, index) => (
                  <li key={`${disk.mount}-${index}`}>
                    <span className="font-mono text-xs">{disk.mount}</span>{" "}
                    <span className="text-muted-foreground">{disk.kind}</span>
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

      <div className="space-y-1 sm:col-span-2 xl:col-span-3">
        <dt className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground">
          Network
        </dt>
        <dd className="text-sm leading-relaxed">
          {network ? (
            network.length === 0 ? (
              <span className="text-muted-foreground">No adapters reported.</span>
            ) : (
              <ul className="grid grid-cols-1 gap-x-6 gap-y-1 sm:grid-cols-2">
                {network.map((adapter, index) => (
                  <li key={`${adapter.name}-${index}`}>
                    {adapter.name}{" "}
                    <span className="text-xs text-muted-foreground">
                      {adapter.isUp ? "up" : "down"}
                    </span>
                    {adapter.addresses.length > 0 && (
                      <>
                        {" · "}
                        <span className="font-mono text-xs">{adapter.addresses.join(", ")}</span>
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

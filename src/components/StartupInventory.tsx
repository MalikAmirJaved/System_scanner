import { useEffect, useState, type ReactNode } from "react";
import { RefreshCw } from "lucide-react";
import { summarizeError } from "../lib/errors";
import { formatTime } from "../lib/format";
import { cn } from "../lib/utils";
import {
  getStartupInventory,
  type ServiceInfo,
  type StartupInventory as StartupData,
  type StartupScope,
  type StartupSource,
} from "../services/startup";
import type { UnavailableField } from "../services/unavailable";
import { Button } from "./ui/button";

const SOURCE_LABELS: Record<StartupSource, string> = {
  registry_run: "Registry · Run",
  registry_run_once: "Registry · RunOnce",
  startup_folder: "Startup folder",
};

const SCOPE_LABELS: Record<StartupScope, string> = {
  current_user: "Current user",
  all_users: "All users",
};

type PanelState =
  | { status: "loading" }
  | { status: "ready"; inventory: StartupData }
  | { status: "error"; code: string; message: string };

/**
 * Read-only startup-entry and service inventory.
 *
 * Every source is shown even when Windows refused it, so a missing section is
 * explained instead of looking empty. Nothing here changes state, starts, stops,
 * or terminates anything.
 */
export function StartupInventory({ className }: { className?: string }) {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<PanelState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    getStartupInventory()
      .then((inventory) => {
        if (active) setState({ status: "ready", inventory });
      })
      .catch((error: unknown) => {
        if (!active) return;
        const { code, message } = summarizeError(error);
        setState({ status: "error", code, message });
      });

    return () => {
      active = false;
    };
  }, [attempt]);

  const loading = state.status === "loading";

  const refresh = () => {
    setState({ status: "loading" });
    setAttempt((count) => count + 1);
  };

  return (
    <section
      aria-labelledby="startup-panel-title"
      className={cn("rounded-lg border border-border bg-card p-4 shadow-sm sm:p-6", className)}
    >
      <header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <h2 id="startup-panel-title" className="text-lg font-semibold tracking-tight">
            Startup &amp; services
          </h2>
          {state.status === "ready" && (
            <p className="text-xs text-muted-foreground">
              Read-only · updated {formatTime(state.inventory.capturedAt)}
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
          Reading startup entries and services…
        </p>
      )}

      {state.status === "error" && (
        <p className="mt-4 text-sm font-medium text-destructive" role="alert">
          The startup and service inventory is unavailable ({state.code}). {state.message}
        </p>
      )}

      {state.status === "ready" && <Body inventory={state.inventory} />}
    </section>
  );
}

function Body({ inventory }: { inventory: StartupData }) {
  const entryCount = inventory.startup.reduce(
    (total, group) => total + (group.entries?.length ?? 0),
    0,
  );

  if (inventory.services === undefined && entryCount === 0 && inventory.unavailable.length === 0) {
    return (
      <p className="mt-4 text-sm text-muted-foreground" role="status">
        No services or startup entries were reported.
      </p>
    );
  }

  return (
    <div className="mt-4 space-y-6">
      <section aria-labelledby="startup-services-title">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h3 id="startup-services-title" className="text-sm font-semibold">
            Services
          </h3>
          {inventory.services && (
            <span className="text-xs text-muted-foreground">
              {inventory.services.length}
              {inventory.servicesTruncated ? "+" : ""} services · read-only
            </span>
          )}
        </div>

        {inventory.services ? (
          <ServiceTable services={inventory.services} unavailable={inventory.unavailable} />
        ) : (
          <p className="mt-2 text-xs text-destructive">
            The service database is unavailable ({reasonFor(inventory.unavailable, "services")}).
          </p>
        )}
      </section>

      <section aria-labelledby="startup-entries-title">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h3 id="startup-entries-title" className="text-sm font-semibold">
            Startup entries
          </h3>
          <span className="text-xs text-muted-foreground">
            {entryCount}
            {inventory.startupTruncated ? "+" : ""} entries · run at sign-in
          </span>
        </div>

        <div className="mt-2 grid gap-4 md:grid-cols-2">
          {inventory.startup.map((group, index) => {
            const path = `startup[${index}].entries`;

            return (
              <div
                key={`${group.scope}.${group.source}`}
                className="rounded-md border border-border/60 p-3"
              >
                <h4 className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground">
                  {SCOPE_LABELS[group.scope]} · {SOURCE_LABELS[group.source]}
                </h4>

                {group.entries ? (
                  group.entries.length === 0 ? (
                    <p className="mt-1 text-xs text-muted-foreground">No entries.</p>
                  ) : (
                    <ul className="mt-1 space-y-1">
                      {group.entries.map((entry, entryIndex) => (
                        <li key={`${path}[${entryIndex}].${entry.name}`} className="text-sm">
                          <span className="font-medium">{entry.name}</span>
                          {entry.command !== undefined ? (
                            <span
                              className="mt-0.5 block max-w-[60ch] truncate font-mono text-2xs text-muted-foreground"
                              title={entry.command}
                            >
                              {entry.command}
                            </span>
                          ) : (
                            <span className="mt-0.5 block text-2xs">
                              <Gap
                                reason={reasonFor(
                                  inventory.unavailable,
                                  `${path}[${entryIndex}].command`,
                                )}
                              />
                            </span>
                          )}
                        </li>
                      ))}
                    </ul>
                  )
                ) : (
                  <p className="mt-1 text-xs text-destructive">
                    <Gap reason={reasonFor(inventory.unavailable, path)} />
                  </p>
                )}
              </div>
            );
          })}
        </div>
      </section>

      <p className="text-xs text-muted-foreground">
        Shortcut targets are not resolved, and 32-bit registry entries are not listed. Paths and
        commands stay on this machine.
      </p>
    </div>
  );
}

function ServiceTable({
  services,
  unavailable,
}: {
  services: ServiceInfo[];
  unavailable: UnavailableField[];
}) {
  if (services.length === 0) {
    return (
      <p className="mt-2 text-xs text-muted-foreground" role="status">
        No services were reported.
      </p>
    );
  }

  const anyGaps = services.some((service) => service.unavailable.length > 0);

  return (
    <>
      <div className="mt-2 max-h-80 overflow-y-auto overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead className="sticky top-0 bg-card">
            <tr className="border-b border-border text-left">
              <Th>Service</Th>
              <Th>State</Th>
              <Th>Start</Th>
              <Th>Binary path</Th>
            </tr>
          </thead>
          <tbody>
            {services.map((service, index) => (
              <tr key={service.name} className="border-b border-border/60 align-top">
                <td className="py-2 pr-4">
                  <span className="font-medium">{service.name}</span>
                  {service.displayName && service.displayName !== service.name && (
                    <span className="mt-0.5 block max-w-[36ch] truncate text-2xs text-muted-foreground">
                      {service.displayName}
                    </span>
                  )}
                </td>
                <td className="py-2 pr-4 text-xs">{service.state.replace(/_/g, " ")}</td>
                <td className="py-2 pr-4 text-xs">
                  {service.startType !== undefined ? (
                    service.startType
                  ) : (
                    <Gap reason={reasonFor(unavailable, `services[${index}].startType`)} />
                  )}
                </td>
                <td className="py-2">
                  {service.binaryPath !== undefined ? (
                    <span
                      className="block max-w-[48ch] truncate font-mono text-2xs text-muted-foreground"
                      title={service.binaryPath}
                    >
                      {service.binaryPath}
                    </span>
                  ) : (
                    <Gap reason={reasonFor(unavailable, `services[${index}].binaryPath`)} />
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {anyGaps && (
        <p className="mt-2 text-xs text-muted-foreground">
          Rows marked unavailable were denied by Windows; the rest of the row is still reported.
        </p>
      )}
    </>
  );
}

function Th({ children }: { children: ReactNode }) {
  return (
    <th
      scope="col"
      className="py-2 pr-4 text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
    >
      {children}
    </th>
  );
}

function Gap({ reason }: { reason: string }) {
  return <span className="font-medium text-destructive">Unavailable ({reason})</span>;
}

function reasonFor(unavailable: UnavailableField[], field: string): string {
  return unavailable.find((item) => item.field === field)?.reason ?? "reason not reported";
}

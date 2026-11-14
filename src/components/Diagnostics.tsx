import { useState } from "react";
import { Check, Copy, RefreshCw } from "lucide-react";
import { summarizeError } from "../lib/errors";
import { formatBytes, formatDateTime } from "../lib/format";
import { cn } from "../lib/utils";
import {
  getDiagnostics,
  type DiagnosticsOs,
  type DiagnosticsReport,
} from "../services/diagnostics";
import { Button } from "./ui/button";

type DiagnosticsState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; report: DiagnosticsReport }
  | { status: "error"; code: string; message: string };

type CopyState = "idle" | "copied" | "failed";

/**
 * Local diagnostics and a redacted support-bundle preview.
 *
 * The command is only called when the user asks for a bundle, so browsing the
 * dashboard does not read the log file. The preview never leaves the machine
 * unless the user copies it.
 */
export function Diagnostics({ className }: { className?: string }) {
  const [state, setState] = useState<DiagnosticsState>({ status: "idle" });
  const [copyState, setCopyState] = useState<CopyState>("idle");

  const generate = () => {
    setState({ status: "loading" });
    setCopyState("idle");

    getDiagnostics()
      .then((report) => setState({ status: "ready", report }))
      .catch((error: unknown) => {
        const { code, message } = summarizeError(error);
        setState({ status: "error", code, message });
      });
  };

  const copyPreview = async (preview: string) => {
    try {
      await navigator.clipboard.writeText(preview);
      setCopyState("copied");
    } catch {
      setCopyState("failed");
    }
  };

  const loading = state.status === "loading";

  return (
    <section
      aria-labelledby="diagnostics-panel-title"
      className={cn("rounded-lg border border-border bg-card p-4 shadow-sm sm:p-6", className)}
    >
      <header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <h2 id="diagnostics-panel-title" className="text-lg font-semibold tracking-tight">
            Diagnostics
          </h2>
          <p className="text-xs text-muted-foreground">
            Local only · nothing is uploaded · paths and identities are redacted
          </p>
        </div>

        <Button size="sm" onClick={generate} disabled={loading} className="self-start">
          <RefreshCw aria-hidden="true" className={cn("size-3.5", loading && "animate-spin")} />
          {loading ? "Generating…" : buttonLabel(state)}
        </Button>
      </header>

      {state.status === "idle" && (
        <p className="mt-4 max-w-3xl text-sm text-muted-foreground">
          Generate a support bundle to see app and OS versions, log size, and a redacted log tail.
          Account names, machine names, IP addresses, and file paths are replaced before the preview
          is returned.
        </p>
      )}

      {state.status === "loading" && (
        <p className="mt-4 text-sm text-muted-foreground" role="status">
          Collecting environment facts and redacting the log tail…
        </p>
      )}

      {state.status === "error" && (
        <p className="mt-4 text-sm font-medium text-destructive" role="alert">
          Diagnostics are unavailable ({state.code}). {state.message}
        </p>
      )}

      {state.status === "ready" && (
        <Report
          report={state.report}
          copyState={copyState}
          onCopy={() => copyPreview(state.report.bundle.preview)}
        />
      )}
    </section>
  );
}

function buttonLabel(state: DiagnosticsState): string {
  if (state.status === "ready") return "Regenerate";
  if (state.status === "error") return "Try again";
  return "Generate bundle";
}

function Report({
  report,
  copyState,
  onCopy,
}: {
  report: DiagnosticsReport;
  copyState: CopyState;
  onCopy: () => void;
}) {
  const { log, os, bundle } = report;
  const empty = bundle.preview.trim().length === 0;

  return (
    <div className="mt-4 space-y-4">
      <dl className="grid grid-cols-1 gap-x-6 gap-y-3 text-sm sm:grid-cols-2 lg:grid-cols-3">
        <Fact label="App" value={`${report.appVersion} · schema ${report.schemaVersion}`} />
        <Fact label="Tauri" value={report.tauriVersion} />
        <Fact label="OS" value={osSummary(os)} />
        <Fact label="Elevation" value={elevationLabel(os.elevated)} />
        <Fact
          label="Log"
          value={
            log
              ? `${formatBytes(log.sizeBytes)} across ${log.fileCount} ${
                  log.fileCount === 1 ? "file" : "files"
                }`
              : "Unavailable"
          }
        />
        <Fact
          label="Last log write"
          value={log?.modifiedAt === undefined ? "Unknown" : formatDateTime(log.modifiedAt)}
        />
      </dl>

      {report.unavailable.length > 0 && (
        <p className="text-xs text-muted-foreground">
          Some fields could not be read:{" "}
          {report.unavailable.map((item) => `${item.field} (${item.reason})`).join(", ")}.
        </p>
      )}

      <div className="space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <label
            htmlFor="support-bundle-preview"
            className="text-2xs font-semibold uppercase tracking-widest text-muted-foreground"
          >
            Redacted support bundle preview
          </label>

          <div className="flex items-center gap-2">
            {copyState === "copied" && (
              <span className="flex items-center gap-1 text-xs text-muted-foreground" role="status">
                <Check aria-hidden="true" className="size-3.5" />
                Copied
              </span>
            )}
            <Button size="sm" onClick={onCopy} disabled={empty}>
              <Copy aria-hidden="true" className="size-3.5" />
              Copy preview
            </Button>
          </div>
        </div>

        <textarea
          id="support-bundle-preview"
          readOnly
          rows={16}
          value={bundle.preview}
          spellCheck={false}
          className="w-full resize-y rounded-md border border-border bg-background p-3 font-mono text-xs leading-relaxed text-foreground"
        />

        <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>
            {bundle.redactions === 1
              ? "1 replacement made before preview"
              : `${bundle.redactions.toLocaleString()} replacements made before preview`}
          </span>
          <span>
            {bundle.logTruncated
              ? `Preview limited to the last ${formatBytes(bundle.includedLogBytes)}`
              : "Full log tail included"}
          </span>
        </div>

        {copyState === "failed" && (
          <p className="text-xs font-medium text-destructive" role="alert">
            Copy failed. Select the preview text and copy it manually.
          </p>
        )}
      </div>
    </div>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-2xs uppercase tracking-widest text-muted-foreground">{label}</dt>
      <dd className="mt-0.5 break-words">{value}</dd>
    </div>
  );
}

function osSummary(os: DiagnosticsOs): string {
  const parts = [os.name, os.version, os.build === undefined ? undefined : `build ${os.build}`];
  const known = parts.filter((part): part is string => Boolean(part));

  return known.length === 0 ? `Unknown · ${os.arch}` : `${known.join(" ")} · ${os.arch}`;
}

function elevationLabel(elevated: boolean | undefined): string {
  if (elevated === true) return "Elevated";
  if (elevated === false) return "Unelevated";
  return "Unknown";
}

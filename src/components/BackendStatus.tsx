import { useEffect, useState } from "react";
import { RotateCcw } from "lucide-react";
import { getAppInfo, type AppInfo } from "../services/app";
import { IpcError } from "../services/ipc";
import { cn } from "../lib/utils";
import { Button } from "./ui/button";

type BackendState =
  | { status: "loading" }
  | { status: "ready"; info: AppInfo }
  | { status: "error"; code: string; message: string };

function toErrorState(error: unknown): BackendState {
  if (error instanceof IpcError) {
    return { status: "error", code: error.code, message: error.message };
  }

  // Running `npm run dev` in a browser has no Tauri runtime; report honestly.
  return {
    status: "error",
    code: "unavailable",
    message: "The desktop backend is not reachable.",
  };
}

/**
 * Proves the renderer -> IPC -> Rust path with a single read-only command.
 * Shows loading, ready, and failure states instead of hiding them.
 */
export function BackendStatus({ className }: { className?: string }) {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<BackendState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    getAppInfo()
      .then((info) => {
        if (active) setState({ status: "ready", info });
      })
      .catch((error: unknown) => {
        if (active) setState(toErrorState(error));
      });

    return () => {
      active = false;
    };
  }, [attempt]);

  const retry = () => {
    setState({ status: "loading" });
    setAttempt((count) => count + 1);
  };

  return (
    <section
      aria-labelledby="backend-panel-title"
      className={cn("rounded-lg border border-border bg-card p-4 shadow-sm sm:p-6", className)}
    >
      <h2 id="backend-panel-title" className="text-lg font-semibold tracking-tight">
        Backend
      </h2>

      <div className="mt-4 flex flex-wrap items-center gap-3">
        <span
          aria-hidden="true"
          className={cn(
            "size-2.5 shrink-0 rounded-full",
            state.status === "ready" && "bg-primary",
            state.status === "loading" && "animate-pulse bg-muted-foreground",
            state.status === "error" && "bg-destructive",
          )}
        />

        <span className="text-sm" role="status" aria-live="polite">
          {state.status === "loading" && "Checking desktop backend…"}

          {state.status === "ready" &&
            `Backend ${state.info.appVersion} · ${state.info.os} ${state.info.arch}`}

          {state.status === "error" && (
            <span title={state.message}>Backend unavailable ({state.code})</span>
          )}
        </span>

        {state.status === "error" && (
          <Button size="sm" onClick={retry} className="ml-auto">
            <RotateCcw aria-hidden="true" className="size-3.5" />
            Retry
          </Button>
        )}
      </div>
    </section>
  );
}

import { useEffect, useState } from "react";
import { getAppInfo, type AppInfo } from "../services/app";
import { IpcError } from "../services/ipc";

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
export function BackendStatus() {
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
    <div className={`backend-status backend-status--${state.status}`}>
      <span className="backend-dot" aria-hidden="true" />

      <span role="status" aria-live="polite">
        {state.status === "loading" && "Checking desktop backend…"}

        {state.status === "ready" &&
          `Backend ${state.info.appVersion} · ${state.info.os} ${state.info.arch}`}

        {state.status === "error" && (
          <span title={state.message}>Backend unavailable ({state.code})</span>
        )}
      </span>

      {state.status === "error" && (
        <button type="button" className="backend-retry" onClick={retry}>
          Retry
        </button>
      )}
    </div>
  );
}

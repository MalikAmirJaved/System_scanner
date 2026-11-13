import { IpcError } from "../services/ipc";

export interface ErrorSummary {
  code: string;
  message: string;
}

/**
 * Normalizes anything a command call can throw into text a panel can render.
 *
 * `npm run dev` in a browser has no Tauri runtime, so a non-`IpcError` failure
 * is reported as an unreachable backend rather than as an empty result.
 */
export function summarizeError(error: unknown): ErrorSummary {
  if (error instanceof IpcError) {
    return { code: error.code, message: error.message };
  }

  return {
    code: "unavailable",
    message: "The desktop backend is not reachable.",
  };
}

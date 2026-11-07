import { invoke } from "@tauri-apps/api/core";

/**
 * Stable error codes mirror `src-tauri/src/error.rs`.
 * Adding or renaming a code is a breaking IPC change (see `API.md`).
 */
export type ErrorCode =
  | "invalid_input"
  | "not_permitted"
  | "access_denied"
  | "not_found"
  | "unsupported"
  | "timeout"
  | "busy"
  | "cancelled"
  | "disk_full"
  | "db_error"
  | "internal";

export interface AppErrorPayload {
  code: ErrorCode;
  message: string;
  detail?: string;
  retryable: boolean;
  correlationId: string;
}

export type CommandResult<T> =
  { status: "ok"; data: T } | { status: "err"; error: AppErrorPayload };

/** Typed wrapper around a failed command. Components switch on `code`. */
export class IpcError extends Error {
  readonly code: ErrorCode;
  readonly detail?: string;
  readonly retryable: boolean;
  readonly correlationId: string;

  constructor(payload: AppErrorPayload) {
    super(payload.message);
    this.name = "IpcError";
    this.code = payload.code;
    this.detail = payload.detail;
    this.retryable = payload.retryable;
    this.correlationId = payload.correlationId;
  }
}

/**
 * The only place that calls `invoke`.
 *
 * Unwraps the `CommandResult` envelope and throws `IpcError` for failures so
 * callers never see `unknown` or a silently missing field.
 */
export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const result = await invoke<CommandResult<T>>(command, args);

  if (result.status === "err") {
    throw new IpcError(result.error);
  }

  return result.data;
}

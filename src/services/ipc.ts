import { invoke } from "@tauri-apps/api/core";
import {
  at,
  boolean,
  object,
  oneOf,
  optionalString,
  parsePayload,
  string,
  type Validator,
} from "./validate";

export { IpcContractError } from "./validate";

/**
 * Stable error codes mirror `src-tauri/src/error.rs`.
 * Adding or renaming a code is a breaking IPC change (see `API.md`).
 */
const ERROR_CODES = [
  "invalid_input",
  "not_permitted",
  "access_denied",
  "not_found",
  "unsupported",
  "timeout",
  "busy",
  "cancelled",
  "disk_full",
  "db_error",
  "internal",
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];

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

const ENVELOPE_STATUSES = ["ok", "err"] as const;

type Envelope = { status: "ok"; data: unknown } | { status: "err"; error: AppErrorPayload };

/** Validates the `AppError` payload, including the code, before it is trusted. */
function parseAppError(value: unknown, path: string): AppErrorPayload {
  const raw = object(value, path);

  return {
    code: oneOf(ERROR_CODES, raw.code, at(path, "code")),
    message: string(raw.message, at(path, "message")),
    detail: optionalString(raw.detail, at(path, "detail")),
    retryable: boolean(raw.retryable, at(path, "retryable")),
    correlationId: string(raw.correlationId, at(path, "correlationId")),
  };
}

function parseEnvelope(value: unknown, path: string): Envelope {
  const raw = object(value, path);
  const status = oneOf(ENVELOPE_STATUSES, raw.status, at(path, "status"));

  if (status === "err") {
    return { status, error: parseAppError(raw.error, at(path, "error")) };
  }

  return { status, data: raw.data };
}

/**
 * The only place that calls `invoke`.
 *
 * Checks the envelope, then the command's own DTO, before unwrapping. Both the
 * envelope and the data are handled once so callers never see `status` or
 * `unknown`. A shape mismatch throws `IpcContractError` naming the field path;
 * a reported failure throws `IpcError`.
 */
export async function invokeCommand<T>(
  command: string,
  parse: Validator<T>,
  args?: Record<string, unknown>,
): Promise<T> {
  const envelope = parsePayload(command, parseEnvelope, await invoke<unknown>(command, args));

  if (envelope.status === "err") throw new IpcError(envelope.error);

  return parsePayload(command, parse, envelope.data);
}

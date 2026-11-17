/**
 * Runtime validation of IPC payloads (B-090).
 *
 * The Rust side is the source of truth, but a UI build can be older than the
 * backend it talks to, and a DTO can drift. Every response and event is checked
 * against the shape documented in `API.md` before a component sees it, so drift
 * fails as `IpcContractError` instead of rendering `undefined`.
 *
 * Rules:
 * - A missing optional field is `undefined`. A *present* field of the wrong type
 *   is an error — never coerced, clamped, or replaced by a default.
 * - Only known fields are copied, so an unexpected field cannot reach the UI.
 * - A reported problem names the field path, never the value. Payloads may
 *   contain machine-specific text such as paths or account names.
 *
 * Hand-written on purpose: see `API.md` §8 and the dependency policy in
 * `AGENTS.md` §5. This module has no imports and no dependency.
 */

/** Reads an untrusted value at `path`, or throws an internal contract failure. */
export type Validator<T> = (value: unknown, path: string) => T;

/**
 * A payload did not match this build's contract.
 *
 * Distinct from `IpcError`: the Rust side reported success, but the data is
 * unreadable here. Panels surface it as `contract_violation`.
 */
export class IpcContractError extends Error {
  /** Command or event name that produced the payload. */
  readonly source: string;
  /** First field path that failed, plus what was expected. */
  readonly problem: string;

  constructor(source: string, problem: string) {
    super(`The backend response for ${source} does not match this app version: ${problem}.`);
    this.name = "IpcContractError";
    this.source = source;
    this.problem = problem;
  }
}

/** Internal: a single field failure, wrapped by `parsePayload` with its source. */
class Problem extends Error {
  constructor(readonly detail: string) {
    super(detail);
    this.name = "Problem";
  }
}

/**
 * Runs one validator and attributes a failure to its command or event.
 *
 * Used at the two boundary points only: `invokeCommand` and the event listener
 * in `scan.ts`.
 */
export function parsePayload<T>(source: string, validate: Validator<T>, value: unknown): T {
  try {
    return validate(value, "");
  } catch (error) {
    if (error instanceof Problem) throw new IpcContractError(source, error.detail);
    throw error;
  }
}

/** `at("cpu", "model")` → `cpu.model`; `at("", "cpu")` → `cpu`. */
export function at(path: string, key: string): string {
  return path === "" ? key : `${path}.${key}`;
}

/** `index("disks", 0)` → `disks[0]`. */
export function index(path: string, position: number): string {
  return `${path}[${position}]`;
}

function fail(path: string, expected: string, value: unknown): never {
  throw new Problem(`${path || "the payload"} is not ${expected} (got ${describe(value)})`);
}

/** Describes the *kind* of a value. Never includes the value itself. */
function describe(value: unknown): string {
  if (value === null) return "null";
  if (Array.isArray(value)) return "an array";
  switch (typeof value) {
    case "string":
      return "a string";
    case "number":
      return Number.isFinite(value) ? "a number" : "a non-finite number";
    case "boolean":
      return "a boolean";
    case "undefined":
      return "nothing";
    case "function":
      return "a function";
    default:
      return "an object";
  }
}

/** A plain JSON object. Arrays and `null` are not objects here. */
export function object(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    fail(path, "an object", value);
  }

  return value as Record<string, unknown>;
}

export function string(value: unknown, path: string): string {
  if (typeof value !== "string") fail(path, "a string", value);
  return value;
}

/** A string with a hard length cap, for text the UI will render. */
export function text(value: unknown, path: string, maxLength: number): string {
  const result = string(value, path);

  if (result.length > maxLength) fail(path, `a string of at most ${maxLength} characters`, value);

  return result;
}

export function boolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") fail(path, "a boolean", value);
  return value;
}

/** A finite, non-negative integer. Counters and sequence numbers use this. */
export function count(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) fail(path, "a finite number", value);

  if (!Number.isInteger(value) || value < 0) fail(path, "a non-negative integer", value);

  return value;
}

export function optional<T>(value: unknown, path: string, validate: Validator<T>): T | undefined {
  return value === undefined ? undefined : validate(value, path);
}

export function optionalString(value: unknown, path: string): string | undefined {
  return optional(value, path, string);
}

export function optionalCount(value: unknown, path: string): number | undefined {
  return optional(value, path, count);
}

export function optionalBoolean(value: unknown, path: string): boolean | undefined {
  return optional(value, path, boolean);
}

/** An array with a hard item cap, so a runaway payload cannot reach the UI. */
export function listOf<T>(value: unknown, path: string, max: number, validate: Validator<T>): T[] {
  if (!Array.isArray(value)) fail(path, "an array", value);

  if (value.length > max) fail(path, `an array of at most ${max} items`, value);

  return value.map((item, position) => validate(item, index(path, position)));
}

export function optionalListOf<T>(
  value: unknown,
  path: string,
  max: number,
  validate: Validator<T>,
): T[] | undefined {
  return value === undefined ? undefined : listOf(value, path, max, validate);
}

/** A string from a fixed set, such as a state or reason code. */
export function oneOf<const T extends readonly string[]>(
  allowed: T,
  value: unknown,
  path: string,
): T[number] {
  if (typeof value === "string" && (allowed as readonly string[]).includes(value)) {
    return value as T[number];
  }

  return fail(path, `one of ${allowed.join(", ")}`, value);
}

/** An exact schema version. A bump must be mirrored in this build. */
export function version(expected: number, value: unknown, path: string): number {
  if (typeof value === "number" && value === expected) return expected;

  const actual = typeof value === "number" ? `version ${value}` : describe(value);

  throw new Problem(`${path} is ${actual}, but this build reads version ${expected}`);
}

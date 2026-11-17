import { at, object, oneOf, string, type Validator } from "./validate";

/**
 * Mirrors `src-tauri/src/dto/unavailable.rs`.
 *
 * A field the backend could not read is omitted and named here, so the UI never
 * has to guess whether `undefined` means "missing" or "zero".
 */
export type UnavailableReason = "access_denied" | "unsupported" | "failed";

const UNAVAILABLE_REASONS = ["access_denied", "unsupported", "failed"] as const;

export interface UnavailableField {
  field: string;
  reason: UnavailableReason;
}

export const parseUnavailableField: Validator<UnavailableField> = (value, path) => {
  const raw = object(value, path);

  return {
    field: string(raw.field, at(path, "field")),
    reason: oneOf(UNAVAILABLE_REASONS, raw.reason, at(path, "reason")),
  };
};

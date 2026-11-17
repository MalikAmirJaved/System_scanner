import { invokeCommand } from "./ipc";
import {
  at,
  boolean,
  count,
  listOf,
  object,
  optional,
  optionalBoolean,
  optionalCount,
  optionalString,
  string,
  text,
  version,
  type Validator,
} from "./validate";
import { parseUnavailableField, type UnavailableField } from "./unavailable";

/**
 * Mirrors `DiagnosticsDto` in `src-tauri/src/dto/diagnostics.rs`
 * (`DIAGNOSTICS_SCHEMA_VERSION = 1`).
 *
 * The preview is generated on disk only when this command runs. Nothing is
 * uploaded and no absolute path is returned.
 */
export const DIAGNOSTICS_SCHEMA_VERSION = 1;

export interface DiagnosticsOs {
  name?: string;
  version?: string;
  build?: string;
  arch: string;
  elevated?: boolean;
}

export interface DiagnosticsLog {
  sizeBytes: number;
  fileCount: number;
  modifiedAt?: number;
}

export interface BundlePreview {
  preview: string;
  redactions: number;
  logTruncated: boolean;
  includedLogBytes: number;
}

export interface DiagnosticsReport {
  generatedAt: number;
  schemaVersion: number;
  appVersion: string;
  tauriVersion: string;
  os: DiagnosticsOs;
  log?: DiagnosticsLog;
  bundle: BundlePreview;
  unavailable: UnavailableField[];
}

/** Rust caps the log tail at 256 KiB; this is a generous ceiling on the text. */
const MAX_PREVIEW_CHARS = 512 * 1_024;
const MAX_UNAVAILABLE = 256;

const parseOs: Validator<DiagnosticsOs> = (value, path) => {
  const raw = object(value, path);

  return {
    name: optionalString(raw.name, at(path, "name")),
    version: optionalString(raw.version, at(path, "version")),
    build: optionalString(raw.build, at(path, "build")),
    arch: string(raw.arch, at(path, "arch")),
    elevated: optionalBoolean(raw.elevated, at(path, "elevated")),
  };
};

const parseLog: Validator<DiagnosticsLog> = (value, path) => {
  const raw = object(value, path);

  return {
    sizeBytes: count(raw.sizeBytes, at(path, "sizeBytes")),
    fileCount: count(raw.fileCount, at(path, "fileCount")),
    modifiedAt: optionalCount(raw.modifiedAt, at(path, "modifiedAt")),
  };
};

const parseBundle: Validator<BundlePreview> = (value, path) => {
  const raw = object(value, path);

  return {
    preview: text(raw.preview, at(path, "preview"), MAX_PREVIEW_CHARS),
    redactions: count(raw.redactions, at(path, "redactions")),
    logTruncated: boolean(raw.logTruncated, at(path, "logTruncated")),
    includedLogBytes: count(raw.includedLogBytes, at(path, "includedLogBytes")),
  };
};

const parseDiagnostics: Validator<DiagnosticsReport> = (value, path) => {
  const raw = object(value, path);

  return {
    generatedAt: count(raw.generatedAt, at(path, "generatedAt")),
    schemaVersion: version(
      DIAGNOSTICS_SCHEMA_VERSION,
      raw.schemaVersion,
      at(path, "schemaVersion"),
    ),
    appVersion: string(raw.appVersion, at(path, "appVersion")),
    tauriVersion: string(raw.tauriVersion, at(path, "tauriVersion")),
    os: parseOs(raw.os, at(path, "os")),
    log: optional(raw.log, at(path, "log"), parseLog),
    bundle: parseBundle(raw.bundle, at(path, "bundle")),
    unavailable: listOf(
      raw.unavailable,
      at(path, "unavailable"),
      MAX_UNAVAILABLE,
      parseUnavailableField,
    ),
  };
};

export function getDiagnostics(): Promise<DiagnosticsReport> {
  return invokeCommand("get_diagnostics", parseDiagnostics);
}

import { invokeCommand } from "./ipc";

/**
 * Mirrors `DiagnosticsDto` in `src-tauri/src/dto/diagnostics.rs`
 * (`DIAGNOSTICS_SCHEMA_VERSION = 1`).
 *
 * The preview is generated on disk only when this command runs. Nothing is
 * uploaded and no absolute path is returned.
 */
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
  unavailable: { field: string; reason: string }[];
}

export function getDiagnostics(): Promise<DiagnosticsReport> {
  return invokeCommand<DiagnosticsReport>("get_diagnostics");
}

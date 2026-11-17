import { invokeCommand } from "./ipc";
import { at, boolean, object, string, version, type Validator } from "./validate";

/**
 * Mirrors `AppInfo` in `src-tauri/src/dto/app_info.rs`.
 * The version must match `DTO_SCHEMA_VERSION` there.
 */
export const APP_INFO_SCHEMA_VERSION = 1;

export interface AppInfo {
  appName: string;
  appVersion: string;
  tauriVersion: string;
  os: string;
  arch: string;
  debug: boolean;
  schemaVersion: number;
}

const parseAppInfo: Validator<AppInfo> = (value, path) => {
  const raw = object(value, path);

  return {
    appName: string(raw.appName, at(path, "appName")),
    appVersion: string(raw.appVersion, at(path, "appVersion")),
    tauriVersion: string(raw.tauriVersion, at(path, "tauriVersion")),
    os: string(raw.os, at(path, "os")),
    arch: string(raw.arch, at(path, "arch")),
    debug: boolean(raw.debug, at(path, "debug")),
    schemaVersion: version(APP_INFO_SCHEMA_VERSION, raw.schemaVersion, at(path, "schemaVersion")),
  };
};

export function getAppInfo(): Promise<AppInfo> {
  return invokeCommand("get_app_info", parseAppInfo);
}

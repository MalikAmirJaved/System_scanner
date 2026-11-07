import { invokeCommand } from "./ipc";

/** Mirrors `AppInfo` in `src-tauri/src/dto/app_info.rs`. */
export interface AppInfo {
  appName: string;
  appVersion: string;
  tauriVersion: string;
  os: string;
  arch: string;
  debug: boolean;
  schemaVersion: number;
}

export function getAppInfo(): Promise<AppInfo> {
  return invokeCommand<AppInfo>("get_app_info");
}

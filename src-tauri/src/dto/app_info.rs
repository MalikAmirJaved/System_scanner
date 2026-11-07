use serde::Serialize;

/// Bump when a breaking field change is made to any DTO in this module.
pub const DTO_SCHEMA_VERSION: u32 = 1;

/// Build and runtime facts about the desktop backend.
///
/// Read-only. Contains no machine-identifying data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_name: &'static str,
    pub app_version: &'static str,
    pub tauri_version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub debug: bool,
    pub schema_version: u32,
}

impl AppInfo {
    pub fn current() -> Self {
        Self {
            app_name: env!("CARGO_PKG_NAME"),
            app_version: env!("CARGO_PKG_VERSION"),
            tauri_version: tauri::VERSION,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            debug: cfg!(debug_assertions),
            schema_version: DTO_SCHEMA_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_serializes_with_camel_case_and_schema_version() {
        let value = serde_json::to_value(AppInfo::current()).unwrap();

        assert_eq!(value["appName"], env!("CARGO_PKG_NAME"));
        assert_eq!(value["appVersion"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["tauriVersion"], tauri::VERSION);
        assert_eq!(value["schemaVersion"], DTO_SCHEMA_VERSION);
        assert!(value.get("app_version").is_none());
    }
}

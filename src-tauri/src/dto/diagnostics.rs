use serde::Serialize;

use crate::domain::diagnostics::{BundleEnvironment, SupportBundle};
use crate::dto::unavailable::UnavailableFieldDto;

/// Bump when a breaking field change is made to this DTO.
pub const DIAGNOSTICS_SCHEMA_VERSION: u32 = 1;

/// Wire shape of `get_diagnostics` data (see `API.md` §3.4).
///
/// The report is local-only: the bundle is generated on request and nothing is
/// uploaded. A missing leaf field is paired with an entry in `unavailable`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsDto {
    pub generated_at: u64,
    pub schema_version: u32,
    pub app_version: String,
    pub tauri_version: String,
    pub os: OsDiagnosticsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<LogSummaryDto>,
    pub bundle: BundlePreviewDto,
    pub unavailable: Vec<UnavailableFieldDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsDiagnosticsDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    pub arch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSummaryDto {
    pub size_bytes: u64,
    pub file_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePreviewDto {
    pub preview: String,
    pub redactions: usize,
    pub log_truncated: bool,
    pub included_log_bytes: usize,
}

impl DiagnosticsDto {
    pub fn new(
        generated_at: u64,
        environment: &BundleEnvironment,
        log: Option<LogSummaryDto>,
        bundle: SupportBundle,
        unavailable: Vec<UnavailableFieldDto>,
    ) -> Self {
        Self {
            generated_at,
            schema_version: DIAGNOSTICS_SCHEMA_VERSION,
            app_version: environment.app_version.clone(),
            tauri_version: environment.tauri_version.clone(),
            os: OsDiagnosticsDto {
                name: environment.os_name.clone(),
                version: environment.os_version.clone(),
                build: environment.os_build.clone(),
                arch: environment.arch.clone(),
                elevated: environment.elevated,
            },
            log,
            bundle: BundlePreviewDto {
                preview: bundle.preview,
                redactions: bundle.redactions,
                log_truncated: bundle.log_truncated,
                included_log_bytes: bundle.included_log_bytes,
            },
            unavailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> BundleEnvironment {
        BundleEnvironment {
            app_version: "0.1.0".to_owned(),
            tauri_version: "2.12.5".to_owned(),
            schema_version: 1,
            os_name: Some("Windows 11 Pro".to_owned()),
            os_version: Some("10.0.26100".to_owned()),
            os_build: Some("26100".to_owned()),
            arch: "x64".to_owned(),
            elevated: Some(false),
            log_file_name: "system-scanner.log",
            log_bytes: 1_234,
            log_files: 2,
            log_modified_ms: Some(1_700_000_000_000),
        }
    }

    #[test]
    fn serializes_camel_case_with_the_bundle_preview() {
        let value = serde_json::to_value(DiagnosticsDto::new(
            1_700_000_000_000,
            &environment(),
            Some(LogSummaryDto {
                size_bytes: 1_234,
                file_count: 2,
                modified_at: Some(1_700_000_000_000),
            }),
            SupportBundle {
                preview: "bundle text".to_owned(),
                redactions: 3,
                log_truncated: false,
                included_log_bytes: 42,
            },
            vec![],
        ))
        .unwrap();

        assert_eq!(value["schemaVersion"], DIAGNOSTICS_SCHEMA_VERSION);
        assert_eq!(value["appVersion"], "0.1.0");
        assert_eq!(value["tauriVersion"], "2.12.5");
        assert_eq!(value["os"]["name"], "Windows 11 Pro");
        assert_eq!(value["os"]["elevated"], false);
        assert_eq!(value["log"]["sizeBytes"], 1_234);
        assert_eq!(value["log"]["fileCount"], 2);
        assert_eq!(value["bundle"]["preview"], "bundle text");
        assert_eq!(value["bundle"]["includedLogBytes"], 42);
        assert!(value.get("app_version").is_none());
        assert!(value["os"].get("edition").is_none());
    }

    #[test]
    fn a_missing_log_group_is_omitted_and_explained() {
        let value = serde_json::to_value(DiagnosticsDto::new(
            0,
            &environment(),
            None,
            SupportBundle {
                preview: String::new(),
                redactions: 0,
                log_truncated: false,
                included_log_bytes: 0,
            },
            vec![UnavailableFieldDto {
                field: "log".to_owned(),
                reason: "failed",
            }],
        ))
        .unwrap();

        assert!(value.get("log").is_none());
        assert_eq!(
            value["unavailable"],
            serde_json::json!([{ "field": "log", "reason": "failed" }])
        );
    }
}

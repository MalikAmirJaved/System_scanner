use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::adapters::filesystem::traversal::{MAX_ROOT_BYTES, MAX_ROOTS};
use crate::domain::traversal::{Completeness, TraversalSummary};
use crate::error::AppError;

/// Hard ceiling on entries one scan may produce. An omitted `maxEntries` means
/// this ceiling, so no scan is unbounded.
pub const MAX_SCAN_ENTRIES: u64 = 100_000_000;

/// Input of `start_scan` (see `API.md` §4). Untrusted: validated by
/// [`StartScanRequest::parse`] before any filesystem access.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartScanRequest {
    pub roots: Vec<String>,
    #[serde(default)]
    pub max_entries: Option<u64>,
}

/// A validated request. Paths are still unverified against the disk.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedScanRequest {
    pub roots: Vec<PathBuf>,
    pub max_entries: u64,
}

impl StartScanRequest {
    /// Shape checks only. The messages never echo the offending value.
    pub fn parse(self) -> Result<ParsedScanRequest, &'static str> {
        if self.roots.is_empty() {
            return Err("at least one root is required");
        }
        if self.roots.len() > MAX_ROOTS {
            return Err("too many roots");
        }

        let mut roots = Vec::with_capacity(self.roots.len());
        for root in self.roots {
            if root.is_empty() {
                return Err("roots must not be empty");
            }
            if root.contains('\0') {
                return Err("roots must not contain NUL characters");
            }
            if root.len() > MAX_ROOT_BYTES {
                return Err("a root path is too long");
            }
            if is_device_namespace(&root) {
                return Err("device and NT-namespace paths are not allowed");
            }
            roots.push(PathBuf::from(root));
        }

        let max_entries = match self.max_entries {
            None => MAX_SCAN_ENTRIES,
            Some(0) => return Err("maxEntries must be at least 1"),
            Some(value) if value > MAX_SCAN_ENTRIES => {
                return Err("maxEntries exceeds the allowed ceiling");
            }
            Some(value) => value,
        };

        Ok(ParsedScanRequest { roots, max_entries })
    }
}

/// Whether a root addresses a device or NT object namespace instead of a
/// filesystem location.
///
/// Rejected: `\\.\...` (device namespace), `\??\...` (NT namespace), and any
/// `\\?\` path that is not a drive (`\\?\C:\`) or UNC (`\\?\UNC\server\share`)
/// form, such as `\\?\GLOBALROOT\...` or `\\?\Volume{...}\`. Plain drives, plain
/// UNC shares, and the extended-length drive/UNC forms stay allowed. Pure string
/// logic, so it behaves the same on every host and in tests.
fn is_device_namespace(root: &str) -> bool {
    let normalized = root.replace('/', "\\");

    if normalized.starts_with("\\??\\") || normalized == "\\??" {
        return true;
    }
    if normalized == "\\\\." || normalized.starts_with("\\\\.\\") {
        return true;
    }

    let Some(rest) = normalized.strip_prefix("\\\\?\\") else {
        return normalized == "\\\\?";
    };

    let bytes = rest.as_bytes();
    let is_drive = bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes.len() == 2 || bytes[2] == b'\\');
    let is_unc = rest
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UNC\\"));

    !(is_drive || is_unc)
}

/// Data of `start_scan`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanDto {
    pub job_id: String,
}

/// Data of `cancel_scan`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelScanDto {
    pub accepted: bool,
}

/// Aggregate counters only. No path, name, or entry list ever leaves Rust.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCountersDto {
    pub entries: u64,
    pub files: u64,
    pub directories: u64,
    pub reparse_points: u64,
    pub bytes: u64,
    pub skipped: u64,
    pub denied: u64,
    pub disappeared: u64,
    pub unsupported: u64,
    pub failed: u64,
}

impl From<&TraversalSummary> for ScanCountersDto {
    fn from(summary: &TraversalSummary) -> Self {
        Self {
            entries: summary.entries,
            files: summary.files,
            directories: summary.directories,
            reparse_points: summary.reparse_points,
            bytes: summary.bytes,
            skipped: summary.skipped,
            denied: summary.denied,
            disappeared: summary.disappeared,
            unsupported: summary.unsupported,
            failed: summary.failed,
        }
    }
}

/// Data of `get_scan_status`: the authoritative job state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatusDto {
    pub job_id: String,
    /// `queued`, `running`, `cancelling`, `completed`, `failed`, `cancelled`.
    pub state: &'static str,
    /// Sequence of the last progress event sent for this job.
    pub seq: u64,
    pub elapsed_ms: u64,
    pub counters: ScanCountersDto,
    /// Set once the job is terminal and the walk produced a result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completeness: Option<&'static str>,
    /// Stable error code when the job failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<&'static str>,
}

/// An event sent to the UI. Payloads follow `API.md` §4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanEvent {
    Progress(ScanProgressEvent),
    Completed(ScanCompletedEvent),
    Failed(ScanFailedEvent),
    Cancelled(ScanCancelledEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressEvent {
    pub job_id: String,
    pub seq: u64,
    pub phase: &'static str,
    pub files_seen: u64,
    pub bytes_seen: u64,
    pub skipped: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCompletedEvent {
    pub job_id: String,
    pub summary: ScanCountersDto,
    pub completeness: &'static str,
    /// `true` unless the completeness is `complete`.
    pub partial: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFailedEvent {
    pub job_id: String,
    pub error_code: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCancelledEvent {
    pub job_id: String,
    pub reason: &'static str,
    pub summary: ScanCountersDto,
    pub elapsed_ms: u64,
}

impl ScanEvent {
    /// Event name on the wire. Names are published in `API.md`.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Progress(_) => "scan:progress",
            Self::Completed(_) => "scan:completed",
            Self::Failed(_) => "scan:failed",
            Self::Cancelled(_) => "scan:cancelled",
        }
    }

    pub fn payload(&self) -> serde_json::Value {
        let value = match self {
            Self::Progress(payload) => serde_json::to_value(payload),
            Self::Completed(payload) => serde_json::to_value(payload),
            Self::Failed(payload) => serde_json::to_value(payload),
            Self::Cancelled(payload) => serde_json::to_value(payload),
        };
        value.unwrap_or(serde_json::Value::Null)
    }
}

impl ScanCompletedEvent {
    pub fn new(
        job_id: String,
        summary: &TraversalSummary,
        completeness: Completeness,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            job_id,
            summary: ScanCountersDto::from(summary),
            completeness: completeness.code(),
            partial: completeness != Completeness::Complete,
            elapsed_ms,
        }
    }
}

impl ScanFailedEvent {
    pub fn new(job_id: String, error: &AppError) -> Self {
        Self {
            job_id,
            error_code: error.code.as_str(),
            message: error.code.default_message(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(roots: &[&str], max_entries: Option<u64>) -> StartScanRequest {
        StartScanRequest {
            roots: roots.iter().map(|root| (*root).to_owned()).collect(),
            max_entries,
        }
    }

    #[test]
    fn a_valid_request_defaults_to_the_entry_ceiling() {
        let parsed = request(&[r"C:\data"], None).parse().expect("valid");
        assert_eq!(parsed.roots, vec![PathBuf::from(r"C:\data")]);
        assert_eq!(parsed.max_entries, MAX_SCAN_ENTRIES);
    }

    #[test]
    fn malformed_requests_are_rejected_without_echoing_input() {
        let long = "x".repeat(MAX_ROOT_BYTES + 1);
        let many: Vec<&str> = vec!["C:\\a"; MAX_ROOTS + 1];

        for (input, expected) in [
            (request(&[], None), "at least one root is required"),
            (request(&many, None), "too many roots"),
            (request(&[""], None), "roots must not be empty"),
            (
                request(&["C:\\bad\0name"], None),
                "roots must not contain NUL characters",
            ),
            (request(&[&long], None), "a root path is too long"),
            (
                request(&["C:\\a"], Some(0)),
                "maxEntries must be at least 1",
            ),
            (
                request(&["C:\\a"], Some(MAX_SCAN_ENTRIES + 1)),
                "maxEntries exceeds the allowed ceiling",
            ),
        ] {
            let message = input.parse().expect_err("must be rejected");
            assert_eq!(message, expected);
            assert!(!message.contains("bad"));
        }
    }

    #[test]
    fn device_and_nt_namespace_roots_are_rejected() {
        for root in [
            r"\\.\PhysicalDrive0",
            r"\\.\C:",
            "//./C:/",
            r"\\.",
            r"\??\C:\",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1",
            r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\",
            r"\\?\pipe\name",
            r"\\?\",
            r"\\?\1:\",
        ] {
            assert_eq!(
                request(&[root], None).parse().unwrap_err(),
                "device and NT-namespace paths are not allowed",
                "{root} must be refused"
            );
        }
    }

    #[test]
    fn drives_shares_and_extended_length_forms_stay_allowed() {
        for root in [
            r"C:\",
            r"C:\Users\someone",
            r"\\server\share\folder",
            r"\\?\C:\very\long\path",
            r"\\?\c:",
            r"\\?\UNC\server\share",
            r"\\?\unc\server\share",
            r"D:/forward/slashes",
        ] {
            assert!(
                request(&[root], None).parse().is_ok(),
                "{root} must be accepted"
            );
        }
    }

    #[test]
    fn requests_use_camel_case_and_reject_unknown_fields() {
        let ok: StartScanRequest =
            serde_json::from_str(r#"{"roots":["C:\\a"],"maxEntries":5}"#).expect("camelCase");
        assert_eq!(ok.max_entries, Some(5));

        assert!(serde_json::from_str::<StartScanRequest>(r#"{"roots":[],"extra":1}"#).is_err());
        assert!(serde_json::from_str::<StartScanRequest>(r#"{"max_entries":1}"#).is_err());
    }

    #[test]
    fn event_names_and_payloads_match_the_published_contract() {
        let progress = ScanEvent::Progress(ScanProgressEvent {
            job_id: "job-1".into(),
            seq: 3,
            phase: "traverse",
            files_seen: 4,
            bytes_seen: 5,
            skipped: 1,
            elapsed_ms: 6,
        });
        assert_eq!(progress.name(), "scan:progress");
        let value = progress.payload();
        assert_eq!(value["jobId"], "job-1");
        assert_eq!(value["filesSeen"], 4);
        assert_eq!(value["elapsedMs"], 6);

        let completed = ScanEvent::Completed(ScanCompletedEvent::new(
            "job-1".into(),
            &TraversalSummary {
                denied: 1,
                ..TraversalSummary::default()
            },
            Completeness::Partial,
            9,
        ));
        assert_eq!(completed.name(), "scan:completed");
        assert_eq!(completed.payload()["partial"], true);
        assert_eq!(completed.payload()["completeness"], "partial");
        assert_eq!(completed.payload()["summary"]["denied"], 1);
    }

    #[test]
    fn status_omits_unset_optional_fields_and_never_carries_paths() {
        let status = ScanStatusDto {
            job_id: "job-1".into(),
            state: "running",
            seq: 0,
            elapsed_ms: 1,
            counters: ScanCountersDto::default(),
            completeness: None,
            error_code: None,
        };
        let value = serde_json::to_value(status).expect("serializes");
        assert!(value.get("completeness").is_none());
        assert!(value.get("errorCode").is_none());
        assert_eq!(value["state"], "running");
    }
}

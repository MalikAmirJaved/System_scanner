//! Support bundle assembly.
//!
//! Platform-neutral: the Windows adapter and the command layer supply plain
//! values, and this module renders the text the user previews and copies. No
//! account name, machine name, or path is added on purpose; the log tail is
//! untrusted and passes through [`crate::redact::Redactor`].

use std::fmt::Write as _;

use crate::redact::Redactor;

/// Maximum number of log bytes included in one bundle.
///
/// The bundle is meant to be pasted into a support message, so it is capped
/// well below the 10 MB log file.
pub const MAX_LOG_TAIL_BYTES: usize = 256 * 1024;

/// Everything the bundle header needs. Never contains secrets by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleEnvironment {
    pub app_version: String,
    pub tauri_version: String,
    pub schema_version: u32,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_build: Option<String>,
    pub arch: String,
    pub elevated: Option<bool>,
    pub log_file_name: &'static str,
    pub log_bytes: u64,
    pub log_files: usize,
    pub log_modified_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportBundle {
    pub preview: String,
    pub redactions: usize,
    pub log_truncated: bool,
    pub included_log_bytes: usize,
}

/// Renders the bundle and applies redaction to the finished text.
///
/// Redaction runs once over the header and the log tail together, which keeps
/// the replacement count accurate and catches a machine name that only appears
/// in an environment string.
pub fn build_bundle(
    environment: &BundleEnvironment,
    generated_at_ms: u64,
    log_text: &str,
    log_truncated: bool,
    redactor: &Redactor,
) -> SupportBundle {
    let mut raw = String::with_capacity(log_text.len() + 1024);

    writeln!(raw, "System Scanner support bundle").expect("writing to a String cannot fail");
    writeln!(raw, "Generated: {}", format_utc_timestamp(generated_at_ms))
        .expect("writing to a String cannot fail");
    writeln!(
        raw,
        "App: {} (schema {})",
        environment.app_version, environment.schema_version
    )
    .expect("writing to a String cannot fail");
    writeln!(raw, "Tauri: {}", environment.tauri_version).expect("writing to a String cannot fail");
    writeln!(raw, "OS: {}", os_line(environment)).expect("writing to a String cannot fail");
    writeln!(raw, "Elevation: {}", elevation_line(environment.elevated))
        .expect("writing to a String cannot fail");
    writeln!(
        raw,
        "Log: {}, {} bytes across {} files",
        environment.log_file_name, environment.log_bytes, environment.log_files
    )
    .expect("writing to a String cannot fail");
    writeln!(
        raw,
        "Log last written: {}",
        environment
            .log_modified_ms
            .map(format_utc_timestamp)
            .unwrap_or_else(|| "unknown".to_owned())
    )
    .expect("writing to a String cannot fail");
    writeln!(raw, "--- log tail ---").expect("writing to a String cannot fail");

    if log_text.is_empty() {
        writeln!(raw, "(no log entries)").expect("writing to a String cannot fail");
    } else {
        if log_truncated {
            writeln!(
                raw,
                "(earlier entries omitted; last {} bytes included)",
                log_text.len()
            )
            .expect("writing to a String cannot fail");
        }
        raw.push_str(log_text);
        if !log_text.ends_with('\n') {
            raw.push('\n');
        }
    }

    let redaction = redactor.apply(&raw);

    SupportBundle {
        preview: redaction.text,
        redactions: redaction.replacements,
        log_truncated,
        included_log_bytes: log_text.len(),
    }
}

fn os_line(environment: &BundleEnvironment) -> String {
    let mut parts = Vec::new();

    if let Some(name) = &environment.os_name {
        parts.push(name.clone());
    }
    if let Some(version) = &environment.os_version {
        parts.push(version.clone());
    }
    if let Some(build) = &environment.os_build {
        parts.push(format!("build {build}"));
    }

    if parts.is_empty() {
        return format!("unknown ({})", environment.arch);
    }

    parts.push(format!("({})", environment.arch));
    parts.join(" ")
}

fn elevation_line(elevated: Option<bool>) -> &'static str {
    match elevated {
        Some(true) => "elevated",
        Some(false) => "unelevated",
        None => "unknown",
    }
}

/// Formats Unix milliseconds as a readable UTC timestamp without a date crate.
pub fn format_utc_timestamp(milliseconds: u64) -> String {
    let seconds = (milliseconds / 1000) as i64;
    let millis = milliseconds % 1000;
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);

    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    let second = remainder % 60;

    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03} UTC")
}

/// Howard Hinnant's `civil_from_days`, which is valid for the whole range this
/// app can produce.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let shifted = days_since_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };

    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::redact::Secret;

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
    fn formats_unix_milliseconds_as_utc_without_a_date_dependency() {
        assert_eq!(format_utc_timestamp(0), "1970-01-01 00:00:00.000 UTC");
        assert_eq!(
            format_utc_timestamp(1_700_000_000_000),
            "2023-11-14 22:13:20.000 UTC"
        );
    }

    #[test]
    fn bundle_states_the_environment_and_redacts_the_log_tail() {
        let redactor = Redactor::new([
            Secret::new("ada", "<user>"),
            Secret::new("DESKTOP-ABC", "<host>"),
        ]);

        let bundle = build_bundle(
            &environment(),
            1_700_000_000_000,
            "user ada on DESKTOP-ABC opened C:\\Users\\ada\\secret.txt\n",
            false,
            &redactor,
        );

        assert!(bundle.preview.contains("App: 0.1.0 (schema 1)"));
        assert!(
            bundle
                .preview
                .contains("OS: Windows 11 Pro 10.0.26100 build 26100 (x64)")
        );
        assert!(bundle.preview.contains("Elevation: unelevated"));
        assert!(bundle.preview.contains("%USERPROFILE%\\secret.txt"));
        assert!(!bundle.preview.contains("ada"));
        assert!(!bundle.preview.contains("DESKTOP-ABC"));
        assert!(bundle.redactions >= 3, "{}", bundle.preview);
        assert_eq!(bundle.included_log_bytes, 55);
        assert!(!bundle.log_truncated);
    }

    #[test]
    fn empty_logs_are_explicit_and_truncation_is_stated() {
        let bundle = build_bundle(&environment(), 0, "", false, &Redactor::default());

        assert!(bundle.preview.contains("(no log entries)"));
        assert_eq!(bundle.included_log_bytes, 0);

        let truncated = build_bundle(&environment(), 0, "kept line\n", true, &Redactor::default());

        assert!(truncated.preview.contains("earlier entries omitted"));
        assert!(truncated.preview.contains("kept line"));
        assert!(truncated.log_truncated);
    }

    #[test]
    fn unknown_environment_values_are_labelled() {
        let mut environment = environment();
        environment.os_name = None;
        environment.os_version = None;
        environment.os_build = None;
        environment.elevated = None;

        let bundle = build_bundle(&environment, 0, "", false, &Redactor::default());

        assert!(bundle.preview.contains("OS: unknown (x64)"));
        assert!(bundle.preview.contains("Elevation: unknown"));
    }
}

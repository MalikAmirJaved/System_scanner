use crate::domain::diagnostics::{BundleEnvironment, MAX_LOG_TAIL_BYTES, build_bundle};
use crate::domain::system::{Availability, UnavailableReason};
use crate::dto::diagnostics::{DIAGNOSTICS_SCHEMA_VERSION, DiagnosticsDto, LogSummaryDto};
use crate::dto::unavailable::UnavailableFieldDto;
use crate::error::{AppError, ErrorCode};
use crate::ipc::CommandResult;
use crate::logging;
use crate::ports::diagnostics::LocalIdentity;
use crate::redact::{Redactor, Secret};

/// Builds a local diagnostics report and a redacted support-bundle preview.
///
/// The command reads the log directory, so it runs on a blocking thread. It
/// takes no arguments and never writes a file or opens a network connection.
/// The returned preview contains no account name, machine name, or full path.
#[tauri::command]
pub async fn get_diagnostics(app: tauri::AppHandle) -> CommandResult<DiagnosticsDto> {
    let result = run(app).await;
    result.log_outcome("get_diagnostics")
}

async fn run(app: tauri::AppHandle) -> CommandResult<DiagnosticsDto> {
    match tauri::async_runtime::spawn_blocking(move || collect(&app)).await {
        Ok(Ok(dto)) => CommandResult::ok(dto),
        Ok(Err(error)) => CommandResult::err(error),
        // The join error text can contain a panic message; do not forward it.
        Err(_) => CommandResult::err(
            AppError::new(ErrorCode::Internal).with_detail("diagnostics task did not complete"),
        ),
    }
}

fn collect(app: &tauri::AppHandle) -> Result<DiagnosticsDto, AppError> {
    use tauri::Manager;

    let generated_at = crate::commands::now_ms()?;
    let mut unavailable = Vec::new();

    let (os_name, os_version, os_build, arch) = collect_os(&mut unavailable);
    let (identity, elevated) = collect_identity();
    if elevated.is_none() {
        unavailable.push(gap("os.elevated"));
    }

    let (summary, tail, tail_failed) = match app.path().app_log_dir() {
        Ok(dir) => {
            let summary = logging::summary(&dir).ok().flatten();
            let (tail, tail_failed) = match logging::tail(&dir, MAX_LOG_TAIL_BYTES) {
                Ok(tail) => (tail, false),
                Err(_) => (None, true),
            };
            (summary, tail, tail_failed)
        }
        Err(_) => (None, None, false),
    };

    if summary.is_none() {
        unavailable.push(gap("log"));
    }
    if tail_failed {
        unavailable.push(gap("log.tail"));
    }

    let mut environment = BundleEnvironment {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        tauri_version: tauri::VERSION.to_owned(),
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        os_name,
        os_version,
        os_build,
        arch,
        elevated,
        log_file_name: logging::LOG_FILE_NAME,
        log_bytes: 0,
        log_files: 0,
        log_modified_ms: None,
    };

    if let Some(summary) = &summary {
        environment.log_bytes = summary.size_bytes;
        environment.log_files = summary.file_count;
        environment.log_modified_ms = summary.modified_ms;
    }

    let log_text = tail.as_ref().map_or("", |tail| tail.text.as_str());
    let log_truncated = tail.as_ref().is_some_and(|tail| tail.truncated);

    let redactor = Redactor::new([
        Secret::new(identity.user_name.unwrap_or_default(), "<user>"),
        Secret::new(identity.computer_name.unwrap_or_default(), "<host>"),
    ]);

    let bundle = build_bundle(
        &environment,
        generated_at,
        log_text,
        log_truncated,
        &redactor,
    );
    let log = summary.map(|summary| LogSummaryDto {
        size_bytes: summary.size_bytes,
        file_count: summary.file_count,
        modified_at: summary.modified_ms,
    });

    Ok(DiagnosticsDto::new(
        generated_at,
        &environment,
        log,
        bundle,
        unavailable,
    ))
}

fn gap(field: &str) -> UnavailableFieldDto {
    UnavailableFieldDto {
        field: field.to_owned(),
        reason: UnavailableReason::Failed.as_str(),
    }
}

fn leaf(
    field: &str,
    value: &Availability<String>,
    unavailable: &mut Vec<UnavailableFieldDto>,
) -> Option<String> {
    match value {
        Availability::Available(value) => Some(value.clone()),
        Availability::Unavailable(reason) => {
            unavailable.push(UnavailableFieldDto {
                field: field.to_owned(),
                reason: reason.as_str(),
            });
            None
        }
    }
}

#[cfg(windows)]
fn collect_os(
    unavailable: &mut Vec<UnavailableFieldDto>,
) -> (Option<String>, Option<String>, Option<String>, String) {
    use crate::adapters::windows::system_info::WindowsSystemInfo;
    use crate::ports::system_info::SystemInfoPort;

    let info = match WindowsSystemInfo.os_info() {
        Ok(info) => info,
        Err(_) => {
            unavailable.push(gap("os"));
            return (None, None, None, std::env::consts::ARCH.to_owned());
        }
    };

    (
        leaf("os.name", &info.name, unavailable),
        leaf("os.version", &info.version, unavailable),
        leaf("os.build", &info.build, unavailable),
        info.arch,
    )
}

#[cfg(not(windows))]
fn collect_os(
    unavailable: &mut Vec<UnavailableFieldDto>,
) -> (Option<String>, Option<String>, Option<String>, String) {
    unavailable.push(gap("os"));
    (None, None, None, std::env::consts::ARCH.to_owned())
}

#[cfg(windows)]
fn collect_identity() -> (LocalIdentity, Option<bool>) {
    use crate::adapters::windows::diagnostics::WindowsDiagnostics;
    use crate::ports::diagnostics::DiagnosticsPort;

    let probe = WindowsDiagnostics;
    (probe.identity(), probe.is_elevated())
}

#[cfg(not(windows))]
fn collect_identity() -> (LocalIdentity, Option<bool>) {
    let identity = LocalIdentity {
        user_name: std::env::var("USERNAME").ok(),
        computer_name: std::env::var("COMPUTERNAME").ok(),
    };

    (identity, None)
}

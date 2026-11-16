//! Last-resort reporting when the app cannot start.
//!
//! This path runs after Tauri's event loop failed, so there is no window, no
//! IPC, and possibly no log file. Everything here must be dependency-free and
//! must not panic: it is the only chance to tell the user what happened.
//!
//! The message is assembled from the app version and a redacted error detail.
//! The raw detail is not forwarded, because it can contain a path (KI-043).

/// Guideline shown under every fatal message. Kept static so the dialog stays
/// short and cannot contain user-specific data.
const NEXT_STEPS: &str = "\
Next steps:
1. Close any other running copy of System Scanner and start it again.
2. If it keeps failing, reinstall or update System Scanner.
3. Report the problem with this message. The log file is system-scanner.log in
   the app log folder; it records what happened before the window failed.";

/// Builds the dialog text. Pure, so the wording and redaction are testable.
pub fn message(error_detail: &str) -> String {
    let detail = crate::redact::redact_text(error_detail);
    let detail = truncate(&detail, 400);

    format!(
        "System Scanner {} could not start.\n\n{}\n\n{NEXT_STEPS}",
        env!("CARGO_PKG_VERSION"),
        detail
    )
}

/// Shows the fatal message and exits with a non-zero status.
///
/// Returning `!` keeps the caller honest: there is no recovery path from here.
pub fn report_fatal(error_detail: &str) -> ! {
    let text = message(error_detail);
    show(&text);
    std::process::exit(1);
}

/// Non-Windows hosts have no dialog; the text still reaches the console so a
/// developer sees the same message.
#[cfg(windows)]
fn show(text: &str) {
    crate::adapters::windows::dialog::show_error("System Scanner — could not start", text);
}

#[cfg(not(windows))]
fn show(text: &str) {
    eprintln!("{text}");
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    let mut shortened: String = text.chars().take(max_chars).collect();
    shortened.push('…');
    shortened
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_names_the_version_and_the_next_steps() {
        let text = message("could not create the main window");

        assert!(text.starts_with(&format!(
            "System Scanner {} could not start.",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(text.contains("could not create the main window"));
        assert!(text.contains("Next steps:"));
        assert!(text.contains("system-scanner.log"));
    }

    #[test]
    fn message_redacts_paths_and_bounds_the_detail() {
        let text = message("failed to open C:\\Users\\ada\\AppData\\Roaming\\state.json");

        assert!(!text.contains("ada"));
        assert!(text.contains("%USERPROFILE%\\AppData\\Roaming\\state.json"));

        let long = "x".repeat(1_000);
        let bounded = message(&long);
        assert!(bounded.contains('…'));
        assert!(!bounded.contains(&"x".repeat(401)));
    }
}

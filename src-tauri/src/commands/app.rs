use crate::dto::app_info::AppInfo;
use crate::ipc::CommandResult;

/// Returns build and runtime information about the desktop backend.
///
/// This is the reference implementation of the IPC envelope described in
/// `API.md`: read-only, no input, no OS handle crosses the boundary.
#[tauri::command]
pub fn get_app_info() -> CommandResult<AppInfo> {
    CommandResult::ok(AppInfo::current()).log_outcome("get_app_info")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_app_info_returns_ok() {
        match get_app_info() {
            CommandResult::Ok { data } => {
                assert_eq!(data.app_version, env!("CARGO_PKG_VERSION"));
                assert_eq!(data.schema_version, 1);
            }
            CommandResult::Err { error } => panic!("unexpected error: {error}"),
        }
    }
}

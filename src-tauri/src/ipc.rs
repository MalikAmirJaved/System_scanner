use crate::error::AppError;
use serde::Serialize;

/// Every command returns this envelope, including failures.
///
/// Wire shape (see `API.md`):
/// `{ "status": "ok", "data": ... }` or `{ "status": "err", "error": ... }`
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CommandResult<T> {
    Ok { data: T },
    Err { error: AppError },
}

impl<T> CommandResult<T> {
    pub fn ok(data: T) -> Self {
        Self::Ok { data }
    }

    pub fn err(error: AppError) -> Self {
        Self::Err { error }
    }
}

impl<T> From<Result<T, AppError>> for CommandResult<T> {
    fn from(result: Result<T, AppError>) -> Self {
        match result {
            Ok(data) => Self::ok(data),
            Err(error) => Self::err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn ok_envelope_has_status_and_data() {
        let value = serde_json::to_value(CommandResult::ok(7)).unwrap();

        assert_eq!(value, serde_json::json!({ "status": "ok", "data": 7 }));
    }

    #[test]
    fn err_envelope_uses_stable_code_and_camel_case() {
        let error = AppError::new(ErrorCode::AccessDenied);
        let value = serde_json::to_value(CommandResult::<()>::err(error)).unwrap();

        assert_eq!(value["status"], "err");
        assert_eq!(value["error"]["code"], "access_denied");
        assert_eq!(value["error"]["retryable"], true);
        assert!(
            value["error"]["correlationId"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
        );
        assert!(value["error"].get("detail").is_none());
    }

    #[test]
    fn result_conversion_preserves_the_error_code() {
        let result: Result<(), AppError> = Err(AppError::new(ErrorCode::Timeout));

        match CommandResult::from(result) {
            CommandResult::Err { error } => assert_eq!(error.code, ErrorCode::Timeout),
            CommandResult::Ok { .. } => panic!("expected an error envelope"),
        }
    }
}

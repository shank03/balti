use tokio::task::JoinError;

pub type AppResult<T> = Result<T, AppError>;

#[allow(warnings)]
#[derive(Debug, Clone)]
pub struct AppError {
    pub status: u16,
    pub message: String,
    pub req_id: String,
}

impl AppError {
    #[track_caller]
    pub fn message(message: impl Into<String>) -> Self {
        let message = message.into();

        let location = std::panic::Location::caller();
        tracing::error!(
            "Error [{}:{}:{}]: {}",
            location.file(),
            location.line(),
            location.column(),
            message,
        );

        Self {
            status: 400,
            message,
            req_id: "".into(),
        }
    }

    #[track_caller]
    pub fn err(err: impl std::error::Error) -> Self {
        let source = err.source();
        let location = std::panic::Location::caller();
        tracing::error!(
            "Error [{}:{}:{}]: {} - {:?}",
            location.file(),
            location.line(),
            location.column(),
            err,
            source,
        );

        Self {
            status: 500,
            message: format!("{}", err),
            req_id: "".into(),
        }
    }
}

impl From<JoinError> for AppError {
    fn from(value: JoinError) -> Self {
        AppError::err(value)
    }
}

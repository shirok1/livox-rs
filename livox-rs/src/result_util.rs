use crate::{AsyncCommandTask, LivoxError};
use crate::LivoxError::*;


pub(crate) trait ToLivoxError {
    fn of_reason(self, reason: &'static str) -> LivoxError;
}

impl ToLivoxError for std::io::Error {
    fn of_reason(self, reason: &'static str) -> LivoxError {
        IoError(reason, self)
    }
}

impl ToLivoxError for tokio::sync::mpsc::error::SendError<AsyncCommandTask> {
    fn of_reason(self, reason: &'static str) -> LivoxError {
        AsyncChannelError(reason, self)
    }
}

impl ToLivoxError for tokio::sync::oneshot::error::RecvError {
    fn of_reason(self, reason: &'static str) -> LivoxError {
        AsyncCallbackError(reason, self)
    }
}

pub(crate) trait ToLivoxResult<O> {
    fn err_reason(self, reason: &'static str) -> Result<O, LivoxError>;
}

impl<O, E: ToLivoxError> ToLivoxResult<O> for Result<O, E> {
    fn err_reason(self, reason: &'static str) -> Result<O, LivoxError> {
        self.map_err(|err| err.of_reason(reason))
    }
}

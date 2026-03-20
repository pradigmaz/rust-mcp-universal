use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(super) struct InvalidParamsError {
    message: String,
}

impl InvalidParamsError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for InvalidParamsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for InvalidParamsError {}

pub(super) fn invalid_params_error(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(InvalidParamsError::new(message))
}

pub(super) fn is_invalid_params_error(err: &anyhow::Error) -> bool {
    err.chain()
        .any(|cause| cause.downcast_ref::<InvalidParamsError>().is_some())
}

#[derive(Debug)]
pub(super) struct ToolDomainError {
    message: String,
}

impl ToolDomainError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ToolDomainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for ToolDomainError {}

pub(super) fn tool_domain_error(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(ToolDomainError::new(message))
}

pub(super) fn is_tool_domain_error(err: &anyhow::Error) -> bool {
    err.chain()
        .any(|cause| cause.downcast_ref::<ToolDomainError>().is_some())
}

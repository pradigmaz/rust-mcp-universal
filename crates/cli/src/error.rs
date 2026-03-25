use std::fmt;

pub(crate) const CODE_PARSE_ARGS: &str = "E_PARSE_ARGS";
pub(crate) const CODE_RUNTIME: &str = "E_RUNTIME";
pub(crate) const CODE_COMPATIBILITY: &str = "E_COMPATIBILITY";
pub(crate) const CODE_CONFIRM_REQUIRED: &str = "E_CONFIRM_REQUIRED";
pub(crate) const CODE_INDEX_NOT_READY: &str = "E_INDEX_NOT_READY";

#[derive(Debug)]
pub(crate) struct CliError {
    code: &'static str,
    message: String,
}

impl CliError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

pub(crate) fn cli_error(code: &'static str, message: impl Into<String>) -> anyhow::Error {
    CliError::new(code, message).into()
}

pub(crate) fn classify_error(
    err: &anyhow::Error,
    default_code: &'static str,
) -> (&'static str, String) {
    if let Some(cli_error) = err.downcast_ref::<CliError>() {
        return (cli_error.code(), cli_error.to_string());
    }
    let message = err.to_string();
    if message.contains("newer than binary supported")
        || message.contains("running binary version") && message.contains("is stale")
    {
        return (CODE_COMPATIBILITY, message);
    }
    (default_code, message)
}

use std::fmt::{Display, Formatter};
use std::io::{self, BufRead, Write};

use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WireMode {
    Framed,
    LineJson,
}

const MAX_CONTENT_LENGTH: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct ReadMessageError {
    message: String,
    recoverable: bool,
}

impl ReadMessageError {
    fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: false,
        }
    }

    fn recoverable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: true,
        }
    }

    fn message(&self) -> &str {
        &self.message
    }

    fn is_recoverable(&self) -> bool {
        self.recoverable
    }
}

impl Display for ReadMessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ReadMessageError {}

impl ReadMessageError {
    #[cfg(test)]
    pub(crate) fn kind(&self) -> io::ErrorKind {
        io::ErrorKind::InvalidData
    }
}

pub(crate) fn read_framed_message<R: BufRead>(
    reader: &mut R,
) -> std::result::Result<Option<(String, WireMode)>, ReadMessageError> {
    let mut content_length: Option<usize> = None;
    let mut saw_header_line = false;
    let mut invalid_framed_reason: Option<String> = None;

    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| ReadMessageError::fatal(err.to_string()))?;
        if read == 0 {
            if saw_header_line {
                if let Some(reason) = invalid_framed_reason {
                    return if content_length.is_some() {
                        Err(ReadMessageError::recoverable(reason))
                    } else {
                        Err(ReadMessageError::fatal(reason))
                    };
                }
                return Err(ReadMessageError::fatal("missing Content-Length header"));
            }
            return Ok(None);
        }

        let line = line.trim_end_matches(['\r', '\n']);
        let line = strip_utf8_bom(line);
        if line.is_empty() {
            if content_length.is_some() {
                break;
            }
            if saw_header_line {
                if let Some(reason) = invalid_framed_reason {
                    return Err(ReadMessageError::fatal(reason));
                }
                return Err(ReadMessageError::fatal("missing Content-Length header"));
            }
            continue;
        }

        if !saw_header_line {
            let Some((header_name, header_value)) = parse_header_line(line) else {
                return Ok(Some((line.to_string(), WireMode::LineJson)));
            };
            saw_header_line = true;
            apply_header(
                header_name,
                header_value,
                &mut content_length,
                &mut invalid_framed_reason,
            );
            continue;
        }

        if let Some((header_name, header_value)) = parse_header_line(line) {
            apply_header(
                header_name,
                header_value,
                &mut content_length,
                &mut invalid_framed_reason,
            );
            continue;
        }

        if invalid_framed_reason.is_none() {
            invalid_framed_reason = Some(format!("invalid framed header line `{line}`"));
        }
    }

    let Some(length) = content_length else {
        return Err(ReadMessageError::fatal("missing Content-Length header"));
    };

    if let Some(reason) = invalid_framed_reason {
        let mut body = vec![0_u8; length];
        reader
            .read_exact(&mut body)
            .map_err(|err| ReadMessageError::fatal(err.to_string()))?;
        return Err(ReadMessageError::recoverable(reason));
    }

    let mut body = vec![0_u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|err| ReadMessageError::fatal(err.to_string()))?;
    String::from_utf8(body)
        .map(|raw| Some((raw, WireMode::Framed)))
        .map_err(|err| ReadMessageError::fatal(format!("message body is not utf-8: {err}")))
}

fn strip_utf8_bom(line: &str) -> &str {
    line.trim_start_matches('\u{feff}')
}

fn parse_header_line(line: &str) -> Option<(&str, &str)> {
    let (header_name, header_value) = line.split_once(':')?;
    let header_name = header_name.trim();
    looks_like_header_name(header_name).then_some((header_name, header_value))
}

fn looks_like_header_name(header_name: &str) -> bool {
    let mut chars = header_name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn apply_header(
    header_name: &str,
    header_value: &str,
    content_length: &mut Option<usize>,
    invalid_framed_reason: &mut Option<String>,
) {
    if header_name.eq_ignore_ascii_case("Content-Length") {
        if content_length.is_some() && invalid_framed_reason.is_none() {
            *invalid_framed_reason = Some("duplicate Content-Length header".to_string());
            return;
        }
        match header_value.trim().parse::<usize>() {
            Ok(parsed) if parsed <= MAX_CONTENT_LENGTH => {
                *content_length = Some(parsed);
            }
            Ok(parsed) => {
                if invalid_framed_reason.is_none() {
                    *invalid_framed_reason = Some(format!(
                        "Content-Length {parsed} exceeds maximum supported size {MAX_CONTENT_LENGTH}"
                    ));
                }
            }
            Err(err) => {
                if invalid_framed_reason.is_none() {
                    *invalid_framed_reason = Some(format!("invalid Content-Length header: {err}"));
                }
            }
        }
    }
}

pub(crate) fn write_framed_message<W: Write>(writer: &mut W, payload: &str) -> io::Result<()> {
    write!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        payload.len(),
        payload
    )
}

pub(crate) fn run_stdio_server<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    state: &mut crate::ServerState,
) -> Result<()> {
    loop {
        let framed = match read_framed_message(reader) {
            Ok(value) => value,
            Err(err) => {
                let response = crate::parse_error_response(err.message().to_string());
                let payload = serde_json::to_string(&response)?;
                write_framed_message(writer, &payload)?;
                writer.flush()?;
                if err.is_recoverable() {
                    continue;
                }
                break;
            }
        };

        let Some((raw_message, wire_mode)) = framed else {
            break;
        };

        let response = crate::process_raw_message(&raw_message, state);
        let should_exit = state.should_exit();
        let Some(response) = response else {
            if should_exit {
                break;
            }
            continue;
        };

        let payload = serde_json::to_string(&response)?;
        match wire_mode {
            WireMode::Framed => write_framed_message(writer, &payload)?,
            WireMode::LineJson => {
                writeln!(writer, "{payload}")?;
            }
        }
        writer.flush()?;
        if should_exit {
            break;
        }
    }

    Ok(())
}

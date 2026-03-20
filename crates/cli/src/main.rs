mod args;
mod commands;
mod error;
mod output;
mod validation;

use std::any::Any;
use std::ffi::OsString;
use std::panic::{self, AssertUnwindSafe};
use std::process;

use args::App;
use clap::Parser;
use rmu_core::PrivacyMode;

fn main() {
    let json_requested = is_json_flag_requested(std::env::args_os());
    let exit_code = if json_requested {
        run_json_mode()
    } else {
        run_text_mode()
    };
    process::exit(exit_code);
}

fn run_text_mode() -> i32 {
    let app = match App::try_parse() {
        Ok(parsed) => parsed,
        Err(err) => err.exit(),
    };
    let json_output = app.json;
    let privacy_mode = PrivacyMode::parse(&app.privacy_mode).unwrap_or(PrivacyMode::Off);
    maybe_trigger_contract_test_panic();

    match commands::run(app) {
        Ok(()) => 0,
        Err(err) => {
            let (code, message) = error::classify_error(&err, error::CODE_RUNTIME);
            let _ = output::print_app_error(json_output, code, &message, privacy_mode);
            1
        }
    }
}

fn run_json_mode() -> i32 {
    let app = match catch_json_panic(App::try_parse) {
        Ok(Ok(parsed)) => parsed,
        Ok(Err(err)) => {
            let _ = output::print_app_error(
                true,
                error::CODE_PARSE_ARGS,
                &err.to_string(),
                PrivacyMode::Off,
            );
            return 2;
        }
        Err(message) => {
            let _ = output::print_app_error(true, error::CODE_RUNTIME, &message, PrivacyMode::Off);
            return 1;
        }
    };
    let privacy_mode = PrivacyMode::parse(&app.privacy_mode).unwrap_or(PrivacyMode::Off);

    match catch_json_panic(AssertUnwindSafe(|| {
        maybe_trigger_contract_test_panic();
        commands::run(app)
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(err)) => {
            let (code, message) = error::classify_error(&err, error::CODE_RUNTIME);
            let _ = output::print_app_error(true, code, &message, privacy_mode);
            1
        }
        Err(message) => {
            let _ = output::print_app_error(true, error::CODE_RUNTIME, &message, privacy_mode);
            1
        }
    }
}

fn catch_json_panic<T>(f: impl FnOnce() -> T) -> std::result::Result<T, String> {
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    panic::set_hook(previous_hook);
    result.map_err(|payload| panic_payload_message(payload.as_ref()))
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&'static str>()
        .map(|message| (*message).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "internal panic".to_string())
}

fn maybe_trigger_contract_test_panic() {
    if std::env::var_os("RMU_TEST_PANIC").is_some() {
        panic!("contract test panic");
    }
}

fn is_json_flag_requested<I>(args: I) -> bool
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    args.into_iter().skip(1).any(|arg| {
        let value = arg.into();
        let value = value.to_string_lossy();
        value == "--json" || value == "-j" || value.starts_with("--json=")
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{is_json_flag_requested, panic_payload_message};

    #[test]
    fn detects_json_flag_variants_for_parse_errors() {
        assert!(is_json_flag_requested(vec![
            OsString::from("rmu"),
            OsString::from("--json"),
            OsString::from("status")
        ]));
        assert!(is_json_flag_requested(vec![
            OsString::from("rmu"),
            OsString::from("--json=true"),
            OsString::from("status")
        ]));
        assert!(is_json_flag_requested(vec![
            OsString::from("rmu"),
            OsString::from("-j"),
            OsString::from("status")
        ]));
    }

    #[test]
    fn ignores_non_json_flags() {
        assert!(!is_json_flag_requested(vec![
            OsString::from("rmu"),
            OsString::from("--project-path"),
            OsString::from("."),
            OsString::from("status")
        ]));
    }

    #[test]
    fn panic_payload_message_prefers_string_payloads() {
        assert_eq!(
            panic_payload_message(&"sample panic"),
            "sample panic".to_string()
        );
        assert_eq!(
            panic_payload_message(&"owned panic".to_string()),
            "owned panic".to_string()
        );
    }
}

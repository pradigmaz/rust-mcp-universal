use anyhow::{Result, anyhow};
use rmu_core::{
    Engine, FindingFamily, PrivacyMode, SignalMemoryDecision, SignalMemoryMarkRequest,
    SignalMemoryOptions, sanitize_value_for_privacy,
};

use crate::output::{print_json, print_line};

pub(crate) struct SignalMemoryArgs {
    pub(crate) limit: usize,
    pub(crate) finding_family: Option<String>,
    pub(crate) decision: Option<String>,
}

pub(crate) struct MarkSignalMemoryArgs {
    pub(crate) signal_key: String,
    pub(crate) finding_family: String,
    pub(crate) decision: String,
    pub(crate) reason: String,
    pub(crate) source: String,
    pub(crate) scope: Option<String>,
}

pub(crate) fn inspect(
    engine: &Engine,
    json: bool,
    privacy_mode: PrivacyMode,
    args: SignalMemoryArgs,
) -> Result<()> {
    let result = engine.signal_memory(&SignalMemoryOptions {
        limit: args.limit,
        finding_family: parse_finding_family(args.finding_family.as_deref())?,
        decision: parse_decision(args.decision.as_deref())?,
    })?;
    if json {
        let mut value = serde_json::to_value(&result)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!("entries={}", result.entries.len()));
    }
    Ok(())
}

pub(crate) fn mark(
    engine: &Engine,
    json: bool,
    privacy_mode: PrivacyMode,
    args: MarkSignalMemoryArgs,
) -> Result<()> {
    let result = engine.mark_signal_memory(&SignalMemoryMarkRequest {
        signal_key: args.signal_key,
        finding_family: parse_finding_family(Some(&args.finding_family))?
            .ok_or_else(|| anyhow!("finding_family is required"))?,
        scope: args.scope,
        decision: parse_decision(Some(&args.decision))?
            .ok_or_else(|| anyhow!("decision is required"))?,
        reason: args.reason,
        source: args.source,
    })?;
    if json {
        let mut value = serde_json::to_value(&result)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!("stored={}", result.signal_key));
    }
    Ok(())
}

fn parse_finding_family(raw: Option<&str>) -> Result<Option<FindingFamily>> {
    raw.map(|value| {
        FindingFamily::parse(value).ok_or_else(|| anyhow!("unsupported finding_family `{value}`"))
    })
    .transpose()
}

fn parse_decision(raw: Option<&str>) -> Result<Option<SignalMemoryDecision>> {
    raw.map(|value| {
        SignalMemoryDecision::parse(value).ok_or_else(|| anyhow!("unsupported decision `{value}`"))
    })
    .transpose()
}

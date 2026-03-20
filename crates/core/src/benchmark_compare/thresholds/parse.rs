use serde_json::Value;

pub(super) struct ParsedThresholdSections<'a> {
    pub(super) min: Option<&'a Value>,
    pub(super) max: Option<&'a Value>,
}

pub(super) fn parse_sections(value: &Value) -> ParsedThresholdSections<'_> {
    ParsedThresholdSections {
        min: value.get("min"),
        max: value.get("max"),
    }
}

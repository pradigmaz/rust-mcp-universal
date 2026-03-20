use serde::{Deserialize, Deserializer, Serializer};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use super::{defaults, parse};

pub(super) fn default_chunk_source() -> String {
    defaults::default_chunk_source()
}

pub(super) fn serialize_optional_offset_datetime_rfc3339<S>(
    value: &Option<OffsetDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(
            &value
                .to_offset(UtcOffset::UTC)
                .format(&Rfc3339)
                .map_err(serde::ser::Error::custom)?,
        ),
        None => serializer.serialize_none(),
    }
}

pub(super) fn deserialize_optional_offset_datetime_rfc3339<'de, D>(
    deserializer: D,
) -> Result<Option<OffsetDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    raw.map(|raw| {
        parse::changed_since(&raw).ok_or_else(|| {
            serde::de::Error::custom(
                OffsetDateTime::parse(raw.trim(), &Rfc3339)
                    .err()
                    .map(|err| err.to_string())
                    .unwrap_or_else(|| "invalid RFC3339 timestamp".to_string()),
            )
        })
    })
    .transpose()
}

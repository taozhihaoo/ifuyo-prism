//! 统一时间戳。
//!
//! 一律 UTC、RFC3339 序列化（不使用本地时区），保证同一事实在 GUI / CLI / JSON
//! 报告中的字符串表示确定一致。精度截断到毫秒。

use crate::error::{PrismError, PrismResult};
use std::fmt;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// UTC 时间戳，JSON 序列化为 RFC3339 字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(OffsetDateTime);

impl Timestamp {
    /// 当前时刻（截断到毫秒）。
    pub fn now_utc() -> Self {
        let dt = OffsetDateTime::now_utc();
        let millis = dt.millisecond();
        let dt = dt
            .replace_nanosecond(u32::from(millis) * 1_000_000)
            .unwrap_or(dt);
        Timestamp(dt)
    }

    pub fn from_rfc3339(value: &str) -> PrismResult<Self> {
        let dt =
            OffsetDateTime::parse(value, &Rfc3339).map_err(|err| PrismError::Serialization {
                message: format!("invalid RFC3339 timestamp {value:?}: {err}"),
            })?;
        Ok(Timestamp(dt))
    }

    pub fn from_unix_millis(millis: i128) -> PrismResult<Self> {
        OffsetDateTime::from_unix_timestamp_nanos(millis * 1_000_000).map_or_else(
            |_| {
                Err(PrismError::Serialization {
                    message: format!("unix millis out of range: {millis}"),
                })
            },
            |dt| Ok(Timestamp(dt)),
        )
    }

    pub fn as_rfc3339(&self) -> String {
        self.0
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }

    pub fn to_unix_millis(&self) -> i128 {
        self.0.unix_timestamp_nanos() / 1_000_000
    }

    pub fn datetime(&self) -> OffsetDateTime {
        self.0
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Timestamp(OffsetDateTime::UNIX_EPOCH)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_rfc3339())
    }
}

impl serde::Serialize for Timestamp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_rfc3339())
    }
}

impl<'de> serde::Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Timestamp::from_rfc3339(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_serializes_as_rfc3339_utc() {
        let ts = Timestamp::now_utc();
        let text = ts.as_rfc3339();
        assert!(text.ends_with('Z'), "expected UTC suffix, got {text}");
        let back = Timestamp::from_rfc3339(&text).unwrap();
        assert_eq!(ts, back);
    }

    #[test]
    fn unix_millis_roundtrip() {
        let ts = Timestamp::now_utc();
        let millis = ts.to_unix_millis();
        let back = Timestamp::from_unix_millis(millis).unwrap();
        assert_eq!(ts, back);
    }

    #[test]
    fn invalid_string_is_rejected_as_serialization_error() {
        let err = Timestamp::from_rfc3339("not-a-date").unwrap_err();
        assert_eq!(err.code(), crate::error::PrismErrorCode::Serialization);
    }

    #[test]
    fn json_roundtrip_is_plain_string() {
        let ts = Timestamp::now_utc();
        let json = serde_json::to_string(&ts).unwrap();
        assert!(json.starts_with('"') && json.ends_with('"'));
        let back: Timestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(ts, back);
    }

    #[test]
    fn default_is_unix_epoch() {
        assert_eq!(Timestamp::default().as_rfc3339(), "1970-01-01T00:00:00Z");
    }
}

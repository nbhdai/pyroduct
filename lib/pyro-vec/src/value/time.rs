use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::TryFrom;
use std::fmt;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    // Note: We removed serde::Serialize and serde::Deserialize from here
    // so we can implement them manually below.
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Time(pub i128);

// -----------------------------------------------------------------------------
// Equality (Existing Logic)
// -----------------------------------------------------------------------------
impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        let self_secs = self.0.div_euclid(1_000_000_000);
        let other_secs = other.0.div_euclid(1_000_000_000);
        self_secs == other_secs
    }
}
impl Eq for Time {}

// -----------------------------------------------------------------------------
// 1. DateTime<Utc> -> Time (Safe, Lossless)
// -----------------------------------------------------------------------------
impl From<DateTime<Utc>> for Time {
    fn from(dt: DateTime<Utc>) -> Self {
        let secs = dt.timestamp() as i128;
        let nsecs = dt.timestamp_subsec_nanos() as i128;
        Time(secs * 1_000_000_000 + nsecs)
    }
}

// -----------------------------------------------------------------------------
// 2. Time -> DateTime<Utc> (Fallible)
// -----------------------------------------------------------------------------
impl TryFrom<Time> for DateTime<Utc> {
    type Error = &'static str;

    fn try_from(time: Time) -> Result<Self, Self::Error> {
        let nanos = time.0;
        let secs = (nanos.div_euclid(1_000_000_000)) as i64;
        let nsecs = (nanos.rem_euclid(1_000_000_000)) as u32;

        if nanos >= 0 && secs < 0 {
            return Err("Timestamp overflowed positive i64 seconds");
        }
        if nanos < 0 && secs > 0 {
            return Err("Timestamp overflowed negative i64 seconds");
        }

        Utc.timestamp_opt(secs, nsecs)
            .single()
            .ok_or("Invalid timestamp or out of range for Chrono")
    }
}

// -----------------------------------------------------------------------------
// Struct Methods
// -----------------------------------------------------------------------------
impl Time {
    pub fn to_datetime(&self) -> DateTime<Utc> {
        DateTime::try_from(*self).expect("Time value out of bounds for DateTime")
    }

    pub fn to_iso8601(&self) -> String {
        self.to_datetime().to_rfc3339()
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_iso8601())
    }
}

// -----------------------------------------------------------------------------
// Serde Serialization (Time -> ISO String)
// -----------------------------------------------------------------------------
impl Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as the ISO 8601 string
        serializer.serialize_str(&self.to_iso8601())
    }
}

// -----------------------------------------------------------------------------
// Serde Deserialization (ISO String -> Time)
// -----------------------------------------------------------------------------
impl<'de> Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TimeVisitor;

        impl<'de> Visitor<'de> for TimeVisitor {
            type Value = Time;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an ISO 8601 string (full or partial)")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // 1. Try Standard RFC3339 (e.g., "2023-10-25T12:00:00Z")
                if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                    return Ok(Time::from(dt.with_timezone(&Utc)));
                }

                // 2. Try Naive formats (assume UTC if no timezone is present).
                // We attempt most specific (subseconds) to least specific (date only).
                let formats = [
                    "%Y-%m-%dT%H:%M:%S%.f", // Date + Time + fractional seconds
                    "%Y-%m-%d %H:%M:%S%.f", // Date + Time + fractional (space separator)
                    "%Y-%m-%dT%H:%M:%S",    // Date + Time
                    "%Y-%m-%d %H:%M:%S",    // Date + Time (space)
                    "%Y-%m-%dT%H:%M",       // Date + Hour + Minute (Cut off seconds)
                    "%Y-%m-%d %H:%M",       // Date + Hour + Minute (space)
                ];

                for fmt in &formats {
                    if let Ok(naive) = NaiveDateTime::parse_from_str(value, fmt) {
                        return Ok(Time::from(Utc.from_utc_datetime(&naive)));
                    }
                }

                // 3. Try Date + Hour (e.g., "2023-10-25T14")
                // NaiveDateTime requires at least minutes to parse successfully.
                // We append ":00" to try parsing as "Date + Hour + Minute".
                let value_with_min = format!("{}:00", value);
                if let Ok(naive) = NaiveDateTime::parse_from_str(&value_with_min, "%Y-%m-%dT%H:%M")
                {
                    return Ok(Time::from(Utc.from_utc_datetime(&naive)));
                }
                if let Ok(naive) = NaiveDateTime::parse_from_str(&value_with_min, "%Y-%m-%d %H:%M")
                {
                    return Ok(Time::from(Utc.from_utc_datetime(&naive)));
                }

                // 4. Try Date Only (e.g., "2023-10-25") -> Becomes midnight UTC
                if let Ok(naive_date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                    let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
                    return Ok(Time::from(Utc.from_utc_datetime(&naive_dt)));
                }

                Err(E::custom(format!("Could not parse time string: {}", value)))
            }
        }

        deserializer.deserialize_str(TimeVisitor)
    }
}

// -----------------------------------------------------------------------------
// Usage Example & Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_serde_full_iso() {
        // Create a specific time
        let dt = Utc.timestamp_opt(1609459200, 0).unwrap(); // 2021-01-01 00:00:00 UTC
        let t = Time::from(dt);

        // Test Serialization
        let serialized = serde_json::to_string(&t).unwrap();
        assert_eq!(serialized, "\"2021-01-01T00:00:00+00:00\"");

        // Test Deserialization
        let deserialized: Time = serde_json::from_str(&serialized).unwrap();

        // Note: Using direct i128 comparison because your PartialEq impl ignores seconds
        assert_eq!(t.0, deserialized.0);
    }

    #[test]
    fn test_serde_partial_date_only() {
        let json_str = "\"2023-11-25\"";
        let t: Time = serde_json::from_str(json_str).unwrap();

        let dt = t.to_datetime();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 11);
        assert_eq!(dt.day(), 25);
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_serde_partial_cut_off_seconds() {
        // Missing seconds: 14:30
        let json_str = "\"2023-11-25T14:30\"";
        let t: Time = serde_json::from_str(json_str).unwrap();

        let dt = t.to_datetime();
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_serde_partial_cut_off_minutes() {
        // Missing minutes: 14:00 implied
        let json_str = "\"2023-11-25T14\"";
        let t: Time = serde_json::from_str(json_str).unwrap();

        let dt = t.to_datetime();
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_serde_space_separator() {
        let json_str = "\"2023-11-25 14:30:00\"";
        let t: Time = serde_json::from_str(json_str).unwrap();
        let dt = t.to_datetime();
        assert_eq!(dt.hour(), 14);
    }

    #[test]
    fn test_serde_invalid_format() {
        let json_str = "\"not-a-date\"";
        let res: Result<Time, _> = serde_json::from_str(json_str);
        assert!(res.is_err());
    }

    #[test]
    fn test_struct_equality_logic() {
        // Ensure your original custom PartialEq logic still holds
        let t1 = Time(10_000_000_100);
        let t2 = Time(10_000_000_900);
        assert_eq!(
            t1, t2,
            "Seconds match, so they are Equal per implementation"
        );
    }
}

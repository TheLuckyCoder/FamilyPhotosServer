// This file is based on https://time-rs.github.io/internal-api/src/time/serde/timestamp.rs.html

//! Treat an [`OffsetDateTime`] as a [Unix timestamp] for the purposes of serde.
//!
//! Use this module in combination with serde's [`#[with]`][with] attribute.
//!
//! When deserializing, the offset is assumed to be UTC.
//!
//! [Unix timestamp]: https://en.wikipedia.org/wiki/Unix_time
//! [with]: https://serde.rs/field-attrs.html#with

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use time::{OffsetDateTime, PrimitiveDateTime};

/// Serialize an `PrimitiveDateTime` as its Unix timestamp
pub fn serialize<S: Serializer>(
    datetime: &PrimitiveDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    datetime.assume_utc().unix_timestamp().serialize(serializer)
}

/// Deserialize an `PrimitiveDateTime` from its Unix timestamp
pub fn deserialize<'a, D: Deserializer<'a>>(
    deserializer: D,
) -> Result<PrimitiveDateTime, D::Error> {
    OffsetDateTime::from_unix_timestamp(<_>::deserialize(deserializer)?)
        .map(|date_time| PrimitiveDateTime::new(date_time.date(), date_time.time()))
        .map_err(|err| de::Error::custom(err.to_string()))
}

/// Treat an `Option<PrimitiveDateTime>` as a [Unix timestamp] for the purposes of
/// serde.
///
/// Use this module in combination with serde's [`#[with]`][with] attribute.
///
/// When deserializing, the offset is assumed to be UTC.
///
/// [Unix timestamp]: https://en.wikipedia.org/wiki/Unix_time
/// [with]: https://serde.rs/field-attrs.html#with
#[allow(dead_code)]
pub mod option {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use time::PrimitiveDateTime;

    /// Serialize an `Option<PrimitiveDateTime>` as its Unix timestamp
    pub fn serialize<S: Serializer>(
        option: &Option<PrimitiveDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        option
            .map(|time| time.assume_utc().unix_timestamp())
            .serialize(serializer)
    }

    /// Deserialize an `Option<PrimitiveDateTime>` from its Unix timestamp
    pub fn deserialize<'a, D: Deserializer<'a>>(
        deserializer: D,
    ) -> Result<Option<PrimitiveDateTime>, D::Error> {
        Option::deserialize(deserializer)?
            .map(|timestamp| {
                let date_time_result = OffsetDateTime::from_unix_timestamp(timestamp);
                match date_time_result {
                    Ok(date_time) => Ok(PrimitiveDateTime::new(date_time.date(), date_time.time())),
                    Err(e) => Err(e),
                }
            })
            .transpose()
            .map_err(|err| de::Error::custom(err.to_string()))
    }
}

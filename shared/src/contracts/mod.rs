//! Wire contracts between the services: one Avro schema and one model per Kafka message.
//!
//! This is the single source of truth for the messages. Producer and consumer use the same
//! struct, so a schema change shows up as a compile error on both sides instead of messages
//! that silently fail to decode.

pub mod catalog;
pub mod customer;
pub mod payment;
pub mod restaurant;
pub mod topics;

use apache_avro::Schema;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A message model with its Avro schema.
///
/// Messages are Avro object containers (the schema is embedded in every message).
pub trait AvroMessage: Serialize + DeserializeOwned {
    const SCHEMA: &'static str;

    fn to_avro_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let schema = Schema::parse_str(Self::SCHEMA)?;
        let value = apache_avro::to_value(self)?.resolve(&schema)?;
        let mut writer = apache_avro::Writer::new(&schema, Vec::new())?;

        writer.append_value(value)?;
        writer.flush()?;

        Ok(writer.into_inner()?)
    }

    /// Decodes every record of an object container.
    fn from_avro_bytes(payload: &[u8]) -> anyhow::Result<Vec<Self>> {
        apache_avro::Reader::new(payload)?
            .map(|value| Ok(apache_avro::from_value::<Self>(&value?)?))
            .collect()
    }
}

macro_rules! avro_message {
    ($model:ty, $file:literal) => {
        impl $crate::contracts::AvroMessage for $model {
            const SCHEMA: &'static str =
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/avro/schemas/", $file));
        }
    };
}
pub(crate) use avro_message;

#[cfg(test)]
mod tests;

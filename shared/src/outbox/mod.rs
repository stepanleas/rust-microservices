//! Transactional outbox: messages are stored in the same database transaction as the state
//! change that produced them, and an [`relay::OutboxRelay`] publishes them afterwards.
//!
//! Every service has its own set of message kinds (an enum implementing [`MessageKind`]); the
//! model, the ports, the relay and the Postgres repository are shared.

pub mod ports;
pub mod relay;

#[cfg(feature = "outbox-postgres")]
pub mod postgres;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use uuid::Uuid;

/// The kinds of messages a service writes to its outbox. Stored as text in `message_type`.
pub trait MessageKind: Copy + Debug + PartialEq + Send + Sync + 'static {
    fn as_str(&self) -> &'static str;
    fn parse(value: &str) -> anyhow::Result<Self>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxStatus {
    Started,
    Completed,
    Failed,
}

impl OutboxStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutboxStatus::Started => "STARTED",
            OutboxStatus::Completed => "COMPLETED",
            OutboxStatus::Failed => "FAILED",
        }
    }
}

impl TryFrom<&str> for OutboxStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "STARTED" => Ok(OutboxStatus::Started),
            "COMPLETED" => Ok(OutboxStatus::Completed),
            "FAILED" => Ok(OutboxStatus::Failed),
            other => Err(anyhow::anyhow!("Unknown outbox status: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutboxMessage<K: MessageKind> {
    id: Uuid,
    saga_id: Uuid,
    message_type: K,
    payload: serde_json::Value,
    status: OutboxStatus,
    attempts: i32,
    created_at: DateTime<Utc>,
}

impl<K: MessageKind> OutboxMessage<K> {
    pub fn new<T: Serialize>(saga_id: Uuid, message_type: K, payload: &T) -> anyhow::Result<Self> {
        Ok(Self {
            id: Uuid::new_v4(),
            saga_id,
            message_type,
            payload: serde_json::to_value(payload)?,
            status: OutboxStatus::Started,
            attempts: 0,
            created_at: Utc::now(),
        })
    }

    pub fn restore(
        id: Uuid,
        saga_id: Uuid,
        message_type: K,
        payload: serde_json::Value,
        status: OutboxStatus,
        attempts: i32,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            saga_id,
            message_type,
            payload,
            status,
            attempts,
            created_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn saga_id(&self) -> Uuid {
        self.saga_id
    }

    pub fn message_type(&self) -> K {
        self.message_type
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    pub fn payload_as<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        Ok(serde_json::from_value(self.payload.clone())?)
    }

    pub fn status(&self) -> OutboxStatus {
        self.status
    }

    pub fn attempts(&self) -> i32 {
        self.attempts
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

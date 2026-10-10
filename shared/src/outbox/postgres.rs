//! Postgres implementation of [`OutboxRepository`]. Every service has the same
//! `outbox_messages` table (see the services' `create_outbox_messages_table` migration).

use super::ports::OutboxRepository;
use super::{MessageKind, OutboxMessage, OutboxStatus};
use crate::db::DbPool;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use std::marker::PhantomData;
use uuid::Uuid;

diesel::table! {
    outbox_messages (id) {
        id -> Uuid,
        saga_id -> Uuid,
        #[max_length = 64]
        message_type -> Varchar,
        payload -> Jsonb,
        #[max_length = 16]
        outbox_status -> Varchar,
        attempts -> Int4,
        created_at -> Timestamp,
        processed_at -> Nullable<Timestamp>,
    }
}

#[derive(Queryable, Selectable, Insertable, Debug)]
#[diesel(table_name = outbox_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct OutboxMessageRow {
    id: Uuid,
    saga_id: Uuid,
    message_type: String,
    payload: serde_json::Value,
    outbox_status: String,
    attempts: i32,
    created_at: NaiveDateTime,
    processed_at: Option<NaiveDateTime>,
}

impl<K: MessageKind> From<&OutboxMessage<K>> for OutboxMessageRow {
    fn from(message: &OutboxMessage<K>) -> Self {
        Self {
            id: message.id(),
            saga_id: message.saga_id(),
            message_type: message.message_type().as_str().to_string(),
            payload: message.payload().clone(),
            outbox_status: message.status().as_str().to_string(),
            attempts: message.attempts(),
            created_at: message.created_at().naive_utc(),
            processed_at: None,
        }
    }
}

impl OutboxMessageRow {
    fn into_message<K: MessageKind>(self) -> anyhow::Result<OutboxMessage<K>> {
        Ok(OutboxMessage::restore(
            self.id,
            self.saga_id,
            K::parse(&self.message_type)?,
            self.payload,
            OutboxStatus::try_from(self.outbox_status.as_str())?,
            self.attempts,
            self.created_at.and_utc(),
        ))
    }
}

/// Inserts an outbox message with the caller's connection, so it joins the caller's
/// transaction. This is what makes the outbox transactional.
pub fn insert_outbox_message<K: MessageKind>(
    connection: &mut PgConnection,
    message: &OutboxMessage<K>,
) -> QueryResult<()> {
    diesel::insert_into(outbox_messages::table)
        .values(&OutboxMessageRow::from(message))
        .execute(connection)?;

    Ok(())
}

pub struct PostgresOutboxRepository<K: MessageKind> {
    pool: DbPool,
    kind: PhantomData<K>,
}

impl<K: MessageKind> PostgresOutboxRepository<K> {
    pub fn new(pool: &DbPool) -> Self {
        Self {
            pool: pool.clone(),
            kind: PhantomData,
        }
    }
}

impl<K: MessageKind> OutboxRepository<K> for PostgresOutboxRepository<K> {
    fn find_by_status(
        &self,
        status: OutboxStatus,
        limit: i64,
    ) -> anyhow::Result<Vec<OutboxMessage<K>>> {
        let mut connection = self.pool.get()?;

        outbox_messages::table
            .filter(outbox_messages::outbox_status.eq(status.as_str()))
            .order(outbox_messages::created_at.asc())
            .limit(limit)
            .select(OutboxMessageRow::as_select())
            .load(&mut connection)?
            .into_iter()
            .map(OutboxMessageRow::into_message)
            .collect()
    }

    fn mark_completed(&self, id: Uuid) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        diesel::update(outbox_messages::table.find(id))
            .set((
                outbox_messages::outbox_status.eq(OutboxStatus::Completed.as_str()),
                outbox_messages::processed_at.eq(Some(chrono::Utc::now().naive_utc())),
            ))
            .execute(&mut connection)?;

        Ok(())
    }

    fn mark_attempt_failed(&self, id: Uuid, max_attempts: i32) -> anyhow::Result<OutboxStatus> {
        let mut connection = self.pool.get()?;

        connection.transaction::<_, anyhow::Error, _>(|conn| {
            let attempts = diesel::update(outbox_messages::table.find(id))
                .set(outbox_messages::attempts.eq(outbox_messages::attempts + 1))
                .returning(outbox_messages::attempts)
                .get_result::<i32>(conn)?;

            if attempts < max_attempts {
                return Ok(OutboxStatus::Started);
            }

            diesel::update(outbox_messages::table.find(id))
                .set((
                    outbox_messages::outbox_status.eq(OutboxStatus::Failed.as_str()),
                    outbox_messages::processed_at.eq(Some(chrono::Utc::now().naive_utc())),
                ))
                .execute(conn)?;

            Ok(OutboxStatus::Failed)
        })
    }
}

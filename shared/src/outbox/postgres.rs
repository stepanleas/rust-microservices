//! Postgres implementation of [`OutboxRepository`]. Every service has the same
//! `outbox_messages` table (see the services' `create_outbox_messages_table` migration).

use super::ports::{ClaimedBatch, OutboxBatchTransaction, OutboxRepository};
use super::{MessageKind, OutboxBacklog, OutboxMessage, OutboxStatus};
use crate::db::DbPool;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::connection::{AnsiTransactionManager, TransactionManager};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
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
        #[max_length = 128]
        trace_context -> Nullable<Varchar>,
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
    trace_context: Option<String>,
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
            trace_context: message.trace_context().map(str::to_string),
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
        )
        .with_trace_context(self.trace_context))
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

fn set_completed(connection: &mut PgConnection, id: Uuid) -> QueryResult<()> {
    diesel::update(outbox_messages::table.find(id))
        .set((
            outbox_messages::outbox_status.eq(OutboxStatus::Completed.as_str()),
            outbox_messages::processed_at.eq(Some(chrono::Utc::now().naive_utc())),
        ))
        .execute(connection)?;

    Ok(())
}

/// Not atomic on its own: call it inside a transaction.
fn count_failed_attempt(
    connection: &mut PgConnection,
    id: Uuid,
    max_attempts: i32,
) -> QueryResult<OutboxStatus> {
    let attempts = diesel::update(outbox_messages::table.find(id))
        .set(outbox_messages::attempts.eq(outbox_messages::attempts + 1))
        .returning(outbox_messages::attempts)
        .get_result::<i32>(connection)?;

    if attempts < max_attempts {
        return Ok(OutboxStatus::Started);
    }

    diesel::update(outbox_messages::table.find(id))
        .set((
            outbox_messages::outbox_status.eq(OutboxStatus::Failed.as_str()),
            outbox_messages::processed_at.eq(Some(chrono::Utc::now().naive_utc())),
        ))
        .execute(connection)?;

    Ok(OutboxStatus::Failed)
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

    fn claim_batch(&self, limit: i64) -> anyhow::Result<ClaimedBatch<K>> {
        let mut transaction = PostgresBatchTransaction::begin(self.pool.get()?)?;

        let rows = outbox_messages::table
            .filter(outbox_messages::outbox_status.eq(OutboxStatus::Started.as_str()))
            .order(outbox_messages::created_at.asc())
            .limit(limit)
            .for_update()
            .skip_locked()
            .select(OutboxMessageRow::as_select())
            .load(&mut *transaction.connection)?;
        let messages = rows
            .into_iter()
            .map(OutboxMessageRow::into_message)
            .collect::<anyhow::Result<_>>()?;

        Ok(ClaimedBatch {
            messages,
            transaction: Box::new(transaction),
        })
    }

    fn mark_completed(&self, id: Uuid) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;
        Ok(set_completed(&mut connection, id)?)
    }

    fn mark_attempt_failed(&self, id: Uuid, max_attempts: i32) -> anyhow::Result<OutboxStatus> {
        let mut connection = self.pool.get()?;
        Ok(connection.transaction(|conn| count_failed_attempt(conn, id, max_attempts))?)
    }

    fn delete_completed(&self, before: DateTime<Utc>, limit: i64) -> anyhow::Result<usize> {
        let mut connection = self.pool.get()?;

        // Bounded batches keep each delete (and its locks) short on a large table.
        let ids: Vec<Uuid> = outbox_messages::table
            .filter(outbox_messages::outbox_status.eq(OutboxStatus::Completed.as_str()))
            .filter(outbox_messages::processed_at.lt(before.naive_utc()))
            .select(outbox_messages::id)
            .limit(limit)
            .load(&mut connection)?;

        Ok(
            diesel::delete(outbox_messages::table.filter(outbox_messages::id.eq_any(ids)))
                .execute(&mut connection)?,
        )
    }

    fn backlog(&self) -> anyhow::Result<OutboxBacklog> {
        let mut connection = self.pool.get()?;

        // Both use the (outbox_status, created_at) index; COMPLETED rows (the bulk) are skipped.
        let count = |connection: &mut PgConnection, status: OutboxStatus| {
            outbox_messages::table
                .filter(outbox_messages::outbox_status.eq(status.as_str()))
                .count()
                .get_result::<i64>(connection)
        };
        let started = count(&mut connection, OutboxStatus::Started)?;
        let failed = count(&mut connection, OutboxStatus::Failed)?;
        let oldest_started = outbox_messages::table
            .filter(outbox_messages::outbox_status.eq(OutboxStatus::Started.as_str()))
            .select(diesel::dsl::min(outbox_messages::created_at))
            .first::<Option<NaiveDateTime>>(&mut connection)?;

        Ok(OutboxBacklog {
            started,
            failed,
            oldest_started: oldest_started.map(|at| at.and_utc()),
        })
    }
}

type PooledPgConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// A transaction on a pooled connection that outlives a closure (diesel's `transaction()`
/// can't span the relay's publish calls). Rolls back on drop unless committed.
struct PostgresBatchTransaction {
    connection: PooledPgConnection,
    open: bool,
}

impl PostgresBatchTransaction {
    fn begin(mut connection: PooledPgConnection) -> QueryResult<Self> {
        AnsiTransactionManager::begin_transaction(&mut *connection)?;
        Ok(Self {
            connection,
            open: true,
        })
    }
}

impl OutboxBatchTransaction for PostgresBatchTransaction {
    fn mark_completed(&mut self, id: Uuid) -> anyhow::Result<()> {
        Ok(set_completed(&mut self.connection, id)?)
    }

    fn mark_attempt_failed(&mut self, id: Uuid, max_attempts: i32) -> anyhow::Result<OutboxStatus> {
        Ok(count_failed_attempt(
            &mut self.connection,
            id,
            max_attempts,
        )?)
    }

    fn commit(mut self: Box<Self>) -> anyhow::Result<()> {
        self.open = false;
        AnsiTransactionManager::commit_transaction(&mut *self.connection)?;
        Ok(())
    }
}

impl Drop for PostgresBatchTransaction {
    fn drop(&mut self) {
        if self.open
            && let Err(error) = AnsiTransactionManager::rollback_transaction(&mut *self.connection)
        {
            tracing::error!("Failed to roll back the outbox batch: {error}");
        }
    }
}

//! The Postgres outbox and relay against a real database (testcontainers, needs Docker).

use chrono::Utc;
use diesel::RunQueryDsl;
use shared::db::DbPool;
use shared::outbox::ports::{OutboxMessagePublisher, OutboxRepository};
use shared::outbox::postgres::{PostgresOutboxRepository, insert_outbox_message};
use shared::outbox::relay::{OutboxRelay, RelaySettings};
use shared::outbox::{MessageKind, OutboxMessage, OutboxStatus};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// Same table as the services' `create_outbox_messages_table` migration.
const CREATE_TABLE: &str = "
CREATE TABLE outbox_messages (
    id uuid PRIMARY KEY,
    saga_id uuid NOT NULL,
    message_type VARCHAR(64) NOT NULL,
    payload JSONB NOT NULL,
    outbox_status VARCHAR(16) NOT NULL CHECK (outbox_status IN ('STARTED', 'COMPLETED', 'FAILED')),
    attempts INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMP,
    trace_context VARCHAR(128)
);
CREATE INDEX idx_outbox_messages_status_created_at ON outbox_messages (outbox_status, created_at);
";

#[derive(Debug, Clone, Copy, PartialEq)]
struct Kind;

impl MessageKind for Kind {
    fn as_str(&self) -> &'static str {
        "KIND"
    }

    fn parse(_: &str) -> anyhow::Result<Self> {
        Ok(Kind)
    }
}

async fn start_postgres() -> anyhow::Result<(ContainerAsync<Postgres>, DbPool)> {
    let container = Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let pool = tokio::task::spawn_blocking(move || -> anyhow::Result<DbPool> {
        let manager = diesel::r2d2::ConnectionManager::new(url.as_str());
        let pool = diesel::r2d2::Pool::builder().max_size(8).build(manager)?;
        diesel::connection::SimpleConnection::batch_execute(&mut *pool.get()?, CREATE_TABLE)?;
        Ok(pool)
    })
    .await??;

    Ok((container, pool))
}

fn insert_messages(pool: &DbPool, count: usize) -> anyhow::Result<Vec<Uuid>> {
    let mut connection = pool.get()?;
    (0..count)
        .map(|i| {
            let message = OutboxMessage::new(Uuid::new_v4(), Kind, &i)?;
            insert_outbox_message(&mut connection, &message)?;
            Ok(message.id())
        })
        .collect()
}

/// Records every publish; slow enough that two relays overlap.
#[derive(Default)]
struct RecordingPublisher {
    published: Mutex<HashMap<Uuid, usize>>,
}

impl OutboxMessagePublisher<Kind> for RecordingPublisher {
    fn publish(&self, message: &OutboxMessage<Kind>) -> anyhow::Result<()> {
        std::thread::sleep(Duration::from_millis(2));
        *self
            .published
            .lock()
            .unwrap()
            .entry(message.id())
            .or_default() += 1;
        Ok(())
    }
}

fn settings() -> RelaySettings {
    RelaySettings {
        poll_interval: Duration::from_millis(10),
        batch_size: 20,
        max_attempts: 3,
        retention: Duration::from_secs(3600),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_two_relays_never_publish_a_message_twice() -> anyhow::Result<()> {
    let (_postgres, pool) = start_postgres().await?;

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let ids = insert_messages(&pool, 200)?;
        let repository: Arc<PostgresOutboxRepository<Kind>> =
            Arc::new(PostgresOutboxRepository::new(&pool));
        let publisher = Arc::new(RecordingPublisher::default());

        let relays: Vec<_> = (0..2)
            .map(|_| {
                let relay = OutboxRelay::new(repository.clone(), publisher.clone(), settings());
                std::thread::spawn(move || -> anyhow::Result<usize> {
                    let mut total = 0;
                    loop {
                        let published = relay.process_batch()?;
                        if published == 0 {
                            return Ok(total);
                        }
                        total += published;
                    }
                })
            })
            .collect();
        let per_relay: Vec<usize> = relays
            .into_iter()
            .map(|relay| relay.join().expect("relay panicked"))
            .collect::<anyhow::Result<_>>()?;

        let published = publisher.published.lock().unwrap();
        assert_eq!(200, published.len());
        assert!(published.values().all(|count| *count == 1), "duplicates");
        assert!(ids.iter().all(|id| published.contains_key(id)));
        assert!(per_relay.iter().all(|count| *count > 0), "{per_relay:?}");
        assert_eq!(
            200,
            repository
                .find_by_status(OutboxStatus::Completed, 1000)?
                .len()
        );
        Ok(())
    })
    .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn test_a_dropped_claim_releases_its_messages() -> anyhow::Result<()> {
    let (_postgres, pool) = start_postgres().await?;

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        insert_messages(&pool, 3)?;
        let repository = PostgresOutboxRepository::<Kind>::new(&pool);

        let first = repository.claim_batch(10)?;
        assert_eq!(3, first.messages.len());
        // Locked by the first claim: a second relay skips them.
        assert!(repository.claim_batch(10)?.messages.is_empty());

        drop(first);
        let again = repository.claim_batch(10)?;
        assert_eq!(3, again.messages.len());
        Ok(())
    })
    .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cleanup_deletes_only_old_completed_messages() -> anyhow::Result<()> {
    let (_postgres, pool) = start_postgres().await?;

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let ids = insert_messages(&pool, 3)?;
        let repository = PostgresOutboxRepository::<Kind>::new(&pool);
        repository.mark_completed(ids[0])?;
        repository.mark_completed(ids[1])?;
        // ids[0] was processed two days ago, ids[1] just now, ids[2] is still pending.
        diesel::sql_query(format!(
            "UPDATE outbox_messages SET processed_at = now() - interval '2 days' WHERE id = '{}'",
            ids[0]
        ))
        .execute(&mut pool.get()?)?;

        let deleted = repository.delete_completed(Utc::now() - chrono::Duration::hours(24), 100)?;

        assert_eq!(1, deleted);
        let remaining: Vec<Uuid> = [OutboxStatus::Completed, OutboxStatus::Started]
            .into_iter()
            .map(|status| repository.find_by_status(status, 10))
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .map(|message| message.id())
            .collect();
        assert_eq!(2, remaining.len());
        assert!(!remaining.contains(&ids[0]));
        Ok(())
    })
    .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn test_backlog_counts_pending_messages_and_keeps_the_trace_context() -> anyhow::Result<()> {
    let (_postgres, pool) = start_postgres().await?;

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let repository = PostgresOutboxRepository::<Kind>::new(&pool);
        assert_eq!(0, repository.backlog()?.started);

        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let message = OutboxMessage::new(Uuid::new_v4(), Kind, &"traced")?
            .with_trace_context(Some(traceparent.to_string()));
        insert_outbox_message(&mut *pool.get()?, &message)?;
        let failed = insert_messages(&pool, 2)?;
        for id in &failed {
            repository.mark_attempt_failed(*id, 1)?;
        }

        let backlog = repository.backlog()?;
        assert_eq!(1, backlog.started);
        assert_eq!(2, backlog.failed);
        let oldest = backlog.oldest_started.expect("a STARTED message exists");
        assert!((Utc::now() - oldest).num_seconds() < 60);

        let claimed = repository.claim_batch(10)?;
        assert_eq!(Some(traceparent), claimed.messages[0].trace_context());
        Ok(())
    })
    .await??;

    Ok(())
}

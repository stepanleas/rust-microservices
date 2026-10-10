use diesel::PgConnection;
use diesel::RunQueryDsl;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use std::time::Duration;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone)]
pub struct DbSettings {
    pub url: String,
    /// Should be at least the number of actix blocking threads that use the database plus the
    /// Kafka listener and the outbox relay, or requests queue up waiting for a connection.
    pub max_connections: u32,
    pub connection_timeout: Duration,
}

/// Builds the pool and applies the pending migrations. Blocking: call it before the async
/// runtime starts serving, or from `spawn_blocking`.
pub fn connect(settings: &DbSettings, migrations: EmbeddedMigrations) -> anyhow::Result<DbPool> {
    let manager = ConnectionManager::<PgConnection>::new(settings.url.as_str());
    let pool = Pool::builder()
        .max_size(settings.max_connections)
        .connection_timeout(settings.connection_timeout)
        .build(manager)?;

    pool.get()?
        .run_pending_migrations(migrations)
        .map_err(|error| anyhow::anyhow!("Failed to run database migrations: {error}"))?;

    Ok(pool)
}

/// Readiness probes time out after 1s by default (Kubernetes `timeoutSeconds`), so give up first.
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_millis(800);

/// Ready when a connection can be taken from the pool in time and answers `SELECT 1`.
#[derive(Clone)]
pub struct DbHealthCheck {
    pool: DbPool,
}

impl DbHealthCheck {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl crate::health::HealthCheck for DbHealthCheck {
    fn name(&self) -> &'static str {
        "database"
    }

    fn check(&self) -> Result<(), String> {
        let mut connection = self
            .pool
            .get_timeout(HEALTH_CHECK_TIMEOUT)
            .map_err(|error| error.to_string())?;

        diesel::sql_query("SELECT 1")
            .execute(&mut connection)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

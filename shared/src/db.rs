use diesel::PgConnection;
use diesel::RunQueryDsl;
use diesel::r2d2::event::{AcquireEvent, CheckinEvent, CheckoutEvent, ReleaseEvent, TimeoutEvent};
use diesel::r2d2::{ConnectionManager, HandleEvent, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone)]
pub struct DbSettings {
    pub url: String,
    /// Should be at least the number of actix blocking threads that use the database plus the
    /// Kafka listener and the outbox relay, or requests queue up waiting for a connection.
    pub max_connections: u32,
    pub connection_timeout: Duration,
}

/// How long [`connect`] keeps retrying while the database is unreachable, e.g. while Postgres
/// is still starting next to the service in Kubernetes. Keeps the pod waiting (not ready)
/// instead of crash-looping with a growing restart back-off.
pub const STARTUP_RETRY_WINDOW: Duration = Duration::from_secs(120);

/// Builds the pool and applies the pending migrations, retrying the connection for up to
/// [`STARTUP_RETRY_WINDOW`]. Blocking: call it before the async runtime starts serving, or from
/// `spawn_blocking`.
pub fn connect(settings: &DbSettings, migrations: EmbeddedMigrations) -> anyhow::Result<DbPool> {
    connect_retrying(settings, migrations, STARTUP_RETRY_WINDOW)
}

/// [`connect`] with a custom retry window. Only connection failures are retried; a failing
/// migration is returned right away.
pub fn connect_retrying(
    settings: &DbSettings,
    migrations: EmbeddedMigrations,
    window: Duration,
) -> anyhow::Result<DbPool> {
    let deadline = Instant::now() + window;
    let mut backoff = Duration::from_secs(1);

    let pool = loop {
        let manager = ConnectionManager::<PgConnection>::new(settings.url.as_str());
        let built = Pool::builder()
            .max_size(settings.max_connections)
            .connection_timeout(settings.connection_timeout)
            .event_handler(Box::new(PoolMetrics::default()))
            .build(manager);

        match built {
            Ok(pool) => break pool,
            Err(error) if Instant::now() + backoff < deadline => {
                tracing::warn!("Database not reachable yet: {error}. Retrying in {backoff:?}");
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(Duration::from_secs(10));
            }
            Err(error) => return Err(error.into()),
        }
    };

    run_migrations(&mut *pool.get()?, migrations)?;

    Ok(pool)
}

/// Publishes the pool's state as metrics:
///
/// - `db_pool_connections{state="idle"|"in_use"}` (gauges)
/// - `db_pool_wait_seconds`: how long getting a connection took (histogram)
/// - `db_pool_timeouts_total`: requests that got no connection within the connection timeout
#[derive(Debug, Default)]
struct PoolMetrics {
    open: AtomicI64,
    in_use: AtomicI64,
}

impl PoolMetrics {
    fn publish(&self) {
        let open = self.open.load(Ordering::Relaxed);
        let in_use = self.in_use.load(Ordering::Relaxed);
        metrics::gauge!("db_pool_connections", "state" => "in_use").set(in_use as f64);
        metrics::gauge!("db_pool_connections", "state" => "idle").set((open - in_use) as f64);
    }
}

impl HandleEvent for PoolMetrics {
    fn handle_acquire(&self, _: AcquireEvent) {
        self.open.fetch_add(1, Ordering::Relaxed);
        self.publish();
    }

    fn handle_release(&self, _: ReleaseEvent) {
        self.open.fetch_sub(1, Ordering::Relaxed);
        self.publish();
    }

    fn handle_checkout(&self, event: CheckoutEvent) {
        self.in_use.fetch_add(1, Ordering::Relaxed);
        metrics::histogram!("db_pool_wait_seconds").record(event.duration().as_secs_f64());
        self.publish();
    }

    fn handle_timeout(&self, event: TimeoutEvent) {
        metrics::counter!("db_pool_timeouts_total").increment(1);
        metrics::histogram!("db_pool_wait_seconds").record(event.timeout().as_secs_f64());
    }

    fn handle_checkin(&self, _: CheckinEvent) {
        self.in_use.fetch_sub(1, Ordering::Relaxed);
        self.publish();
    }
}

/// Key of the advisory lock that serializes migrations (any constant works, one per database).
const MIGRATIONS_LOCK: i64 = 0x6d69_6772_6174_6521;

/// Applies the pending migrations while holding a session advisory lock, so replicas starting
/// at the same time apply them one after the other instead of racing on the same `CREATE TABLE`.
/// Needs a session-level connection: through PgBouncer, use `pool_mode = session`.
fn run_migrations(
    connection: &mut PgConnection,
    migrations: EmbeddedMigrations,
) -> anyhow::Result<()> {
    diesel::sql_query(format!("SELECT pg_advisory_lock({MIGRATIONS_LOCK})")).execute(connection)?;
    let result = connection
        .run_pending_migrations(migrations)
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("Failed to run database migrations: {error}"));
    diesel::sql_query(format!("SELECT pg_advisory_unlock({MIGRATIONS_LOCK})"))
        .execute(connection)?;

    result
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

//! `shared::db` against a real database (testcontainers, needs Docker).

use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use shared::db::DbSettings;
use std::time::Duration;
use testcontainers::ImageExt;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("tests/migrations");

/// Replicas starting together on an empty database all come up: the advisory lock makes them
/// apply the migrations one after the other (without it, all but one fail on `CREATE TABLE`).
#[tokio::test(flavor = "multi_thread")]
async fn test_replicas_starting_together_migrate_once() -> anyhow::Result<()> {
    let postgres = Postgres::default().start().await?;
    let port = postgres.get_host_port_ipv4(5432).await?;
    let settings = DbSettings {
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres"),
        max_connections: 2,
        connection_timeout: Duration::from_secs(10),
    };

    let replicas: Vec<_> = (0..4)
        .map(|_| {
            let settings = settings.clone();
            std::thread::spawn(move || shared::db::connect(&settings, MIGRATIONS).map(|_| ()))
        })
        .collect();

    for replica in replicas {
        replica.join().expect("replica panicked")?;
    }
    Ok(())
}

/// A service started before its database waits for it instead of failing.
#[tokio::test(flavor = "multi_thread")]
async fn test_connect_waits_for_a_database_that_starts_later() -> anyhow::Result<()> {
    // A free port for Postgres to appear on later.
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let settings = DbSettings {
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres"),
        max_connections: 2,
        connection_timeout: Duration::from_secs(2),
    };
    let connecting = std::thread::spawn(move || {
        shared::db::connect_retrying(&settings, MIGRATIONS, Duration::from_secs(90)).map(|_| ())
    });

    tokio::time::sleep(Duration::from_secs(3)).await;
    let _postgres = Postgres::default()
        .with_mapped_port(port, 5432.tcp())
        .start()
        .await?;

    tokio::task::spawn_blocking(move || connecting.join().expect("connect panicked")).await??;
    Ok(())
}

#[test]
fn test_connect_gives_up_after_the_window() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let settings = DbSettings {
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres"),
        max_connections: 1,
        connection_timeout: Duration::from_millis(500),
    };

    let started = std::time::Instant::now();
    let result = shared::db::connect_retrying(&settings, MIGRATIONS, Duration::from_secs(3));

    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_secs(10));
}

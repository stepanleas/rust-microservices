use crate::config::Settings;
use application::ports::out::publishers::CustomerMessagePublisher;
use application::ports::out::repositories::CustomerRepository;
use application::use_cases::{CreateCustomer, UpdateCustomer};
use infrastructure::postgres::PostgresCustomerRepository;
use messaging::publishers::CustomerKafkaPublisher;
use presentation::state::AppState;
use shared::db::{DbPool, DbSettings};
use shared::kafka::KafkaProducer;
use std::sync::Arc;

/// Builds the pool and runs the migrations.
///
/// Blocking (database connection): call it before the server starts.
pub fn connect_database(settings: &Settings) -> anyhow::Result<DbPool> {
    shared::db::connect(
        &DbSettings {
            url: settings.database_url.clone(),
            max_connections: settings.db_max_connections,
            connection_timeout: settings.db_connection_timeout,
        },
        infrastructure::MIGRATIONS,
    )
}

/// Builds the adapters and the use cases.
pub fn build_app_state(settings: &Settings, pool: &DbPool) -> AppState {
    let customer_repository: Arc<dyn CustomerRepository> =
        Arc::new(PostgresCustomerRepository::new(pool));
    let customer_publisher: Arc<dyn CustomerMessagePublisher> =
        Arc::new(CustomerKafkaPublisher::new(KafkaProducer::new(vec![
            settings.kafka_host.clone(),
        ])));

    AppState {
        environment: settings.environment.clone(),

        create_customer: Arc::new(CreateCustomer::new(
            customer_repository.clone(),
            customer_publisher.clone(),
        )),
        update_customer: Arc::new(UpdateCustomer::new(customer_repository, customer_publisher)),
    }
}

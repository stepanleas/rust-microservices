use crate::config::Settings;
use application::ports::out::publishers::CustomerMessagePublisher;
use application::ports::out::repositories::CustomerRepository;
use application::use_cases::{CreateCustomer, UpdateCustomer};
use infrastructure::postgres::PostgresCustomerRepository;
use messaging::publishers::{CUSTOMER_TOPICS, CustomerKafkaPublisher};
use presentation::state::AppState;
use shared::db::DbSettings;
use shared::kafka::KafkaProducer;
use std::sync::Arc;

/// Builds the pool (running migrations), the adapters and the use cases.
///
/// Blocking (database connection): call it before the server starts.
pub fn build_app_state(settings: &Settings) -> anyhow::Result<AppState> {
    let pool = shared::db::connect(
        &DbSettings {
            url: settings.database_url.clone(),
            max_connections: settings.db_max_connections,
            connection_timeout: settings.db_connection_timeout,
        },
        infrastructure::MIGRATIONS,
    )?;

    let customer_repository: Arc<dyn CustomerRepository> =
        Arc::new(PostgresCustomerRepository::new(&pool));
    let customer_publisher: Arc<dyn CustomerMessagePublisher> =
        Arc::new(CustomerKafkaPublisher::new(KafkaProducer::new(
            vec![settings.kafka_host.clone()],
            &CUSTOMER_TOPICS,
        )));

    Ok(AppState {
        environment: settings.environment.clone(),

        create_customer: Arc::new(CreateCustomer::new(
            customer_repository.clone(),
            customer_publisher.clone(),
        )),
        update_customer: Arc::new(UpdateCustomer::new(customer_repository, customer_publisher)),
    })
}

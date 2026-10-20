//! Builds the object graph: pool, repositories, use cases, HTTP state, Kafka listener and
//! outbox relay.

use crate::config::Settings;
use application::outbox::OutboxMessageType;
use application::ports::out::publishers::PaymentMessagePublisher;
use application::ports::out::repositories::PaymentRepository;
use application::use_cases::{
    ApplicationPaymentRequestMessageListener, CreatePayment, FindPaymentById, FindPaymentByOrderId,
    ProcessPayment, UpdatePayment,
};
use infrastructure::postgres::PostgresPaymentRepository;
use infrastructure::{DbPool, MIGRATIONS};
use messaging::handlers::PaymentMessageHandler;
use messaging::publishers::{PaymentKafkaPublisher, PaymentOutboxKafkaPublisher};
use presentation::state::AppState;
use shared::db::DbSettings;
use shared::kafka::RetryPolicy;
use shared::outbox::postgres::PostgresOutboxRepository;
use shared::outbox::relay::{OutboxRelay, RelaySettings};
use shared::shutdown::Shutdown;
use std::sync::Arc;
use std::thread;

pub const CONSUMER_GROUP: &str = "payment-service-group";

/// Connects to Postgres and runs the pending migrations. Blocking.
pub fn connect_database(settings: &Settings) -> anyhow::Result<DbPool> {
    shared::db::connect(
        &DbSettings {
            url: settings.database_url.clone(),
            max_connections: settings.db_max_connections,
            connection_timeout: settings.db_connection_timeout,
        },
        MIGRATIONS,
    )
}

fn kafka_hosts(settings: &Settings) -> Vec<String> {
    vec![settings.kafka_host.clone()]
}

pub fn build_app_state(settings: &Settings, pool: &DbPool) -> AppState {
    let repository: Arc<dyn PaymentRepository> = Arc::new(PostgresPaymentRepository::new(pool));
    let publisher: Arc<dyn PaymentMessagePublisher> =
        Arc::new(PaymentKafkaPublisher::new(kafka_hosts(settings)));

    AppState {
        environment: settings.environment.clone(),
        create_payment: Arc::new(CreatePayment::new(repository.clone(), publisher.clone())),
        update_payment: Arc::new(UpdatePayment::new(repository.clone(), publisher)),
        find_payment_by_id: Arc::new(FindPaymentById::new(repository.clone())),
        find_payment_by_order_id: Arc::new(FindPaymentByOrderId::new(repository)),
    }
}

/// Handles the topics this service consumes (`payment-request`).
pub fn build_message_handler(pool: &DbPool) -> PaymentMessageHandler {
    let process_payment = Arc::new(ProcessPayment::new(Arc::new(
        PostgresPaymentRepository::new(pool),
    )));

    PaymentMessageHandler::new(Arc::new(ApplicationPaymentRequestMessageListener::new(
        process_payment,
    )))
}

/// Publishes `outbox_messages` to Kafka (`payment-response`).
pub fn build_outbox_relay(settings: &Settings, pool: &DbPool) -> OutboxRelay<OutboxMessageType> {
    OutboxRelay::new(
        Arc::new(PostgresOutboxRepository::new(pool)),
        Arc::new(PaymentOutboxKafkaPublisher::new(kafka_hosts(settings))),
        RelaySettings {
            poll_interval: settings.outbox_poll_interval,
            batch_size: settings.outbox_batch_size,
            max_attempts: settings.outbox_max_attempts,
            retention: settings.outbox_retention,
        },
    )
}

/// Consumes the payment topics on a dedicated thread until `shutdown`.
pub fn spawn_kafka_listener(
    settings: &Settings,
    pool: &DbPool,
    shutdown: &Shutdown,
) -> thread::JoinHandle<()> {
    let hosts = kafka_hosts(settings);
    let handler = Arc::new(build_message_handler(pool));

    let shutdown = shutdown.clone();

    thread::spawn(move || {
        shared::kafka::listen(
            hosts,
            CONSUMER_GROUP.to_string(),
            handler,
            RetryPolicy::default(),
            &shutdown,
        )
    })
}

/// Publishes the outbox messages on a dedicated thread until `shutdown`.
pub fn spawn_outbox_relay(
    settings: &Settings,
    pool: &DbPool,
    shutdown: &Shutdown,
) -> thread::JoinHandle<()> {
    build_outbox_relay(settings, pool).spawn(shutdown)
}

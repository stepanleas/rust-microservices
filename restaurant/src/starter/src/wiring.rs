//! Builds the object graph: pool, repositories, use cases, HTTP state, Kafka listener and
//! outbox relay.

use crate::config::Settings;
use application::outbox::OutboxMessageType;
use application::ports::out::repositories::{OrderApprovalRepository, ProductRepository};
use application::use_cases::{
    ApplicationProductMessageListener, ApplicationRestaurantApprovalMessageListener, ApproveOrder,
    CancelOrderApproval, GetOrderApproval, GetProduct, ListProducts,
};
use infrastructure::postgres::{
    PostgresOrderApprovalRepository, PostgresOutboxRepository, PostgresProductRepository,
};
use infrastructure::{DbPool, MIGRATIONS};
use messaging::handlers::RestaurantMessageHandler;
use messaging::publishers::{OutboxKafkaPublisher, PUBLISHED_TOPICS};
use presentation::state::AppState;
use shared::db::DbSettings;
use shared::kafka::{KafkaProducer, RetryPolicy};
use shared::outbox::relay::{OutboxRelay, RelaySettings};
use std::sync::Arc;
use std::thread;

pub const CONSUMER_GROUP: &str = "restaurant-service-group";

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

struct Repositories {
    order_approvals: Arc<dyn OrderApprovalRepository>,
    products: Arc<dyn ProductRepository>,
}

fn repositories(pool: &DbPool) -> Repositories {
    Repositories {
        order_approvals: Arc::new(PostgresOrderApprovalRepository::new(pool)),
        products: Arc::new(PostgresProductRepository::new(pool)),
    }
}

pub fn build_app_state(settings: &Settings, pool: &DbPool) -> AppState {
    let repositories = repositories(pool);

    AppState {
        environment: settings.environment.clone(),
        get_order_approval: Arc::new(GetOrderApproval::new(repositories.order_approvals)),
        get_product: Arc::new(GetProduct::new(repositories.products.clone())),
        list_products: Arc::new(ListProducts::new(repositories.products)),
    }
}

pub fn build_message_handler(pool: &DbPool) -> RestaurantMessageHandler {
    let repositories = repositories(pool);

    let approve_order = Arc::new(ApproveOrder::new(
        repositories.order_approvals.clone(),
        repositories.products.clone(),
    ));
    let cancel_order_approval = Arc::new(CancelOrderApproval::new(
        repositories.order_approvals,
        repositories.products.clone(),
    ));

    RestaurantMessageHandler::new(
        Arc::new(ApplicationProductMessageListener::new(
            repositories.products,
        )),
        Arc::new(ApplicationRestaurantApprovalMessageListener::new(
            approve_order,
            cancel_order_approval,
        )),
    )
}

pub fn build_outbox_relay(
    settings: &Settings,
    pool: &DbPool,
    producer: KafkaProducer,
) -> OutboxRelay<OutboxMessageType> {
    OutboxRelay::new(
        Arc::new(PostgresOutboxRepository::new(pool)),
        Arc::new(OutboxKafkaPublisher::new(producer)),
        RelaySettings {
            poll_interval: settings.outbox_poll_interval,
            batch_size: settings.outbox_batch_size,
            max_attempts: settings.outbox_max_attempts,
        },
    )
}

/// Consumes the restaurant topics on a dedicated thread, forever.
pub fn spawn_kafka_listener(settings: &Settings, pool: &DbPool) -> thread::JoinHandle<()> {
    let hosts = vec![settings.kafka_host.clone()];
    let handler = Arc::new(build_message_handler(pool));

    thread::spawn(move || {
        shared::kafka::listen(
            hosts,
            CONSUMER_GROUP.to_string(),
            handler,
            RetryPolicy::default(),
        )
    })
}

/// Publishes the outbox messages on a dedicated thread, forever.
pub fn spawn_outbox_relay(settings: &Settings, pool: &DbPool) -> thread::JoinHandle<()> {
    let producer = KafkaProducer::new(vec![settings.kafka_host.clone()], PUBLISHED_TOPICS);
    build_outbox_relay(settings, pool, producer).spawn()
}

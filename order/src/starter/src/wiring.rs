//! Builds the object graph of the order service: adapters, use cases, the HTTP state, the Kafka
//! listener and the outbox relay.

use application::outbox::OutboxMessageType;
use application::ports::out::repositories::{
    CustomerRepository, OrderRepository, ProductRepository,
};
use application::ports::out::saga_store::OrderSagaRepository;
use application::saga::payment_saga::PaymentSaga;
use application::saga::restaurant_approval_saga::RestaurantApprovalSaga;
use application::use_cases::{
    CreateOrder, HandlePaymentResponse, HandleRestaurantApprovalResponse, SyncCustomer,
    SyncProduct, TrackOrder,
};
use infrastructure::DbPool;
use infrastructure::postgres::{
    PostgresCustomerRepository, PostgresOrderRepository, PostgresOrderSagaRepository,
    PostgresProductRepository,
};
use messaging::handlers::OrderMessageHandler;
use messaging::publishers::{OutboxKafkaPublisher, PUBLISHED_TOPICS};
use presentation::state::AppState;
use shared::kafka::{KafkaProducer, RetryPolicy};
use shared::outbox::postgres::PostgresOutboxRepository;
use shared::outbox::relay::{OutboxRelay, RelaySettings};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Every use case of the service, sharing one set of repositories.
pub struct UseCases {
    pub create_order: Arc<CreateOrder>,
    pub track_order: Arc<TrackOrder>,
    pub sync_customer: Arc<SyncCustomer>,
    pub sync_product: Arc<SyncProduct>,
    pub restaurant_approval_response: Arc<HandleRestaurantApprovalResponse>,
    pub payment_response: Arc<HandlePaymentResponse>,
}

impl UseCases {
    pub fn new(pool: &DbPool) -> Self {
        let order_repository: Arc<dyn OrderRepository> =
            Arc::new(PostgresOrderRepository::new(pool));
        let saga_repository: Arc<dyn OrderSagaRepository> =
            Arc::new(PostgresOrderSagaRepository::new(pool));
        let customer_repository: Arc<dyn CustomerRepository> =
            Arc::new(PostgresCustomerRepository::new(pool));
        let product_repository: Arc<dyn ProductRepository> =
            Arc::new(PostgresProductRepository::new(pool));

        Self {
            create_order: Arc::new(CreateOrder::new(
                saga_repository.clone(),
                customer_repository.clone(),
                product_repository.clone(),
            )),
            track_order: Arc::new(TrackOrder::new(order_repository.clone())),
            sync_customer: Arc::new(SyncCustomer::new(customer_repository)),
            sync_product: Arc::new(SyncProduct::new(product_repository)),
            restaurant_approval_response: Arc::new(HandleRestaurantApprovalResponse::new(
                RestaurantApprovalSaga::new(saga_repository.clone(), order_repository.clone()),
            )),
            payment_response: Arc::new(HandlePaymentResponse::new(PaymentSaga::new(
                saga_repository,
                order_repository,
            ))),
        }
    }

    pub fn app_state(&self, environment: &str) -> AppState {
        AppState {
            environment: environment.to_string(),
            create_order: self.create_order.clone(),
            track_order: self.track_order.clone(),
        }
    }

    pub fn message_handler(&self) -> OrderMessageHandler {
        OrderMessageHandler::new(
            self.sync_customer.clone(),
            self.sync_product.clone(),
            self.restaurant_approval_response.clone(),
            self.payment_response.clone(),
        )
    }
}

/// Consumes every topic of the order service on a dedicated thread, forever.
pub fn start_kafka_listener(
    kafka_host: &str,
    group_id: &str,
    handler: OrderMessageHandler,
) -> JoinHandle<()> {
    let hosts = vec![kafka_host.to_string()];
    let group_id = group_id.to_string();

    std::thread::spawn(move || {
        shared::kafka::listen(hosts, group_id, Arc::new(handler), RetryPolicy::default())
    })
}

/// The relay that publishes the saga messages stored in the outbox to Kafka.
pub fn outbox_relay(
    pool: &DbPool,
    kafka_host: &str,
    settings: RelaySettings,
) -> OutboxRelay<OutboxMessageType> {
    let publisher = OutboxKafkaPublisher::new(KafkaProducer::new(
        vec![kafka_host.to_string()],
        &PUBLISHED_TOPICS,
    ));

    OutboxRelay::new(
        Arc::new(PostgresOutboxRepository::<OutboxMessageType>::new(pool)),
        Arc::new(publisher),
        settings,
    )
}

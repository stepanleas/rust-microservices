use crate::commands::{CreateOrderCommand, CreateOrderItemDto};
use crate::mappers::OrderMapper;
use crate::outbox::{OrderApprovalEventPayload, OrderOutboxMessage, OutboxMessageType};
use crate::ports::out::repositories::{CustomerRepository, ProductRepository};
use crate::ports::out::saga_store::OrderSagaRepository;
use domain::aggregates::product::Product;
use domain::saga::OrderSaga;
use shared::domain::value_objects::{CustomerId, ProductId};
use shared::error::{AppError, AppResult};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

/// Places an order: creates a `PENDING` order and starts the order saga.
pub struct CreateOrder {
    saga_repository: Arc<dyn OrderSagaRepository>,
    customer_repository: Arc<dyn CustomerRepository>,
    product_repository: Arc<dyn ProductRepository>,
}

impl CreateOrder {
    pub fn new(
        saga_repository: Arc<dyn OrderSagaRepository>,
        customer_repository: Arc<dyn CustomerRepository>,
        product_repository: Arc<dyn ProductRepository>,
    ) -> Self {
        Self {
            saga_repository,
            customer_repository,
            product_repository,
        }
    }

    /// The order, the saga and the restaurant approval request are stored in one transaction
    /// (transactional outbox); the outbox relay publishes the request to Kafka afterwards.
    /// Returns the tracking id of the order.
    pub fn execute(&self, command: CreateOrderCommand) -> AppResult<Uuid> {
        self.check_customer(command.customer_id())?;
        self.check_products_quantity(command.items())?;

        let order = OrderMapper::map_create_order_command_to_domain_entity(command);
        order.validate()?;

        let saga = OrderSaga::start(order.id());
        let payload = OrderApprovalEventPayload::from_order(&order, saga.id());
        let message = OrderOutboxMessage::new(
            saga.id(),
            OutboxMessageType::RestaurantApprovalRequest,
            &payload,
        )?;

        self.saga_repository.start(&order, &saga, &message)?;
        metrics::counter!("saga_started_total").increment(1);
        tracing::info!(
            "Order created with id: {}, saga {} started",
            order.id(),
            saga.id(),
        );

        Ok(order.tracking_id())
    }

    fn check_customer(&self, customer_id: Uuid) -> AppResult<()> {
        match self
            .customer_repository
            .find_by_id(CustomerId::from_uuid(customer_id))?
        {
            Some(_) => Ok(()),
            None => Err(AppError::business_rule(format!(
                "Could not find customer with id: {customer_id}"
            ))),
        }
    }

    fn retrieve_products(&self, items: &[CreateOrderItemDto]) -> AppResult<Vec<Product>> {
        let product_ids: Vec<Uuid> = items.iter().map(|item| item.product_id()).collect();
        let products = self.product_repository.find_by_ids(
            product_ids
                .iter()
                .map(|product_id| ProductId::from_uuid(*product_id))
                .collect(),
        )?;

        let requested: HashSet<_> = product_ids.iter().cloned().collect();
        let found: HashSet<_> = products
            .iter()
            .map(|product| *product.id().as_uuid())
            .collect();

        let missing: Vec<_> = requested.difference(&found).collect();

        if !missing.is_empty() {
            return Err(AppError::business_rule(format!(
                "Missing product ids {missing:?}"
            )));
        }

        Ok(products)
    }

    fn check_products_quantity(&self, items: &[CreateOrderItemDto]) -> AppResult<()> {
        let products = self.retrieve_products(items)?;

        let products_map: HashMap<Uuid, Product> = products
            .into_iter()
            .map(|product| (*product.id().as_uuid(), product))
            .collect();

        for item in items {
            if let Some(product) = products_map.get(&item.product_id())
                && item.quantity() > product.quantity()
            {
                return Err(AppError::business_rule(format!(
                    "Insufficient quantity for product {}: requested {}, available {}",
                    product.title(),
                    item.quantity(),
                    product.quantity(),
                )));
            }
        }

        Ok(())
    }
}

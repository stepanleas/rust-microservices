use crate::commands::{CreateOrderCommand, CreateOrderItemDto};
use crate::outbox::{OrderApprovalEventPayload, OrderOutboxMessage, OutboxMessageType};
use crate::ports::out::repositories::{MockCustomerRepository, MockProductRepository};
use crate::ports::out::saga_store::MockOrderSagaRepository;
use crate::use_cases::CreateOrder;
use domain::aggregates::customer::Customer;
use domain::aggregates::order::Order;
use domain::aggregates::product::Product;
use domain::enums::OrderStatus;
use domain::saga::{OrderSaga, SagaStatus};
use mockall::predicate;
use rust_decimal::dec;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, ProductId};
use shared::error::AppError;
use shared::outbox::OutboxStatus;
use std::sync::Arc;
use uuid::Uuid;

fn create_customer(customer_id: CustomerId) -> Customer {
    Customer::new(customer_id, "Artellas".into(), "Mike".into(), "Dane".into())
}

fn create_product(product_id: ProductId, quantity: i32) -> anyhow::Result<Product> {
    Ok(Product::new(
        product_id,
        "Sample Product".into(),
        quantity,
        Money::from_str("30.0", iso::USD)?,
    ))
}

struct Mocks {
    saga: MockOrderSagaRepository,
    customer: MockCustomerRepository,
    product: MockProductRepository,
}

impl Mocks {
    fn new() -> Self {
        Self {
            saga: MockOrderSagaRepository::new(),
            customer: MockCustomerRepository::new(),
            product: MockProductRepository::new(),
        }
    }

    fn use_case(self) -> CreateOrder {
        CreateOrder::new(
            Arc::new(self.saga),
            Arc::new(self.customer),
            Arc::new(self.product),
        )
    }
}

fn command(
    customer_id: CustomerId,
    first_product_id: ProductId,
    first_quantity: i32,
    second_product_id: ProductId,
) -> CreateOrderCommand {
    CreateOrderCommand::new(
        *customer_id.as_uuid(),
        dec!(450.0),
        vec![
            CreateOrderItemDto::new(
                *first_product_id.as_uuid(),
                first_quantity,
                dec!(30.0),
                dec!(300.0),
            ),
            CreateOrderItemDto::new(*second_product_id.as_uuid(), 5, dec!(30.0), dec!(150.0)),
        ],
    )
}

#[test]
fn test_create_order_execute() -> anyhow::Result<()> {
    let customer_id = CustomerId::new();
    let first_product_id = ProductId::new();
    let second_product_id = ProductId::new();
    let mut mocks = Mocks::new();

    mocks
        .customer
        .expect_find_by_id()
        .once()
        .with(predicate::eq(customer_id))
        .returning(move |_| Ok(Some(create_customer(customer_id))));

    mocks
        .product
        .expect_find_by_ids()
        .once()
        .with(predicate::eq(vec![first_product_id, second_product_id]))
        .returning(move |_| {
            Ok(vec![
                create_product(first_product_id, 10)?,
                create_product(second_product_id, 5)?,
            ])
        });

    mocks
        .saga
        .expect_start()
        .once()
        .withf(
            move |order: &Order, saga: &OrderSaga, message: &OrderOutboxMessage| {
                let payload: OrderApprovalEventPayload = match message.payload_as() {
                    Ok(payload) => payload,
                    Err(_) => return false,
                };

                order.customer_id() == customer_id
                    && order.status() == OrderStatus::Pending
                    && order.items().len() == 2
                    && order.price().to_string() == "$450.00"
                    && saga.order_id() == order.id()
                    && saga.status() == SagaStatus::Started
                    && message.saga_id() == saga.id()
                    && message.status() == OutboxStatus::Started
                    && message.message_type() == OutboxMessageType::RestaurantApprovalRequest
                    && payload.saga_id == saga.id()
                    && payload.order_id == *order.id().as_uuid()
                    && payload.customer_id == *customer_id.as_uuid()
                    && payload.price == "450.00"
                    && payload.products.len() == 2
                    && payload.products[0].id == *first_product_id.as_uuid()
                    && payload.products[0].quantity == 10
            },
        )
        .returning(|_, _, _| Ok(()));

    let tracking_id = mocks.use_case().execute(command(
        customer_id,
        first_product_id,
        10,
        second_product_id,
    ))?;

    assert_ne!(Uuid::nil(), tracking_id);

    Ok(())
}

#[test]
fn test_create_order_with_non_existent_customer_is_a_business_rule_error() {
    let customer_id = CustomerId::new();
    let mut mocks = Mocks::new();

    mocks
        .customer
        .expect_find_by_id()
        .once()
        .with(predicate::eq(customer_id))
        .returning(|_| Ok(None));
    mocks.product.expect_find_by_ids().never();
    mocks.saga.expect_start().never();

    let result =
        mocks
            .use_case()
            .execute(command(customer_id, ProductId::new(), 10, ProductId::new()));

    let error = result.unwrap_err();
    assert!(matches!(error, AppError::BusinessRule(_)), "{error:?}");
    assert_eq!(
        format!("Could not find customer with id: {}", customer_id),
        error.to_string(),
    );
}

#[test]
fn test_create_order_when_customer_lookup_fails_is_an_internal_error() {
    let mut mocks = Mocks::new();

    mocks
        .customer
        .expect_find_by_id()
        .once()
        .returning(|_| Err(anyhow::anyhow!("timed out waiting for connection")));
    mocks.product.expect_find_by_ids().never();
    mocks.saga.expect_start().never();

    let result = mocks.use_case().execute(command(
        CustomerId::new(),
        ProductId::new(),
        10,
        ProductId::new(),
    ));

    let error = result.unwrap_err();
    assert!(matches!(error, AppError::Internal(_)), "{error:?}");
    assert_eq!("timed out waiting for connection", error.to_string());
}

#[test]
fn test_create_order_with_missing_product_is_a_business_rule_error() {
    let customer_id = CustomerId::new();
    let first_product_id = ProductId::new();
    let second_product_id = ProductId::new();
    let mut mocks = Mocks::new();

    mocks
        .customer
        .expect_find_by_id()
        .returning(move |_| Ok(Some(create_customer(customer_id))));
    mocks
        .product
        .expect_find_by_ids()
        .once()
        .returning(move |_| Ok(vec![create_product(first_product_id, 10)?]));
    mocks.saga.expect_start().never();

    let result = mocks.use_case().execute(command(
        customer_id,
        first_product_id,
        10,
        second_product_id,
    ));

    let error = result.unwrap_err();
    assert!(matches!(error, AppError::BusinessRule(_)), "{error:?}");
    assert_eq!(
        format!("Missing product ids [{}]", second_product_id),
        error.to_string(),
    );
}

#[test]
fn test_create_order_when_product_lookup_fails_is_an_internal_error() {
    let customer_id = CustomerId::new();
    let mut mocks = Mocks::new();

    mocks
        .customer
        .expect_find_by_id()
        .returning(move |_| Ok(Some(create_customer(customer_id))));
    mocks
        .product
        .expect_find_by_ids()
        .once()
        .returning(|_| Err(anyhow::anyhow!("database is down")));
    mocks.saga.expect_start().never();

    let result =
        mocks
            .use_case()
            .execute(command(customer_id, ProductId::new(), 10, ProductId::new()));

    assert!(matches!(result, Err(AppError::Internal(_))));
}

#[test]
fn test_create_order_with_insufficient_quantity_is_a_business_rule_error() {
    let customer_id = CustomerId::new();
    let first_product_id = ProductId::new();
    let second_product_id = ProductId::new();
    let mut mocks = Mocks::new();

    mocks
        .customer
        .expect_find_by_id()
        .once()
        .with(predicate::eq(customer_id))
        .returning(move |_| Ok(Some(create_customer(customer_id))));
    mocks
        .product
        .expect_find_by_ids()
        .once()
        .with(predicate::eq(vec![first_product_id, second_product_id]))
        .returning(move |_| {
            Ok(vec![
                create_product(first_product_id, 10)?,
                create_product(second_product_id, 5)?,
            ])
        });
    mocks.saga.expect_start().never();

    let result = mocks.use_case().execute(command(
        customer_id,
        first_product_id,
        12,
        second_product_id,
    ));

    let error = result.unwrap_err();
    assert!(matches!(error, AppError::BusinessRule(_)), "{error:?}");
    assert_eq!(
        "Insufficient quantity for product Sample Product: requested 12, available 10",
        error.to_string(),
    );
}

#[test]
fn test_create_order_with_prices_that_do_not_add_up_is_a_business_rule_error() {
    let customer_id = CustomerId::new();
    let product_id = ProductId::new();
    let mut mocks = Mocks::new();

    mocks
        .customer
        .expect_find_by_id()
        .returning(move |_| Ok(Some(create_customer(customer_id))));
    mocks
        .product
        .expect_find_by_ids()
        .returning(move |_| Ok(vec![create_product(product_id, 10)?]));
    mocks.saga.expect_start().never();

    let result = mocks.use_case().execute(CreateOrderCommand::new(
        *customer_id.as_uuid(),
        dec!(20.0),
        vec![CreateOrderItemDto::new(
            *product_id.as_uuid(),
            2,
            dec!(30.0),
            dec!(60.0),
        )],
    ));

    let error = result.unwrap_err();
    assert!(matches!(error, AppError::BusinessRule(_)), "{error:?}");
    assert_eq!(
        "Total price: $20.00 is not equal to order items total: $60.00!",
        error.to_string(),
    );
}

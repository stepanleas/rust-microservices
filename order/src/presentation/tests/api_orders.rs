use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{App, test};
use application::ports::out::repositories::{
    MockCustomerRepository, MockOrderRepository, MockProductRepository,
};
use application::ports::out::saga_store::MockOrderSagaRepository;
use application::use_cases::{CreateOrder, TrackOrder};
use domain::aggregates::customer::Customer;
use domain::aggregates::order::{Order, OrderItem};
use domain::aggregates::product::Product;
use domain::enums::OrderStatus;
use presentation::state::AppState;
use rusty_money::{Money, iso};
use serde_json::{Value, json};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use std::sync::Arc;
use uuid::Uuid;

struct Mocks {
    order: MockOrderRepository,
    saga: MockOrderSagaRepository,
    customer: MockCustomerRepository,
    product: MockProductRepository,
}

impl Mocks {
    fn new() -> Self {
        Self {
            order: MockOrderRepository::new(),
            saga: MockOrderSagaRepository::new(),
            customer: MockCustomerRepository::new(),
            product: MockProductRepository::new(),
        }
    }

    fn into_state(self) -> AppState {
        AppState {
            environment: "test".into(),
            create_order: Arc::new(CreateOrder::new(
                Arc::new(self.saga),
                Arc::new(self.customer),
                Arc::new(self.product),
            )),
            track_order: Arc::new(TrackOrder::new(Arc::new(self.order))),
        }
    }
}

async fn call(mocks: Mocks, request: test::TestRequest) -> (StatusCode, Value) {
    let app = test::init_service(
        App::new()
            .app_data(Data::new(mocks.into_state()))
            .configure(presentation::api::configure),
    )
    .await;

    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = test::read_body(response).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[actix_web::test]
async fn test_create_order_returns_tracking_id() -> anyhow::Result<()> {
    let customer_id = CustomerId::new();
    let product_id = ProductId::new();
    let mut mocks = Mocks::new();

    mocks.customer.expect_find_by_id().returning(move |_| {
        Ok(Some(Customer::new(
            customer_id,
            "a".into(),
            "b".into(),
            "c".into(),
        )))
    });
    mocks.product.expect_find_by_ids().returning(move |_| {
        Ok(vec![Product::new(
            product_id,
            "Pizza".into(),
            10,
            Money::from_str("30.0", iso::USD)?,
        )])
    });
    mocks.saga.expect_start().once().returning(|_, _, _| Ok(()));

    let (status, body) = call(
        mocks,
        test::TestRequest::post().uri("/api/orders").set_json(json!({
            "customer_id": customer_id.as_uuid(),
            "price": 60.0,
            "items": [{ "product_id": product_id.as_uuid(), "quantity": 2, "price": 30.0, "sub_total": 60.0 }]
        })),
    )
    .await;

    assert_eq!(StatusCode::CREATED, status);
    assert!(Uuid::parse_str(body["data"]["tracking_id"].as_str().unwrap_or_default()).is_ok());

    Ok(())
}

#[actix_web::test]
async fn test_create_order_validation_error() {
    let mut mocks = Mocks::new();
    mocks.saga.expect_start().never();

    let (status, body) = call(
        mocks,
        test::TestRequest::post()
            .uri("/api/orders")
            .set_json(json!({
                "customer_id": Uuid::new_v4(),
                "price": 0.0,
                "items": []
            })),
    )
    .await;

    assert_eq!(StatusCode::BAD_REQUEST, status);
    assert_eq!(400, body["code"]);
    assert_eq!("Validation error", body["message"]);
    let fields: Vec<&str> = body["errors"]
        .as_array()
        .map(|errors| errors.iter().filter_map(|e| e["field"].as_str()).collect())
        .unwrap_or_default();
    assert!(fields.contains(&"price"), "{body}");
    assert!(fields.contains(&"items"), "{body}");
}

#[actix_web::test]
async fn test_create_order_with_unknown_customer_is_unprocessable() {
    let mut mocks = Mocks::new();
    mocks.customer.expect_find_by_id().returning(|_| Ok(None));
    mocks.saga.expect_start().never();

    let (status, body) = call(
        mocks,
        test::TestRequest::post().uri("/api/orders").set_json(json!({
            "customer_id": Uuid::new_v4(),
            "price": 30.0,
            "items": [{ "product_id": Uuid::new_v4(), "quantity": 1, "price": 30.0, "sub_total": 30.0 }]
        })),
    )
    .await;

    assert_eq!(StatusCode::UNPROCESSABLE_ENTITY, status);
    assert!(
        body["message"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Could not find customer with id")
    );
}

#[actix_web::test]
async fn test_track_order_returns_current_state() -> anyhow::Result<()> {
    let tracking_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let product_id = ProductId::new();
    let mut mocks = Mocks::new();

    mocks
        .order
        .expect_find_by_tracking_id()
        .returning(move |_| {
            Ok(Some(
                Order::builder()
                    .id(order_id)
                    .tracking_id(tracking_id)
                    .price(Money::from_str("60.0", iso::USD)?)
                    .status(OrderStatus::Canceled)
                    .failure_messages(vec!["Insufficient stock".into()])
                    .items(vec![OrderItem::new(
                        OrderItemId::new(),
                        order_id,
                        product_id,
                        2,
                        Money::from_str("30.0", iso::USD)?,
                        Money::from_str("60.0", iso::USD)?,
                    )])
                    .build(),
            ))
        });

    let (status, body) = call(
        mocks,
        test::TestRequest::get().uri(&format!("/api/orders/{tracking_id}")),
    )
    .await;

    assert_eq!(StatusCode::OK, status);
    assert_eq!(tracking_id.to_string(), body["data"]["tracking_id"]);
    assert_eq!(order_id.as_uuid().to_string(), body["data"]["id"]);
    assert_eq!("CANCELLED", body["data"]["status"]);
    assert_eq!(60.0, body["data"]["price"]);
    assert_eq!(
        json!(["Insufficient stock"]),
        body["data"]["failure_messages"]
    );
    assert_eq!(
        product_id.as_uuid().to_string(),
        body["data"]["items"][0]["product_id"]
    );
    assert_eq!(2, body["data"]["items"][0]["quantity"]);

    Ok(())
}

#[actix_web::test]
async fn test_track_unknown_order_is_not_found() {
    let mut mocks = Mocks::new();
    mocks
        .order
        .expect_find_by_tracking_id()
        .returning(|_| Ok(None));

    let (status, body) = call(
        mocks,
        test::TestRequest::get().uri(&format!("/api/orders/{}", Uuid::new_v4())),
    )
    .await;

    assert_eq!(StatusCode::NOT_FOUND, status);
    assert_eq!(404, body["code"]);
}

#[actix_web::test]
async fn test_track_order_with_invalid_tracking_id_is_bad_request() {
    let (status, body) = call(
        Mocks::new(),
        test::TestRequest::get().uri("/api/orders/not-a-uuid"),
    )
    .await;

    assert_eq!(StatusCode::BAD_REQUEST, status);
    assert_eq!(400, body["code"]);
}

#[actix_web::test]
async fn test_create_order_when_database_fails_is_internal_error() {
    let mut mocks = Mocks::new();
    mocks
        .customer
        .expect_find_by_id()
        .returning(|_| Err(anyhow::anyhow!("connection refused")));
    mocks.saga.expect_start().never();

    let (status, body) = call(
        mocks,
        test::TestRequest::post().uri("/api/orders").set_json(json!({
            "customer_id": Uuid::new_v4(),
            "price": 30.0,
            "items": [{ "product_id": Uuid::new_v4(), "quantity": 1, "price": 30.0, "sub_total": 30.0 }]
        })),
    )
    .await;

    assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, status);
    assert_eq!("Internal server error", body["message"]);
}

#[actix_web::test]
async fn test_create_order_with_unrepresentable_price_is_bad_request() {
    let mut mocks = Mocks::new();
    mocks.customer.expect_find_by_id().never();
    mocks.saga.expect_start().never();

    let (status, body) = call(
        mocks,
        test::TestRequest::post().uri("/api/orders").set_json(json!({
            "customer_id": Uuid::new_v4(),
            "price": 1.0e30,
            "items": [{ "product_id": Uuid::new_v4(), "quantity": 1, "price": 30.0, "sub_total": 30.0 }]
        })),
    )
    .await;

    assert_eq!(StatusCode::BAD_REQUEST, status);
    assert_eq!(400, body["code"]);
    assert!(
        body["message"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Invalid price value"),
        "{body}"
    );
}

#[actix_web::test]
async fn test_info_returns_environment() {
    let (status, body) = call(Mocks::new(), test::TestRequest::get().uri("/api/info")).await;

    assert_eq!(StatusCode::OK, status);
    assert_eq!("test", body["data"]["environment"]);
}

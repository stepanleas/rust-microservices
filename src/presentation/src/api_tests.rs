use crate::state::AppState;
use actix_web::http::StatusCode;
use actix_web::{App, test, web};
use application::ports::out::publishers::MockPaymentMessagePublisher;
use application::ports::out::repositories::MockPaymentRepository;
use application::use_cases::{CreatePayment, FindPaymentById, FindPaymentByOrderId, UpdatePayment};
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use rust_decimal::Decimal;
use serde_json::Value;
use shared::domain::value_objects::{CustomerId, OrderId};
use std::sync::Arc;
use uuid::Uuid;

fn state(repository: MockPaymentRepository, publisher: MockPaymentMessagePublisher) -> AppState {
    let repository = Arc::new(repository);
    let publisher = Arc::new(publisher);

    AppState {
        environment: "test".into(),
        create_payment: Arc::new(CreatePayment::new(repository.clone(), publisher.clone())),
        update_payment: Arc::new(UpdatePayment::new(repository.clone(), publisher)),
        find_payment_by_id: Arc::new(FindPaymentById::new(repository.clone())),
        find_payment_by_order_id: Arc::new(FindPaymentByOrderId::new(repository)),
    }
}

fn payment(payment_id: Uuid, order_id: Uuid) -> Payment {
    Payment::new(
        PaymentId::from_uuid(payment_id),
        OrderId::from_uuid(order_id),
        CustomerId::new(),
        Decimal::new(15000, 2),
        PaymentStatus::Completed,
    )
}

macro_rules! app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data(web::Data::new($state))
                .configure(crate::api::configure),
        )
        .await
    };
}

#[actix_web::test]
async fn test_get_payment_by_id() {
    let payment_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_id()
        .once()
        .returning(move |_| Ok(Some(payment(payment_id, order_id))));

    let app = app!(state(repository, MockPaymentMessagePublisher::new()));
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/payments/{payment_id}"))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::OK, response.status());
    let body: Value = test::read_body_json(response).await;
    assert_eq!(payment_id.to_string(), body["data"]["id"]);
    assert_eq!(order_id.to_string(), body["data"]["order_id"]);
    assert_eq!(150.0, body["data"]["price"]);
    assert_eq!("COMPLETED", body["data"]["payment_status"]);
    assert_eq!(serde_json::json!([]), body["data"]["failure_messages"]);
}

#[actix_web::test]
async fn test_get_payment_by_order_id_not_found() {
    let order_id = Uuid::new_v4();
    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));

    let app = app!(state(repository, MockPaymentMessagePublisher::new()));
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/payments/orders/{order_id}"))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::NOT_FOUND, response.status());
    let body: Value = test::read_body_json(response).await;
    assert_eq!(404, body["code"]);
    assert_eq!(
        format!("Could not find payment for order with id: {order_id}"),
        body["message"]
    );
}

#[actix_web::test]
async fn test_get_payment_with_malformed_id() {
    let app = app!(state(
        MockPaymentRepository::new(),
        MockPaymentMessagePublisher::new()
    ));
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/payments/not-a-uuid")
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::NOT_FOUND, response.status());
}

#[actix_web::test]
async fn test_create_payment_conflict_for_existing_order() {
    let order_id = Uuid::new_v4();
    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_order_id()
        .once()
        .returning(move |_| Ok(Some(payment(Uuid::new_v4(), order_id))));

    let app = app!(state(repository, MockPaymentMessagePublisher::new()));
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/payments")
            .set_json(serde_json::json!({
                "order_id": order_id,
                "customer_id": Uuid::new_v4(),
                "price": 10.5
            }))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::CONFLICT, response.status());
}

#[actix_web::test]
async fn test_create_payment() {
    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));
    repository.expect_save().once().returning(Ok);
    let mut publisher = MockPaymentMessagePublisher::new();
    publisher
        .expect_publish_created()
        .once()
        .returning(|_| Ok(()));

    let app = app!(state(repository, publisher));
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/payments")
            .set_json(serde_json::json!({
                "order_id": Uuid::new_v4(),
                "customer_id": Uuid::new_v4(),
                "price": 10.5
            }))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::CREATED, response.status());
    let body: Value = test::read_body_json(response).await;
    assert_eq!(10.5, body["data"]["price"]);
    assert_eq!("COMPLETED", body["data"]["payment_status"]);
}

#[actix_web::test]
async fn test_create_payment_validation_error() {
    let app = app!(state(
        MockPaymentRepository::new(),
        MockPaymentMessagePublisher::new()
    ));
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/payments")
            .set_json(serde_json::json!({
                "order_id": Uuid::new_v4(),
                "customer_id": Uuid::new_v4(),
                "price": 0.0
            }))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
    let body: Value = test::read_body_json(response).await;
    assert_eq!("price", body["errors"][0]["field"]);
}

#[actix_web::test]
async fn test_update_payment_with_invalid_status() {
    let app = app!(state(
        MockPaymentRepository::new(),
        MockPaymentMessagePublisher::new()
    ));
    let response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/api/payments/{}", Uuid::new_v4()))
            .set_json(serde_json::json!({
                "order_id": Uuid::new_v4(),
                "customer_id": Uuid::new_v4(),
                "price": 1.0,
                "payment_status": "PENDING"
            }))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[core::prelude::v1::test]
fn test_open_api_documents_every_endpoint() {
    let doc = serde_json::to_value(crate::api::docs::open_api_docs()).unwrap();
    let paths = doc["paths"].as_object().unwrap();

    for (path, method) in [
        ("/api/payments", "post"),
        ("/api/payments/{id}", "get"),
        ("/api/payments/{id}", "put"),
        ("/api/payments/orders/{order_id}", "get"),
        ("/api/health/startup", "get"),
        ("/api/health/live", "get"),
        ("/api/health/ready", "get"),
        ("/api/info", "get"),
    ] {
        let operation = &paths
            .get(path)
            .unwrap_or_else(|| panic!("missing path {path}"))[method];
        assert!(operation.is_object(), "missing {method} {path}");
        assert!(
            operation["operationId"].is_string(),
            "{method} {path} has no operationId"
        );
        assert!(
            operation["summary"].is_string(),
            "{method} {path} has no summary"
        );
    }

    let get_payment = &paths["/api/payments/{id}"]["get"]["responses"];
    for code in ["200", "400", "404", "500"] {
        assert!(get_payment[code].is_object(), "missing response {code}");
    }

    let schemas = doc["components"]["schemas"].as_object().unwrap();
    for schema in [
        "ErrorResponse",
        "PaymentResponse",
        "PaymentEnvelope",
        "CreatePaymentRequest",
        "UpdatePaymentRequest",
        "HealthCheckEnvelope",
        "AppInfoEnvelope",
    ] {
        assert!(schemas.contains_key(schema), "missing schema {schema}");
    }
}

#[actix_web::test]
async fn test_update_payment() {
    let payment_id = Uuid::new_v4();
    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_id()
        .once()
        .returning(move |_| Ok(Some(payment(payment_id, Uuid::new_v4()))));
    repository.expect_save().once().returning(Ok);
    let mut publisher = MockPaymentMessagePublisher::new();
    publisher
        .expect_publish_updated()
        .once()
        .returning(|_| Ok(()));

    let app = app!(state(repository, publisher));
    let response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/api/payments/{payment_id}"))
            .set_json(serde_json::json!({
                "order_id": Uuid::new_v4(),
                "customer_id": Uuid::new_v4(),
                "price": 2.5,
                "payment_status": "cancelled"
            }))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::OK, response.status());
    let body: Value = test::read_body_json(response).await;
    assert_eq!(payment_id.to_string(), body["data"]["id"]);
    assert_eq!(2.5, body["data"]["price"]);
    assert_eq!("CANCELLED", body["data"]["payment_status"]);
}

#[actix_web::test]
async fn test_get_payment_with_repository_error() {
    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_id()
        .once()
        .returning(|_| Err(anyhow::anyhow!("database is down")));

    let app = app!(state(repository, MockPaymentMessagePublisher::new()));
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/payments/{}", Uuid::new_v4()))
            .to_request(),
    )
    .await;

    assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, response.status());
    let body: Value = test::read_body_json(response).await;
    assert_eq!("Internal server error", body["message"]);
}

#[actix_web::test]
async fn test_app_info() {
    let app = app!(state(
        MockPaymentRepository::new(),
        MockPaymentMessagePublisher::new()
    ));
    let response =
        test::call_service(&app, test::TestRequest::get().uri("/api/info").to_request()).await;

    assert_eq!(StatusCode::OK, response.status());
    let body: Value = test::read_body_json(response).await;
    assert_eq!("test", body["data"]["environment"]);
}

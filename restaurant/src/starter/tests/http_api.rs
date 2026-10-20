mod common;

use actix_web::{App, test, web};
use application::outbox::restaurant_approval_response;
use application::ports::out::repositories::{OrderApprovalRepository, ProductRepository};
use domain::aggregates::order_approval::OrderApproval;
use domain::aggregates::product::Product;
use infrastructure::postgres::{PostgresOrderApprovalRepository, PostgresProductRepository};
use rust_decimal::dec;
use serde_json::Value;
use shared::db::DbHealthCheck;
use shared::domain::value_objects::{OrderId, ProductId};
use shared::web::health::HealthChecks;
use starter::config::Settings;
use starter::wiring::build_app_state;
use uuid::Uuid;

#[actix_web::test]
async fn test_http_api() -> anyhow::Result<()> {
    let (_postgres, pool) = common::start_postgres().await?;

    let product_id = ProductId::new();
    PostgresProductRepository::new(&pool).save(&Product::new(
        product_id,
        "Pizza".into(),
        7,
        dec!(12.50),
    ))?;

    let approvals = PostgresOrderApprovalRepository::new(&pool);
    let order_id = OrderId::new();
    let approval = OrderApproval::rejected(Uuid::new_v4(), order_id, vec!["Out of stock".into()]);
    approvals.save_with_outbox(&approval, &[], &restaurant_approval_response(&approval)?)?;

    let cancelled_order_id = OrderId::new();
    let mut cancelled = OrderApproval::approved(Uuid::new_v4(), cancelled_order_id);
    approvals.save_with_outbox(&cancelled, &[], &restaurant_approval_response(&cancelled)?)?;
    cancelled.cancel()?;
    approvals.save_cancellation(&cancelled, &[])?;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(build_app_state(&Settings::new(), &pool)))
            .app_data(web::Data::new(
                HealthChecks::new().with(DbHealthCheck::new(&pool)),
            ))
            .configure(presentation::api::configure),
    )
    .await;

    // GET /api/products
    let body: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get().uri("/api/products").to_request(),
    )
    .await;
    assert_eq!(1, body["data"].as_array().unwrap().len());
    assert_eq!("Pizza", body["data"][0]["title"]);
    assert_eq!(7, body["data"][0]["quantity"]);
    assert_eq!("12.50", body["data"][0]["price"]);

    // GET /api/products/{id}
    let body: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/products/{}", product_id.as_uuid()))
            .to_request(),
    )
    .await;
    assert_eq!(product_id.as_uuid().to_string(), body["data"]["id"]);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/products/{}", Uuid::new_v4()))
            .to_request(),
    )
    .await;
    assert_eq!(404, response.status().as_u16());

    // GET /api/order-approvals/{order_id}
    let body: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/order-approvals/{}", order_id.as_uuid()))
            .to_request(),
    )
    .await;
    assert_eq!("REJECTED", body["data"]["status"]);
    assert_eq!("Out of stock", body["data"]["failure_messages"][0]);
    assert_eq!(
        approval.saga_id().to_string(),
        body["data"]["saga_id"].as_str().unwrap()
    );

    let body: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/order-approvals/{}",
                cancelled_order_id.as_uuid()
            ))
            .to_request(),
    )
    .await;
    assert_eq!("CANCELLED", body["data"]["status"]);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/order-approvals/{}", Uuid::new_v4()))
            .to_request(),
    )
    .await;
    assert_eq!(404, response.status().as_u16());
    let body: Value = test::read_body_json(response).await;
    assert_eq!(404, body["code"]);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/order-approvals/not-a-uuid")
            .to_request(),
    )
    .await;
    assert_eq!(400, response.status().as_u16());

    // GET /api/info
    let body: Value =
        test::call_and_read_body_json(&app, test::TestRequest::get().uri("/api/info").to_request())
            .await;
    assert_eq!(Settings::new().environment, body["data"]["environment"]);

    // Health
    let body: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get()
            .uri("/api/health/ready")
            .to_request(),
    )
    .await;
    assert_eq!("Ok", body["data"]["status"]);
    assert_eq!("Ok", body["data"]["checks"]["database"]);

    Ok(())
}

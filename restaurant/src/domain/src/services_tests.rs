use crate::aggregates::product::Product;
use crate::enums::OrderApprovalStatus;
use crate::services::{RequestedProduct, RestaurantApprovalDomainService};
use rust_decimal::dec;
use shared::domain::value_objects::{OrderId, ProductId};
use uuid::Uuid;

fn product(product_id: ProductId, title: &str, quantity: i32) -> Product {
    Product::new(product_id, title.into(), quantity, dec!(10.00))
}

#[test]
fn test_approve_order_decreases_stock() {
    let first_product_id = ProductId::new();
    let second_product_id = ProductId::new();
    let mut products = vec![
        product(first_product_id, "Pizza", 10),
        product(second_product_id, "Pasta", 5),
    ];
    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();

    let approval = RestaurantApprovalDomainService::approve_order(
        saga_id,
        order_id,
        &[
            RequestedProduct::new(first_product_id, 3),
            RequestedProduct::new(second_product_id, 5),
        ],
        &mut products,
    );

    assert_eq!(OrderApprovalStatus::Approved, approval.status());
    assert_eq!(saga_id, approval.saga_id());
    assert_eq!(order_id, approval.order_id());
    assert!(approval.failure_messages().is_empty());
    assert_eq!(7, products[0].quantity());
    assert_eq!(0, products[1].quantity());
}

#[test]
fn test_approve_order_aggregates_duplicated_lines() {
    let product_id = ProductId::new();
    let mut products = vec![product(product_id, "Pizza", 5)];

    let approval = RestaurantApprovalDomainService::approve_order(
        Uuid::new_v4(),
        OrderId::new(),
        &[
            RequestedProduct::new(product_id, 3),
            RequestedProduct::new(product_id, 3),
        ],
        &mut products,
    );

    assert_eq!(OrderApprovalStatus::Rejected, approval.status());
    assert_eq!(
        vec!["Insufficient stock for product Pizza: requested 6, available 5".to_string()],
        approval.failure_messages(),
    );
    assert_eq!(5, products[0].quantity());
}

#[test]
fn test_reject_order_with_missing_product() {
    let known_product_id = ProductId::new();
    let missing_product_id = ProductId::new();
    let mut products = vec![product(known_product_id, "Pizza", 10)];

    let approval = RestaurantApprovalDomainService::approve_order(
        Uuid::new_v4(),
        OrderId::new(),
        &[
            RequestedProduct::new(known_product_id, 1),
            RequestedProduct::new(missing_product_id, 1),
        ],
        &mut products,
    );

    assert_eq!(OrderApprovalStatus::Rejected, approval.status());
    assert_eq!(
        vec![format!(
            "Product {} is not available in the restaurant",
            missing_product_id.as_uuid()
        )],
        approval.failure_messages(),
    );
    assert_eq!(
        10,
        products[0].quantity(),
        "stock must not change on reject"
    );
}

#[test]
fn test_reject_order_with_insufficient_stock() {
    let product_id = ProductId::new();
    let mut products = vec![product(product_id, "Pizza", 2)];

    let approval = RestaurantApprovalDomainService::approve_order(
        Uuid::new_v4(),
        OrderId::new(),
        &[RequestedProduct::new(product_id, 3)],
        &mut products,
    );

    assert_eq!(OrderApprovalStatus::Rejected, approval.status());
    assert_eq!(
        vec!["Insufficient stock for product Pizza: requested 3, available 2".to_string()],
        approval.failure_messages(),
    );
    assert_eq!(2, products[0].quantity());
}

#[test]
fn test_reject_order_without_products() {
    let approval = RestaurantApprovalDomainService::approve_order(
        Uuid::new_v4(),
        OrderId::new(),
        &[],
        &mut [],
    );

    assert_eq!(OrderApprovalStatus::Rejected, approval.status());
    assert_eq!(
        vec!["Order must contain at least one product".to_string()],
        approval.failure_messages(),
    );
}

#[test]
fn test_reject_order_with_invalid_quantity() {
    let product_id = ProductId::new();
    let mut products = vec![product(product_id, "Pizza", 2)];

    let approval = RestaurantApprovalDomainService::approve_order(
        Uuid::new_v4(),
        OrderId::new(),
        &[RequestedProduct::new(product_id, 0)],
        &mut products,
    );

    assert_eq!(OrderApprovalStatus::Rejected, approval.status());
    assert_eq!(
        vec![format!(
            "Invalid quantity 0 for product {}",
            product_id.as_uuid()
        )],
        approval.failure_messages(),
    );
}

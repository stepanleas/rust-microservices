use crate::ports::out::repositories::MockOrderRepository;
use crate::queries::TrackOrderQuery;
use crate::use_cases::TrackOrder;
use domain::aggregates::order::{Order, OrderItem};
use domain::enums::OrderStatus;
use mockall::predicate;
use rust_decimal::dec;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_track_order_returns_order_dto() -> anyhow::Result<()> {
    let tracking_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let product_id = ProductId::new();

    let mut mock_order_repository = MockOrderRepository::new();
    mock_order_repository
        .expect_find_by_tracking_id()
        .once()
        .with(predicate::eq(tracking_id))
        .returning(move |_| {
            Ok(Some(
                Order::builder()
                    .id(order_id)
                    .customer_id(CustomerId::new())
                    .tracking_id(tracking_id)
                    .price(Money::from_str("60.0", iso::USD)?)
                    .status(OrderStatus::Canceled)
                    .failure_messages(vec!["Out of stock".into()])
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

    let use_case = TrackOrder::new(Arc::new(mock_order_repository));
    let order = use_case.execute(TrackOrderQuery::new(tracking_id))?;

    assert_eq!(*order_id.as_uuid(), order.id);
    assert_eq!(tracking_id, order.tracking_id);
    assert_eq!(OrderStatus::Canceled, order.status);
    assert_eq!(dec!(60), order.price);
    assert_eq!(vec!["Out of stock".to_string()], order.failure_messages);
    assert_eq!(1, order.items.len());
    assert_eq!(*product_id.as_uuid(), order.items[0].product_id);
    assert_eq!(2, order.items[0].quantity);
    assert_eq!(dec!(30), order.items[0].price);
    assert_eq!(dec!(60), order.items[0].sub_total);

    Ok(())
}

#[test]
fn test_track_order_not_found() {
    let tracking_id = Uuid::new_v4();
    let mut mock_order_repository = MockOrderRepository::new();
    mock_order_repository
        .expect_find_by_tracking_id()
        .once()
        .returning(|_| Ok(None));

    let use_case = TrackOrder::new(Arc::new(mock_order_repository));
    let result = use_case.execute(TrackOrderQuery::new(tracking_id));

    let error = result.unwrap_err();
    assert!(matches!(error, AppError::NotFound(_)), "{error:?}");
    assert_eq!(
        format!("Could not find order by tracking id: {tracking_id}"),
        error.to_string()
    );
}

#[test]
fn test_track_order_repository_error_is_internal() {
    let mut mock_order_repository = MockOrderRepository::new();
    mock_order_repository
        .expect_find_by_tracking_id()
        .once()
        .returning(|_| Err(anyhow::anyhow!("database is down")));

    let use_case = TrackOrder::new(Arc::new(mock_order_repository));
    let result = use_case.execute(TrackOrderQuery::new(Uuid::new_v4()));

    assert!(matches!(result, Err(AppError::Internal(_))));
}

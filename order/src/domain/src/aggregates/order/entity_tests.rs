use crate::aggregates::order::Order;
use crate::aggregates::order::OrderItem;
use crate::enums::OrderStatus;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use uuid::Uuid;

fn make_item(order_id: OrderId, price: &str, quantity: i32) -> anyhow::Result<OrderItem> {
    let price = Money::from_str(price, iso::USD)?;
    let sub_total = price.mul(quantity)?;

    Ok(OrderItem::new(
        OrderItemId::new(),
        order_id,
        ProductId::new(),
        quantity,
        price,
        sub_total,
    ))
}

#[test]
fn test_create_order() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let customer_id = CustomerId::new();

    let order = Order::new(order_id, customer_id);

    assert_eq!(order_id, order.id());
    assert_eq!(customer_id, order.customer_id());
    assert_ne!(Uuid::nil(), order.tracking_id());
    assert_eq!("$0.00", order.price().to_string());
    assert_eq!(0, order.items().len());
    assert_eq!(OrderStatus::Pending, order.status());

    Ok(())
}

#[test]
fn test_validate_order_items_price_should_match_order_price() -> anyhow::Result<()> {
    let mut order = Order::new(OrderId::new(), CustomerId::new());
    order.set_price(Money::from_str("120.0", iso::USD)?);

    let first_item = make_item(order.id(), "10.0", 2)?;
    let second_item = make_item(order.id(), "25.0", 4)?;

    order.set_items(vec![first_item, second_item]);

    let result = order.validate();
    assert!(result.is_ok(), "Expected valid order to pass validation");

    Ok(())
}

#[test]
fn test_validate_order_items_price_should_not_match_order_price() -> anyhow::Result<()> {
    let mut order = Order::new(OrderId::new(), CustomerId::new());
    order.set_price(Money::from_str("20.0", iso::USD)?);

    let first_item = make_item(order.id(), "15.0", 2)?;
    let second_item = make_item(order.id(), "25.0", 4)?;

    order.set_items(vec![first_item, second_item]);

    let result = order.validate();
    assert!(result.is_err(), "Expected invalid order to pass validation");

    let message = result.unwrap_err().to_string();
    assert_eq!(
        "Total price: $20.00 is not equal to order items total: $130.00!",
        message,
    );

    Ok(())
}

#[test]
fn test_validate_invalid_sub_total_for_order_item() -> anyhow::Result<()> {
    let mut order = Order::new(OrderId::new(), CustomerId::new());
    order.set_price(Money::from_str("20.0", iso::USD)?);

    let invalid_item = OrderItem::new(
        OrderItemId::new(),
        order.id(),
        ProductId::new(),
        2,
        Money::from_str("10.0", iso::USD).unwrap_or(Money::from_minor(0, iso::USD)),
        Money::from_str("15.0", iso::USD).unwrap_or(Money::from_minor(0, iso::USD)),
    );

    order.set_items(vec![invalid_item]);

    let result = order.validate();
    assert!(result.is_err());

    let message = result.unwrap_err().to_string();
    assert_eq!("Order item price is not valid!", message);

    Ok(())
}

#[test]
fn test_builder_keeps_persisted_state() -> anyhow::Result<()> {
    let tracking_id = Uuid::new_v4();

    let order = Order::builder()
        .tracking_id(tracking_id)
        .status(OrderStatus::Approved)
        .failure_messages(vec!["some failure".into()])
        .price(Money::from_str("10.0", iso::USD)?)
        .build();

    assert_eq!(tracking_id, order.tracking_id());
    assert_eq!(OrderStatus::Approved, order.status());
    assert_eq!(vec!["some failure".to_string()], order.failure_messages());
    assert_eq!("$10.00", order.price().to_string());

    Ok(())
}

#[test]
fn test_approve_then_pay_order() -> anyhow::Result<()> {
    let mut order = Order::new(OrderId::new(), CustomerId::new());

    order.approve()?;
    assert_eq!(OrderStatus::Approved, order.status());

    order.pay()?;
    assert_eq!(OrderStatus::Paid, order.status());

    Ok(())
}

#[test]
fn test_pay_requires_approved_order() {
    let mut order = Order::new(OrderId::new(), CustomerId::new());

    let result = order.pay();

    assert!(result.is_err());
    assert_eq!(
        "Order is not in correct state for pay operation: Pending!",
        result.unwrap_err().to_string(),
    );
    assert_eq!(OrderStatus::Pending, order.status());
}

#[test]
fn test_approve_requires_pending_order() -> anyhow::Result<()> {
    let mut order = Order::new(OrderId::new(), CustomerId::new());
    order.approve()?;

    let result = order.approve();

    assert!(result.is_err());
    assert_eq!(OrderStatus::Approved, order.status());

    Ok(())
}

#[test]
fn test_cancel_pending_and_approved_orders() -> anyhow::Result<()> {
    let mut rejected = Order::new(OrderId::new(), CustomerId::new());
    rejected.cancel(vec!["Product is out of stock".into(), "".into()])?;
    assert_eq!(OrderStatus::Canceled, rejected.status());
    assert_eq!(
        vec!["Product is out of stock".to_string()],
        rejected.failure_messages(),
    );

    let mut payment_failed = Order::new(OrderId::new(), CustomerId::new());
    payment_failed.approve()?;
    payment_failed.cancel(vec!["Payment failed".into()])?;
    assert_eq!(OrderStatus::Canceled, payment_failed.status());

    Ok(())
}

#[test]
fn test_cancel_paid_order_is_not_allowed() -> anyhow::Result<()> {
    let mut order = Order::new(OrderId::new(), CustomerId::new());
    order.approve()?;
    order.pay()?;

    assert!(order.cancel(vec![]).is_err());
    assert_eq!(OrderStatus::Paid, order.status());

    Ok(())
}

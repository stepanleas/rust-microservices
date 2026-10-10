use crate::aggregates::order::OrderItem;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{OrderId, OrderItemId, ProductId};

#[test]
fn test_create_order_item() -> anyhow::Result<()> {
    let order_item_id = OrderItemId::new();
    let order_id = OrderId::new();
    let product_id = ProductId::new();

    let order_item = OrderItem::new(
        order_item_id,
        order_id,
        product_id,
        2,
        Money::from_str("30.0", iso::USD)?,
        Money::from_str("60.0", iso::USD)?,
    );

    assert_eq!(order_item_id, order_item.id());
    assert_eq!(order_id, order_item.order_id());
    assert_eq!(product_id, order_item.product_id());
    assert_eq!(2, order_item.quantity());
    assert_eq!("$30.00", order_item.price().to_string());
    assert_eq!("$60.00", order_item.sub_total().to_string());
    assert!(order_item.is_price_valid());

    Ok(())
}

#[test]
fn test_create_order_item_with_not_valid_price() -> anyhow::Result<()> {
    let order_item_id = OrderItemId::new();
    let order_id = OrderId::new();
    let product_id = ProductId::new();

    let order_item = OrderItem::new(
        order_item_id,
        order_id,
        product_id,
        2,
        Money::from_str("30.0", iso::USD)?,
        Money::from_str("55.0", iso::USD)?,
    );

    assert_eq!(order_item_id, order_item.id());
    assert_eq!(order_id, order_item.order_id());
    assert_eq!(product_id, order_item.product_id());
    assert_eq!(2, order_item.quantity());
    assert_eq!("$30.00", order_item.price().to_string());
    assert_eq!("$55.00", order_item.sub_total().to_string());
    assert!(!order_item.is_price_valid());

    Ok(())
}

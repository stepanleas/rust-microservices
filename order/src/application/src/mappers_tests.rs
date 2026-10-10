use crate::commands::{CreateOrderCommand, CreateOrderItemDto};
use crate::mappers::OrderMapper;
use rust_decimal::dec;
use uuid::Uuid;

#[test]
fn test_map_create_order_command_to_domain_entity() -> anyhow::Result<()> {
    let customer_id = Uuid::new_v4();
    let first_product_id = Uuid::new_v4();
    let second_product_id = Uuid::new_v4();

    let command = CreateOrderCommand::new(
        customer_id,
        dec!(450.0),
        vec![
            CreateOrderItemDto::new(first_product_id, 10, dec!(30.0), dec!(300.0)),
            CreateOrderItemDto::new(second_product_id, 5, dec!(30.0), dec!(150.0)),
        ],
    );

    let order = OrderMapper::map_create_order_command_to_domain_entity(command);

    assert_ne!(Uuid::nil().to_string(), order.id().as_uuid().to_string());
    assert_eq!(
        customer_id.to_string(),
        order.customer_id().as_uuid().to_string(),
    );
    assert_eq!("$450.00", order.price().to_string());
    assert_eq!(2, order.items().len());

    assert_ne!(
        Uuid::nil().to_string(),
        order.items()[0].id().as_uuid().to_string(),
    );
    assert_eq!(order.id(), order.items()[0].order_id());
    assert_eq!(
        first_product_id.to_string(),
        order.items()[0].product_id().as_uuid().to_string(),
    );
    assert_eq!(10, order.items()[0].quantity());
    assert_eq!("$30.00", order.items()[0].price().to_string());
    assert_eq!("$300.00", order.items()[0].sub_total().to_string());

    assert_ne!(
        Uuid::nil().to_string(),
        order.items()[1].id().as_uuid().to_string(),
    );
    assert_eq!(order.id(), order.items()[1].order_id());
    assert_eq!(
        second_product_id.to_string(),
        order.items()[1].product_id().as_uuid().to_string(),
    );
    assert_eq!(5, order.items()[1].quantity());
    assert_eq!("$30.00", order.items()[1].price().to_string());
    assert_eq!("$150.00", order.items()[1].sub_total().to_string());

    Ok(())
}

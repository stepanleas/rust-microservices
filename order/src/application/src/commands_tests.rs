use crate::commands::{CreateOrderCommand, CreateOrderItemDto};
use rust_decimal::dec;
use uuid::Uuid;

#[test]
fn test_create_order_command() -> anyhow::Result<()> {
    let customer_id = Uuid::new_v4();
    let items = vec![
        CreateOrderItemDto::new(Uuid::new_v4(), 2, dec!(50.0), dec!(100.0)),
        CreateOrderItemDto::new(Uuid::new_v4(), 3, dec!(25.0), dec!(75.0)),
    ];

    let command = CreateOrderCommand::new(customer_id, dec!(150.0), items);

    assert_eq!(command.customer_id(), customer_id);
    assert_eq!(command.price(), dec!(150.0));

    assert_eq!(2, command.items().len());
    assert_eq!(2, command.items()[0].quantity());
    assert_eq!(dec!(50.0), command.items()[0].price());
    assert_eq!(dec!(100.0), command.items()[0].sub_total());

    assert_eq!(3, command.items()[1].quantity());
    assert_eq!(dec!(25.0), command.items()[1].price());
    assert_eq!(dec!(75.0), command.items()[1].sub_total());

    Ok(())
}

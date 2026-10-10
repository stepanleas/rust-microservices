use crate::commands::{ApproveOrderCommand, CancelOrderApprovalCommand, OrderProductDto};
use rust_decimal::dec;
use uuid::Uuid;

#[test]
fn test_create_approve_order_command() {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let command = ApproveOrderCommand::new(
        saga_id,
        order_id,
        customer_id,
        dec!(30.00),
        vec![OrderProductDto::new(product_id, 3)],
    );

    assert_eq!(saga_id, command.saga_id());
    assert_eq!(order_id, command.order_id());
    assert_eq!(customer_id, command.customer_id());
    assert_eq!(dec!(30.00), command.price());
    assert_eq!(1, command.products().len());
    assert_eq!(product_id, command.products()[0].product_id());
    assert_eq!(3, command.products()[0].quantity());
}

#[test]
fn test_create_cancel_order_approval_command() {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let command = CancelOrderApprovalCommand::new(
        saga_id,
        order_id,
        vec![OrderProductDto::new(product_id, 2)],
    );

    assert_eq!(saga_id, command.saga_id());
    assert_eq!(order_id, command.order_id());
    assert_eq!(product_id, command.products()[0].product_id());
    assert_eq!(2, command.products()[0].quantity());
}

use crate::commands::{CreatePaymentCommand, UpdatePaymentCommand};
use domain::enums::PaymentStatus;
use rust_decimal::Decimal;
use uuid::Uuid;

#[test]
fn test_create_payment_command() {
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let price = Decimal::new(10050, 2);

    let command = CreatePaymentCommand::new(order_id, customer_id, price);

    assert_eq!(command.order_id(), order_id);
    assert_eq!(command.customer_id(), customer_id);
    assert_eq!(command.price(), price);
}

#[test]
fn test_update_payment_command() {
    let payment_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let price = Decimal::new(10050, 2);

    let command = UpdatePaymentCommand::new(
        payment_id,
        order_id,
        customer_id,
        price,
        PaymentStatus::Completed,
    );

    assert_eq!(command.id(), payment_id);
    assert_eq!(command.order_id(), order_id);
    assert_eq!(command.customer_id(), customer_id);
    assert_eq!(command.price(), price);
    assert_eq!(command.payment_status(), PaymentStatus::Completed);
}

#[test]
fn test_process_payment_command() {
    use crate::commands::ProcessPaymentCommand;

    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let price = Decimal::new(10050, 2);

    let command = ProcessPaymentCommand::new(saga_id, order_id, customer_id, price);

    assert_eq!(saga_id, command.saga_id());
    assert_eq!(order_id, command.order_id());
    assert_eq!(customer_id, command.customer_id());
    assert_eq!(price, command.price());
}

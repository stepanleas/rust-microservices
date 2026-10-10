use crate::aggregates::payment::{Payment, PaymentId};
use crate::enums::PaymentStatus;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};

#[test]
fn test_create_payment() {
    let payment_id = PaymentId::new();
    let order_id = OrderId::new();
    let customer_id = CustomerId::new();
    let price = Decimal::new(10050, 2); // 100.50

    let payment = Payment::new(
        payment_id,
        order_id,
        customer_id,
        price,
        PaymentStatus::Completed,
    );

    assert_eq!(payment_id, payment.id());
    assert_eq!(order_id, payment.order_id());
    assert_eq!(customer_id, payment.customer_id());
    assert_eq!(price, payment.price());
    assert_eq!(PaymentStatus::Completed, payment.payment_status());
}

#[test]
fn test_process_payment_with_positive_price_completes() {
    let mut payment = Payment::builder()
        .price(Decimal::new(15000, 2))
        .payment_status(PaymentStatus::Failed)
        .build();

    payment.process();

    assert_eq!(PaymentStatus::Completed, payment.payment_status());
    assert!(payment.is_completed());
    assert!(payment.failure_messages().is_empty());
}

#[test]
fn test_process_payment_with_zero_price_fails() {
    let mut payment = Payment::builder().price(Decimal::ZERO).build();

    payment.process();

    assert_eq!(PaymentStatus::Failed, payment.payment_status());
    assert!(!payment.is_completed());
    assert_eq!(
        vec!["Total price must be greater than zero!".to_string()],
        payment.failure_messages(),
    );
}

#[test]
fn test_process_payment_with_negative_price_fails() {
    let mut payment = Payment::builder().price(Decimal::new(-100, 2)).build();

    payment.process();

    assert_eq!(PaymentStatus::Failed, payment.payment_status());
}

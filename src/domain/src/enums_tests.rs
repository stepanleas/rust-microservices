use crate::enums::PaymentStatus;

#[test]
fn test_payment_status_conversions() {
    for status in [
        PaymentStatus::Completed,
        PaymentStatus::Cancelled,
        PaymentStatus::Failed,
    ] {
        assert_eq!(Some(status), PaymentStatus::parse(status.as_str()));
        assert_eq!(status.as_str(), status.to_string());
    }

    assert_eq!(Some(PaymentStatus::Failed), PaymentStatus::parse("failed"));
    assert_eq!(None, PaymentStatus::parse("UNKNOWN"));
}

#[test]
fn test_payment_status_defaults_to_completed() {
    assert_eq!(PaymentStatus::Completed, PaymentStatus::default());
}

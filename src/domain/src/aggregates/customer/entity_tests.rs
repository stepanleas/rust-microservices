use crate::aggregates::customer::Customer;
use shared::domain::value_objects::CustomerId;

#[test]
fn test_create_customer() -> anyhow::Result<()> {
    let customer_id = CustomerId::new();
    let customer = Customer::new(
        customer_id,
        "Artellas".to_string(),
        "John".to_string(),
        "Doe".to_string(),
    );

    assert_eq!(customer_id, customer.id());
    assert_eq!("Artellas", customer.user_name());
    assert_eq!("John", customer.first_name());
    assert_eq!("Doe", customer.last_name());

    Ok(())
}

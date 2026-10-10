use super::*;
use shared::contracts::AvroMessage;
use shared::domain::value_objects::CustomerId;

fn john() -> Customer {
    Customer::new(
        CustomerId::new(),
        "Artellas".to_string(),
        "John".to_string(),
        "Doe".to_string(),
    )
}

#[test]
fn test_customer_created_round_trip() -> anyhow::Result<()> {
    let customer = john();
    let customer_id = customer.id();
    let event = CustomerCreatedEvent::new(customer);
    let model = customer_created_to_avro(&event);

    let decoded = CustomerCreatedEventAvroModel::from_avro_bytes(&model.to_avro_bytes()?)?;

    assert_eq!(vec![model], decoded);
    assert_eq!(event.id().to_string(), decoded[0].id);
    assert_eq!(customer_id.to_string(), decoded[0].customer.id);
    assert_eq!("Artellas", decoded[0].customer.user_name);
    assert_eq!("John", decoded[0].customer.first_name);
    assert_eq!("Doe", decoded[0].customer.last_name);

    Ok(())
}

#[test]
fn test_customer_updated_round_trip() -> anyhow::Result<()> {
    let customer = john();
    let customer_id = customer.id();
    let event = CustomerUpdatedEvent::new(customer);
    let model = customer_updated_to_avro(&event);

    let decoded = CustomerUpdatedEventAvroModel::from_avro_bytes(&model.to_avro_bytes()?)?;

    assert_eq!(vec![model], decoded);
    assert_eq!(customer_id.to_string(), decoded[0].customer.id);

    Ok(())
}

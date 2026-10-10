use super::*;
use rusty_money::{Money, iso};
use shared::contracts::AvroMessage;
use shared::domain::value_objects::ProductId;

fn pizza() -> anyhow::Result<Product> {
    Ok(Product::builder()
        .title("Pizza")
        .quantity(3)
        .price(Money::from_str("25", iso::USD)?)
        .build())
}

/// Consumers (order, restaurant) parse the price as a plain decimal, e.g. "25.00", not "$25.00".
#[test]
fn test_price_is_published_as_plain_decimal() -> anyhow::Result<()> {
    let product = pizza()?;
    let product_id = product.id();

    let bytes = product_created_to_avro(&ProductCreatedEvent::new(product)).to_avro_bytes()?;
    let decoded = ProductCreatedEventAvroModel::from_avro_bytes(&bytes)?;

    assert_eq!(1, decoded.len());
    assert_eq!("25.00", decoded[0].product.price);
    assert_eq!(product_id.to_string(), decoded[0].product.id);
    assert_eq!("Pizza", decoded[0].product.title);
    assert_eq!(3, decoded[0].product.quantity);

    Ok(())
}

#[test]
fn test_product_updated_round_trip() -> anyhow::Result<()> {
    let event = ProductUpdatedEvent::new(pizza()?);
    let model = product_updated_to_avro(&event);

    let decoded = ProductUpdatedEventAvroModel::from_avro_bytes(&model.to_avro_bytes()?)?;

    assert_eq!(vec![model], decoded);
    assert_eq!(event.id().to_string(), decoded[0].id);

    Ok(())
}

#[test]
fn test_product_deleted_round_trip() -> anyhow::Result<()> {
    let product_id = ProductId::new();
    let event = ProductDeletedEvent::new(product_id);
    let model = product_deleted_to_avro(&event);

    let decoded = ProductDeletedEventAvroModel::from_avro_bytes(&model.to_avro_bytes()?)?;

    assert_eq!(product_id.to_string(), decoded[0].product_id);
    assert_eq!(vec![model], decoded);

    Ok(())
}

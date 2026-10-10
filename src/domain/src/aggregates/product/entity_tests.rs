use crate::aggregates::product::Product;
use rusty_money::{Money, iso};
use shared::domain::value_objects::ProductId;

#[test]
fn test_create_product() -> anyhow::Result<()> {
    let product_id = ProductId::new();
    let product = Product::new(
        product_id,
        "Laptop".to_string(),
        5,
        Money::from_str("99.99", iso::USD)?,
    );
    let total_cents = 99 * 100 + 99;
    println!("{}", total_cents);

    assert_eq!(product_id, product.id());
    assert_eq!("Laptop", product.title());
    assert_eq!(5, product.quantity());
    assert_eq!("$99.99", product.price().to_string());

    Ok(())
}

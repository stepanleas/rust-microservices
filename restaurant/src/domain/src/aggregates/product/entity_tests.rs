use crate::aggregates::product::Product;
use rust_decimal::dec;
use shared::domain::value_objects::ProductId;

#[test]
fn test_create_product() {
    let product_id = ProductId::new();

    let product = Product::builder()
        .id(product_id)
        .title("Pizza".into())
        .quantity(10)
        .price(dec!(12.50))
        .build();

    assert_eq!(product_id, product.id());
    assert_eq!("Pizza", product.title());
    assert_eq!(10, product.quantity());
    assert_eq!(dec!(12.50), product.price());
}

#[test]
fn test_decrease_stock() -> anyhow::Result<()> {
    let mut product = Product::new(ProductId::new(), "Pizza".into(), 10, dec!(12.50));

    product.decrease_stock(4)?;

    assert_eq!(6, product.quantity());

    Ok(())
}

#[test]
fn test_decrease_stock_with_insufficient_quantity() {
    let mut product = Product::new(ProductId::new(), "Pizza".into(), 3, dec!(12.50));

    let result = product.decrease_stock(4);

    assert!(result.is_err());
    assert_eq!(
        "Insufficient stock for product Pizza: requested 4, available 3",
        result.unwrap_err().to_string(),
    );
    assert_eq!(3, product.quantity());
}

#[test]
fn test_decrease_stock_with_non_positive_quantity() {
    let mut product = Product::new(ProductId::new(), "Pizza".into(), 3, dec!(12.50));

    let result = product.decrease_stock(0);

    assert!(result.is_err());
    assert_eq!(3, product.quantity());
}

#[test]
fn test_increase_stock() -> anyhow::Result<()> {
    let mut product = Product::new(ProductId::new(), "Pizza".into(), 3, dec!(12.50));

    product.increase_stock(4)?;

    assert_eq!(7, product.quantity());

    Ok(())
}

#[test]
fn test_increase_stock_with_non_positive_quantity() {
    let mut product = Product::new(ProductId::new(), "Pizza".into(), 3, dec!(12.50));

    assert!(product.increase_stock(0).is_err());
    assert!(product.increase_stock(-2).is_err());
    assert_eq!(3, product.quantity());
}

#[test]
fn test_increase_stock_overflow() {
    let mut product = Product::new(ProductId::new(), "Pizza".into(), i32::MAX, dec!(12.50));

    assert!(product.increase_stock(1).is_err());
    assert_eq!(i32::MAX, product.quantity());
}

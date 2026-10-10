use crate::aggregates::category::Category;
use crate::aggregates::product::Product;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CategoryId, ProductId};

#[test]
fn test_product_builder() {
    let product_id = ProductId::new();
    let category = Category::new(CategoryId::new(), "Pizza".into(), "Italian".into());

    let product = Product::builder()
        .id(product_id)
        .title("Margherita")
        .description("Tomato, mozzarella")
        .quantity(3)
        .price(Money::from_minor(1250, iso::USD))
        .category(category.clone())
        .build();

    assert_eq!(product_id, product.id());
    assert_eq!("Margherita", product.title());
    assert_eq!("Tomato, mozzarella", product.description());
    assert_eq!(3, product.quantity());
    assert_eq!("$12.50", product.price().to_string());
    assert_eq!(&category, product.category());
}

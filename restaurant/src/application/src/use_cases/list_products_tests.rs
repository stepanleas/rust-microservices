use crate::ports::out::repositories::MockProductRepository;
use crate::use_cases::ListProducts;
use domain::aggregates::product::Product;
use rust_decimal::dec;
use shared::domain::value_objects::ProductId;
use std::sync::Arc;

#[test]
fn test_list_products() -> anyhow::Result<()> {
    let product_id = ProductId::new();
    let mut repository = MockProductRepository::new();
    repository.expect_find_all().once().returning(move || {
        Ok(vec![Product::new(
            product_id,
            "Pizza".into(),
            4,
            dec!(9.99),
        )])
    });

    let products = ListProducts::new(Arc::new(repository)).execute()?;

    assert_eq!(1, products.len());
    assert_eq!(*product_id.as_uuid(), products[0].id);
    assert_eq!("Pizza", products[0].title);
    assert_eq!(4, products[0].quantity);
    assert_eq!(dec!(9.99), products[0].price);

    Ok(())
}

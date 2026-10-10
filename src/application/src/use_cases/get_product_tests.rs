use crate::ports::out::repositories::MockProductRepository;
use crate::use_cases::GetProduct;
use domain::aggregates::product::Product;
use rust_decimal::dec;
use shared::domain::value_objects::ProductId;
use shared::error::AppError;
use std::sync::Arc;

#[test]
fn test_get_product() -> anyhow::Result<()> {
    let product_id = ProductId::new();
    let mut repository = MockProductRepository::new();
    repository
        .expect_find_by_id()
        .once()
        .returning(move |id| Ok(Some(Product::new(id, "Pizza".into(), 4, dec!(9.99)))));

    let product = GetProduct::new(Arc::new(repository)).execute(*product_id.as_uuid())?;

    assert_eq!(*product_id.as_uuid(), product.id);

    Ok(())
}

#[test]
fn test_get_product_not_found() {
    let product_id = ProductId::new();
    let mut repository = MockProductRepository::new();
    repository
        .expect_find_by_id()
        .once()
        .returning(|_| Ok(None));

    let error = GetProduct::new(Arc::new(repository))
        .execute(*product_id.as_uuid())
        .unwrap_err();

    assert!(matches!(error, AppError::NotFound(_)));
    assert_eq!(
        format!("Could not find product with id: {product_id}"),
        error.to_string()
    );
}

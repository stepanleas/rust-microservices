use crate::ports::r#in::messages::ProductMessageListener;
use crate::ports::out::repositories::MockProductRepository;
use crate::use_cases::SyncProduct;
use domain::aggregates::product::Product;
use mockall::predicate;
use rusty_money::{Money, iso};
use shared::domain::value_objects::ProductId;
use shared::error::AppError;
use std::sync::Arc;

fn product(product_id: ProductId) -> anyhow::Result<Product> {
    Ok(Product::new(
        product_id,
        "Pizza".into(),
        3,
        Money::from_str("30.0", iso::USD)?,
    ))
}

#[test]
fn test_created_updated_and_deleted_products_are_synced() -> anyhow::Result<()> {
    let product_id = ProductId::new();
    let mut repository = MockProductRepository::new();
    repository
        .expect_save()
        .times(2)
        .withf(move |product| product.id() == product_id)
        .returning(|_| Ok(()));
    repository
        .expect_delete()
        .once()
        .with(predicate::eq(product_id))
        .returning(|_| Ok(()));

    let use_case = SyncProduct::new(Arc::new(repository));
    use_case.product_created(product(product_id)?)?;
    use_case.product_updated(product(product_id)?)?;
    use_case.product_deleted(product_id)?;

    Ok(())
}

#[test]
fn test_delete_error_is_propagated() {
    let mut repository = MockProductRepository::new();
    repository
        .expect_delete()
        .returning(|_| Err(anyhow::anyhow!("database is down")));

    let result = SyncProduct::new(Arc::new(repository)).product_deleted(ProductId::new());

    assert!(matches!(result, Err(AppError::Internal(_))));
}

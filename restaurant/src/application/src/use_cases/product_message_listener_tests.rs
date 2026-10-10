use crate::dtos::ProductDto;
use crate::ports::r#in::messages::ProductMessageListener;
use crate::ports::out::repositories::MockProductRepository;
use crate::use_cases::ApplicationProductMessageListener;
use mockall::predicate;
use rust_decimal::dec;
use shared::domain::value_objects::ProductId;
use std::sync::Arc;
use uuid::Uuid;

fn product(id: Uuid, quantity: i32) -> ProductDto {
    ProductDto {
        id,
        title: "Pizza".into(),
        quantity,
        price: dec!(1.00),
    }
}

#[test]
fn test_product_created_and_updated_save_product() -> anyhow::Result<()> {
    let product_id = Uuid::new_v4();
    let mut repository = MockProductRepository::new();
    repository
        .expect_save()
        .times(2)
        .withf(move |product| *product.id().as_uuid() == product_id && product.title() == "Pizza")
        .returning(|_| Ok(()));

    let listener = ApplicationProductMessageListener::new(Arc::new(repository));

    listener.product_created(product(product_id, 1))?;
    listener.product_updated(product(product_id, 2))?;

    Ok(())
}

#[test]
fn test_product_deleted_deletes_product() -> anyhow::Result<()> {
    let product_id = Uuid::new_v4();
    let mut repository = MockProductRepository::new();
    repository
        .expect_delete()
        .once()
        .with(predicate::eq(ProductId::from_uuid(product_id)))
        .returning(|_| Ok(()));

    ApplicationProductMessageListener::new(Arc::new(repository)).product_deleted(product_id)?;

    Ok(())
}

#[test]
fn test_product_created_propagates_error() {
    let mut repository = MockProductRepository::new();
    repository
        .expect_save()
        .returning(|_| Err(anyhow::anyhow!("boom")));

    let result = ApplicationProductMessageListener::new(Arc::new(repository))
        .product_created(product(Uuid::new_v4(), 1));

    assert!(result.is_err());
}

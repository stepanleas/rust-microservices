use crate::commands::{CreateProductCommand, DeleteProductCommand, UpdateProductCommand};
use crate::ports::out::publishers::MockProductMessagePublisher;
use crate::ports::out::repositories::{MockCategoryRepository, MockProductRepository};
use crate::queries::FindProductQuery;
use crate::use_cases::{CreateProduct, DeleteProduct, FindProduct, ListProducts, UpdateProduct};
use domain::aggregates::category::Category;
use domain::aggregates::product::Product;
use mockall::predicate;
use rust_decimal::dec;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CategoryId, ProductId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

fn electronics() -> Category {
    Category::new(
        CategoryId::new(),
        "Electronics".to_string(),
        "Electronic devices and gadgets".to_string(),
    )
}

fn category_repository_returning(category_id: CategoryId) -> MockCategoryRepository {
    let mut mock_category_repository = MockCategoryRepository::new();
    mock_category_repository
        .expect_find_by_id()
        .once()
        .with(predicate::eq(category_id))
        .returning(move |_| Ok(Some(electronics())));

    mock_category_repository
}

fn is_expected_laptop(product: &Product) -> bool {
    product.title() == "Laptop"
        && product.description() == "A high-performance laptop"
        && product.quantity() == 15
        && product.price().to_string() == "$30.50"
        && product.category().title() == "Electronics"
        && product.category().description() == "Electronic devices and gadgets"
}

fn create_laptop_command(category_id: CategoryId) -> CreateProductCommand {
    CreateProductCommand::new(
        "Laptop",
        "A high-performance laptop",
        15,
        dec!(30.5),
        *category_id.as_uuid(),
    )
}

#[test]
fn test_list_products_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockProductRepository::new();

    let first_category = electronics();
    let second_category = Category::new(
        CategoryId::new(),
        "Home Appliances".to_string(),
        "Appliances for home use".to_string(),
    );

    let expected_products = vec![
        Product::new(
            ProductId::new(),
            "Laptop".to_string(),
            "A high-performance laptop".to_string(),
            15,
            Money::from_decimal(dec!(30.5), iso::USD),
            first_category,
        ),
        Product::new(
            ProductId::new(),
            "Smartphone".to_string(),
            "A latest model smartphone".to_string(),
            7,
            Money::from_decimal(dec!(13.75), iso::USD),
            second_category,
        ),
    ];

    mock_repository
        .expect_list_all()
        .once()
        .returning(move || Ok(expected_products.clone()));

    let use_case = ListProducts::new(Arc::new(mock_repository));
    let product_dtos = use_case.execute()?;

    assert_eq!(2, product_dtos.len());

    assert_eq!("Laptop", product_dtos[0].title());
    assert_eq!("A high-performance laptop", product_dtos[0].description());
    assert_eq!(15, product_dtos[0].quantity());
    assert_eq!("$30.50", product_dtos[0].price().to_string());
    assert_eq!("Electronics", product_dtos[0].category().title());
    assert_eq!(
        "Electronic devices and gadgets",
        product_dtos[0].category().description(),
    );

    assert_eq!("Smartphone", product_dtos[1].title());
    assert_eq!("A latest model smartphone", product_dtos[1].description());
    assert_eq!(7, product_dtos[1].quantity());
    assert_eq!("$13.75", product_dtos[1].price().to_string());
    assert_eq!("Home Appliances", product_dtos[1].category().title());
    assert_eq!(
        "Appliances for home use",
        product_dtos[1].category().description(),
    );

    Ok(())
}

#[test]
fn test_find_product_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockProductRepository::new();

    let product_id = ProductId::new();
    let expected_product = Product::new(
        product_id,
        "Laptop".to_string(),
        "A high-performance laptop".to_string(),
        15,
        Money::from_str("30.5", iso::USD)?,
        electronics(),
    );

    mock_repository
        .expect_find_by_id()
        .once()
        .with(predicate::eq(product_id))
        .returning(move |_| Ok(Some(expected_product.clone())));

    let use_case = FindProduct::new(Arc::new(mock_repository));
    let product_dto = use_case.execute(FindProductQuery::new(*product_id.as_uuid()))?;

    assert_eq!("Laptop", product_dto.title());
    assert_eq!("A high-performance laptop", product_dto.description());
    assert_eq!(15, product_dto.quantity());
    assert_eq!("$30.50", product_dto.price().to_string());
    assert_eq!("Electronics", product_dto.category().title());
    assert_eq!(
        "Electronic devices and gadgets",
        product_dto.category().description(),
    );

    Ok(())
}

#[test]
fn test_find_product_not_found() {
    let mut mock_repository = MockProductRepository::new();
    let product_id = Uuid::new_v4();

    mock_repository
        .expect_find_by_id()
        .once()
        .returning(|_| Ok(None));

    let result =
        FindProduct::new(Arc::new(mock_repository)).execute(FindProductQuery::new(product_id));

    match result {
        Err(AppError::NotFound(message)) => assert_eq!(
            format!("Could not find product with id: {product_id}"),
            message
        ),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn test_create_product_execute() -> anyhow::Result<()> {
    let category_id = CategoryId::new();
    let mock_category_repository = category_repository_returning(category_id);
    let mut mock_product_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    mock_product_repository
        .expect_save()
        .once()
        .withf(is_expected_laptop)
        .returning(Ok);

    mock_message_publisher
        .expect_publish_created()
        .once()
        .withf(|event| is_expected_laptop(event.product()))
        .returning(|_| Ok(()));

    let use_case = CreateProduct::new(
        Arc::new(mock_product_repository),
        Arc::new(mock_category_repository),
        Arc::new(mock_message_publisher),
    );

    let product_dto = use_case.execute(create_laptop_command(category_id))?;
    assert_eq!("Laptop", product_dto.title());
    assert_eq!("A high-performance laptop", product_dto.description());
    assert_eq!(15, product_dto.quantity());
    assert_eq!("$30.50", product_dto.price().to_string());

    assert_eq!("Electronics", product_dto.category().title());
    assert_eq!(
        "Electronic devices and gadgets",
        product_dto.category().description(),
    );

    Ok(())
}

#[test]
fn test_create_product_execute_with_non_existent_category() {
    let mut mock_category_repository = MockCategoryRepository::new();
    let mut mock_product_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    let category_id = CategoryId::new();

    mock_category_repository
        .expect_find_by_id()
        .once()
        .with(predicate::eq(category_id))
        .returning(|_| Ok(None));

    mock_product_repository.expect_save().never();
    mock_message_publisher.expect_publish_created().never();

    let use_case = CreateProduct::new(
        Arc::new(mock_product_repository),
        Arc::new(mock_category_repository),
        Arc::new(mock_message_publisher),
    );

    match use_case.execute(create_laptop_command(category_id)) {
        Err(AppError::NotFound(message)) => assert_eq!(
            format!("Could not find category with id: {category_id}"),
            message
        ),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn test_create_product_execute_with_failed_save() {
    let category_id = CategoryId::new();
    let mock_category_repository = category_repository_returning(category_id);
    let mut mock_product_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    mock_product_repository
        .expect_save()
        .once()
        .withf(is_expected_laptop)
        .returning(move |_| Err(anyhow::anyhow!("Failed to save product to the repository")));

    mock_message_publisher.expect_publish_created().never();

    let use_case = CreateProduct::new(
        Arc::new(mock_product_repository),
        Arc::new(mock_category_repository),
        Arc::new(mock_message_publisher),
    );

    match use_case.execute(create_laptop_command(category_id)) {
        Err(AppError::Internal(error)) => {
            assert_eq!(
                "Failed to save product to the repository",
                error.to_string()
            )
        }
        other => panic!("expected Internal, got {other:?}"),
    }
}

/// The product is already committed when publishing fails: the error is still returned.
#[test]
fn test_create_product_execute_with_failed_publish() {
    let category_id = CategoryId::new();
    let mock_category_repository = category_repository_returning(category_id);
    let mut mock_product_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    mock_product_repository
        .expect_save()
        .once()
        .withf(is_expected_laptop)
        .returning(Ok);

    mock_message_publisher
        .expect_publish_created()
        .once()
        .withf(|event| is_expected_laptop(event.product()))
        .returning(move |_| Err(anyhow::anyhow!("Failed to publish product created event")));

    let use_case = CreateProduct::new(
        Arc::new(mock_product_repository),
        Arc::new(mock_category_repository),
        Arc::new(mock_message_publisher),
    );

    match use_case.execute(create_laptop_command(category_id)) {
        Err(AppError::Internal(error)) => {
            assert_eq!("Failed to publish product created event", error.to_string())
        }
        other => panic!("expected Internal, got {other:?}"),
    }
}

#[test]
fn test_update_product_execute() -> anyhow::Result<()> {
    let product_id = Uuid::new_v4();
    let category_id = CategoryId::new();
    let mock_category_repository = category_repository_returning(category_id);
    let mut mock_product_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    mock_product_repository
        .expect_save()
        .once()
        .withf(move |product| {
            product.id() == ProductId::from_uuid(product_id) && is_expected_laptop(product)
        })
        .returning(Ok);

    mock_message_publisher
        .expect_publish_updated()
        .once()
        .withf(move |event| {
            event.product().id() == ProductId::from_uuid(product_id)
                && is_expected_laptop(event.product())
        })
        .returning(|_| Ok(()));

    let command = UpdateProductCommand::new(
        product_id,
        "Laptop",
        "A high-performance laptop",
        15,
        dec!(30.5),
        *category_id.as_uuid(),
    );

    let use_case = UpdateProduct::new(
        Arc::new(mock_product_repository),
        Arc::new(mock_category_repository),
        Arc::new(mock_message_publisher),
    );

    let product_dto = use_case.execute(command)?;
    assert_eq!(product_id, product_dto.id());
    assert_eq!("Laptop", product_dto.title());
    assert_eq!("A high-performance laptop", product_dto.description());
    assert_eq!(15, product_dto.quantity());
    assert_eq!("$30.50", product_dto.price().to_string());

    assert_eq!("Electronics", product_dto.category().title());
    assert_eq!(
        "Electronic devices and gadgets",
        product_dto.category().description(),
    );

    Ok(())
}

#[test]
fn test_update_product_execute_with_non_existent_category() {
    let mut mock_category_repository = MockCategoryRepository::new();
    let mut mock_product_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    mock_category_repository
        .expect_find_by_id()
        .once()
        .returning(|_| Ok(None));
    mock_product_repository.expect_save().never();
    mock_message_publisher.expect_publish_updated().never();

    let command = UpdateProductCommand::new(
        Uuid::new_v4(),
        "Laptop",
        "A high-performance laptop",
        15,
        dec!(30.5),
        Uuid::new_v4(),
    );

    let result = UpdateProduct::new(
        Arc::new(mock_product_repository),
        Arc::new(mock_category_repository),
        Arc::new(mock_message_publisher),
    )
    .execute(command);

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[test]
fn test_delete_product_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    let product_id = Uuid::new_v4();

    mock_repository
        .expect_delete()
        .with(predicate::eq(ProductId::from_uuid(product_id)))
        .once()
        .returning(|_| Ok(()));

    mock_message_publisher
        .expect_publish_deleted()
        .once()
        .withf(move |event| event.product_id() == ProductId::from_uuid(product_id))
        .returning(|_| Ok(()));

    let use_case = DeleteProduct::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    use_case.execute(DeleteProductCommand::new(product_id))?;

    Ok(())
}

#[test]
fn test_delete_product_execute_with_failed_publish() {
    let mut mock_repository = MockProductRepository::new();
    let mut mock_message_publisher = MockProductMessagePublisher::new();

    mock_repository.expect_delete().once().returning(|_| Ok(()));
    mock_message_publisher
        .expect_publish_deleted()
        .once()
        .returning(|_| Err(anyhow::anyhow!("broker down")));

    let result = DeleteProduct::new(Arc::new(mock_repository), Arc::new(mock_message_publisher))
        .execute(DeleteProductCommand::new(Uuid::new_v4()));

    assert!(matches!(result, Err(AppError::Internal(_))));
}

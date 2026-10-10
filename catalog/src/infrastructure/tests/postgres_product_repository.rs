mod common;

use application::ports::out::repositories::CategoryRepository;
use application::ports::out::repositories::ProductRepository;
use domain::aggregates::category::Category;
use domain::aggregates::product::Product;
use infrastructure::postgres::PostgresCategoryRepository;
use infrastructure::postgres::PostgresProductRepository;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CategoryId, ProductId};
use uuid::Uuid;

struct TestContext {
    _container: common::PostgresContainer,
    category_repository: PostgresCategoryRepository,
    product_repository: PostgresProductRepository,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        category_repository: PostgresCategoryRepository::new(&db_pool),
        product_repository: PostgresProductRepository::new(&db_pool),
    })
}

#[tokio::test]
async fn test_list_all() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let first_category_id = CategoryId::new();
    let first_category = Category::new(
        first_category_id,
        "first category title".into(),
        "first category description".into(),
    );

    ctx.category_repository.save(first_category.clone())?;

    let first_product_id = ProductId::new();
    let first_product = Product::new(
        first_product_id,
        "first product title".to_string(),
        "first product description".to_string(),
        10,
        Money::from_str("25.5", iso::USD)?,
        first_category,
    );

    ctx.product_repository.save(first_product)?;

    let second_category_id = CategoryId::new();
    let second_category = Category::new(
        second_category_id,
        "second category title".into(),
        "second category description".into(),
    );

    ctx.category_repository.save(second_category.clone())?;

    let second_product_id = ProductId::new();
    let second_product = Product::new(
        second_product_id,
        "second product title".to_string(),
        "second product description".to_string(),
        10,
        Money::from_str("25.5", iso::USD)?,
        second_category,
    );

    ctx.product_repository.save(second_product)?;

    let products = ctx.product_repository.list_all()?;
    assert_eq!(2, products.len());

    assert_eq!("first product title", products[0].title());
    assert_eq!("first product description", products[0].description());
    assert_eq!("second product title", products[1].title());
    assert_eq!("second product description", products[1].description());

    Ok(())
}

#[tokio::test]
async fn test_create_product() -> anyhow::Result<()> {
    let ctx = setup_context().await?;
    let repository = &ctx.product_repository;

    let category_id = CategoryId::new();
    let category = Category::new(
        category_id,
        "category title".into(),
        "category description".into(),
    );

    ctx.category_repository.save(category.clone())?;

    let product_id = ProductId::new();
    let product = Product::new(
        product_id,
        "product title".to_string(),
        "product description".to_string(),
        10,
        Money::from_str("25.5", iso::USD)?,
        category,
    );

    repository.save(product)?;
    let saved_product = repository.find_by_id(product_id)?.expect("product exists");

    assert_ne!(
        Uuid::nil().to_string(),
        saved_product.id().as_uuid().to_string(),
    );
    assert_eq!("product title", saved_product.title());
    assert_eq!("product description", saved_product.description());
    assert_eq!(10, saved_product.quantity());
    assert_eq!("$25.50", saved_product.price().to_string());

    assert_eq!(category_id, saved_product.category().id());
    assert_eq!("category title", saved_product.category().title());
    assert_eq!(
        "category description",
        saved_product.category().description(),
    );

    Ok(())
}

#[tokio::test]
async fn test_update_product() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let category_id = CategoryId::new();
    let category = Category::new(
        category_id,
        "category title".into(),
        "category description".into(),
    );

    ctx.category_repository.save(category.clone())?;

    let product_id = ProductId::new();
    let product = Product::new(
        product_id,
        "product title".to_string(),
        "product description".to_string(),
        10,
        Money::from_str("25.5", iso::USD)?,
        category,
    );

    ctx.product_repository.save(product)?;

    let updated_product = Product::builder()
        .id(product_id)
        .title("updated product title")
        .description("updated product description")
        .quantity(20)
        .price(Money::from_str("30.0", iso::USD)?)
        .category(
            ctx.category_repository
                .find_by_id(category_id)?
                .expect("category exists"),
        )
        .build();
    ctx.product_repository.save(updated_product.clone())?;

    assert_ne!(
        Uuid::nil().to_string(),
        updated_product.id().as_uuid().to_string(),
    );
    assert_eq!("updated product title", updated_product.title());
    assert_eq!("updated product description", updated_product.description());
    assert_eq!(20, updated_product.quantity());
    assert_eq!("$30.00", updated_product.price().to_string());

    assert_eq!(category_id, updated_product.category().id());
    assert_eq!("category title", updated_product.category().title());
    assert_eq!(
        "category description",
        updated_product.category().description(),
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_product() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let category_id = CategoryId::new();
    let category = Category::new(
        category_id,
        "category title".into(),
        "category description".into(),
    );

    ctx.category_repository.save(category.clone())?;

    let product_id = ProductId::new();
    let product = Product::new(
        product_id,
        "product title".to_string(),
        "product description".to_string(),
        10,
        Money::from_str("25.5", iso::USD)?,
        category,
    );

    ctx.product_repository.save(product)?;
    ctx.product_repository.delete(product_id)?;

    assert!(ctx.product_repository.find_by_id(product_id)?.is_none());

    Ok(())
}

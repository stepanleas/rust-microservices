mod common;

use application::ports::out::repositories::ProductRepository;
use domain::aggregates::product::Product;
use infrastructure::postgres::PostgresProductRepository;
use rusty_money::iso;
use shared::domain::value_objects::ProductId;

struct TestContext {
    _container: common::PostgresContainer,
    repository: PostgresProductRepository,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        repository: PostgresProductRepository::new(&db_pool),
    })
}

#[tokio::test]
async fn test_create_product() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let id = ProductId::new();
    ctx.repository.save(Product::new(
        id,
        "Product 1".into(),
        2,
        rusty_money::Money::from_str("30.0", iso::USD)?,
    ))?;

    let saved_product = ctx.repository.find_by_id(id)?.expect("should be stored");
    assert_eq!(id, saved_product.id());
    assert_eq!("Product 1", saved_product.title());
    assert_eq!(2, saved_product.quantity());
    assert_eq!("$30.00", saved_product.price().to_string());

    Ok(())
}

#[tokio::test]
async fn test_update_product() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let id = ProductId::new();
    let product = Product::new(
        id,
        "Product 1".into(),
        2,
        rusty_money::Money::from_str("30.0", iso::USD)?,
    );
    ctx.repository.save(product)?;

    let updated_product = Product::builder()
        .id(id)
        .title("Updated Product".into())
        .quantity(5)
        .price(rusty_money::Money::from_str("50.0", iso::USD)?)
        .build();
    ctx.repository.save(updated_product)?;

    let saved_product = ctx.repository.find_by_id(id)?.expect("should be stored");
    assert_eq!(id, saved_product.id());
    assert_eq!("Updated Product", saved_product.title());
    assert_eq!(5, saved_product.quantity());
    assert_eq!("$50.00", saved_product.price().to_string());

    Ok(())
}

#[tokio::test]
async fn test_find_unknown_returns_none() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    assert!(ctx.repository.find_by_id(ProductId::new())?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_delete_product() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let id = ProductId::new();
    ctx.repository.save(Product::new(
        id,
        "Product 1".into(),
        2,
        rusty_money::Money::from_str("30.0", iso::USD)?,
    ))?;

    ctx.repository.delete(id)?;

    assert!(ctx.repository.find_by_id(id)?.is_none());

    Ok(())
}

mod common;

use application::ports::out::repositories::ProductRepository;
use domain::aggregates::product::Product;
use infrastructure::postgres::PostgresProductRepository;
use rust_decimal::dec;
use shared::domain::value_objects::ProductId;

#[tokio::test]
async fn test_product_repository_crud() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let repository = PostgresProductRepository::new(&pool);

    let first_product_id = ProductId::new();
    let second_product_id = ProductId::new();

    repository.save(&Product::new(
        first_product_id,
        "Pizza".into(),
        10,
        dec!(12.50),
    ))?;
    repository.save(&Product::new(
        second_product_id,
        "Burger".into(),
        3,
        dec!(8.00),
    ))?;

    let product = repository.find_by_id(first_product_id)?.unwrap();
    assert_eq!("Pizza", product.title());
    assert_eq!(10, product.quantity());
    assert_eq!(dec!(12.50), product.price());

    // Upsert keeps a single row and overwrites the values.
    repository.save(&Product::new(
        first_product_id,
        "Pizza XL".into(),
        7,
        dec!(15.00),
    ))?;
    let product = repository.find_by_id(first_product_id)?.unwrap();
    assert_eq!("Pizza XL", product.title());
    assert_eq!(7, product.quantity());

    let all = repository.find_all()?;
    assert_eq!(2, all.len());
    assert_eq!("Burger", all[0].title(), "products are ordered by title");

    let selected = repository.find_by_ids(vec![second_product_id, ProductId::new()])?;
    assert_eq!(1, selected.len());
    assert_eq!(second_product_id, selected[0].id());

    repository.delete(first_product_id)?;
    assert!(repository.find_by_id(first_product_id)?.is_none());

    Ok(())
}

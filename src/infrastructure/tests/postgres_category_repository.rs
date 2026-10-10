mod common;

use application::ports::out::repositories::CategoryRepository;
use domain::aggregates::category::Category;
use infrastructure::postgres::PostgresCategoryRepository;
use shared::domain::value_objects::CategoryId;

struct TestContext {
    _container: common::PostgresContainer,
    repository: PostgresCategoryRepository,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        repository: PostgresCategoryRepository::new(&db_pool),
    })
}

#[tokio::test]
async fn test_list_all() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let first_category = Category::new(
        CategoryId::new(),
        "category title 1".into(),
        "category description 1".into(),
    );
    let second_category = Category::new(
        CategoryId::new(),
        "category title 2".into(),
        "category description 2".into(),
    );
    ctx.repository.save(first_category)?;
    ctx.repository.save(second_category)?;

    let categories = ctx.repository.list_all()?;
    assert_eq!(2, categories.len());

    assert_eq!("category title 1", categories[0].title());
    assert_eq!("category description 1", categories[0].description());
    assert_eq!("category title 2", categories[1].title());
    assert_eq!("category description 2", categories[1].description());

    Ok(())
}

#[tokio::test]
async fn test_create_category() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let category_id = CategoryId::new();
    let category = Category::new(
        category_id,
        "category title".into(),
        "category description".into(),
    );
    ctx.repository.save(category)?;

    let saved_category = ctx
        .repository
        .find_by_id(category_id)?
        .expect("category exists");
    assert_eq!(category_id, saved_category.id());
    assert_eq!("category title", saved_category.title());
    assert_eq!("category description", saved_category.description());

    Ok(())
}

#[tokio::test]
async fn test_update_category() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let category_id = CategoryId::new();
    ctx.repository.save(Category::new(
        category_id,
        "category title".into(),
        "category description".into(),
    ))?;

    let updated_category = Category::builder()
        .id(category_id)
        .title("updated category title")
        .description("updated category description")
        .build();
    ctx.repository.save(updated_category)?;

    let saved_category = ctx
        .repository
        .find_by_id(category_id)?
        .expect("category exists");
    assert_eq!(category_id, saved_category.id());
    assert_eq!("updated category title", saved_category.title());
    assert_eq!("updated category description", saved_category.description());

    Ok(())
}

#[tokio::test]
async fn test_delete_category() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let category_id = CategoryId::new();
    let category = Category::new(
        category_id,
        "category title".into(),
        "category description".into(),
    );

    ctx.repository.save(category)?;
    ctx.repository.delete(category_id)?;

    assert!(ctx.repository.find_by_id(category_id)?.is_none());

    Ok(())
}

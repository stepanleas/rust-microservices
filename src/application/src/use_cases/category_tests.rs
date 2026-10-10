use crate::commands::{CreateCategoryCommand, DeleteCategoryCommand, UpdateCategoryCommand};
use crate::ports::out::repositories::MockCategoryRepository;
use crate::queries::FindCategoryQuery;
use crate::use_cases::{
    CreateCategory, DeleteCategory, FindCategory, ListCategories, UpdateCategory,
};
use domain::aggregates::category::Category;
use mockall::predicate;
use shared::domain::value_objects::CategoryId;
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_list_categories_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockCategoryRepository::new();

    let expected_categories = vec![
        Category::new(
            CategoryId::new(),
            "Books".to_string(),
            "A category for all book products".to_string(),
        ),
        Category::new(
            CategoryId::new(),
            "Electronics".to_string(),
            "A category for all electronic products".to_string(),
        ),
    ];

    mock_repository
        .expect_list_all()
        .once()
        .returning(move || Ok(expected_categories.clone()));

    let use_case = ListCategories::new(Arc::new(mock_repository));
    let category_dtos = use_case.execute()?;

    assert_eq!(2, category_dtos.len());
    assert_eq!("Books", category_dtos[0].title());
    assert_eq!(
        "A category for all book products",
        category_dtos[0].description(),
    );
    assert_eq!("Electronics", category_dtos[1].title());
    assert_eq!(
        "A category for all electronic products",
        category_dtos[1].description(),
    );

    Ok(())
}

#[test]
fn test_list_categories_repository_failure_is_internal() {
    let mut mock_repository = MockCategoryRepository::new();

    mock_repository
        .expect_list_all()
        .once()
        .returning(|| Err(anyhow::anyhow!("db down")));

    let result = ListCategories::new(Arc::new(mock_repository)).execute();

    assert!(matches!(result, Err(AppError::Internal(_))));
}

#[test]
fn test_find_category_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockCategoryRepository::new();

    let category_id = CategoryId::from_uuid(Uuid::new_v4());

    mock_repository
        .expect_find_by_id()
        .with(predicate::eq(category_id))
        .once()
        .returning(move |_| {
            Ok(Some(Category::new(
                category_id,
                "Books".to_string(),
                "A category for all book products".to_string(),
            )))
        });

    let use_case = FindCategory::new(Arc::new(mock_repository));

    let category_dto = use_case.execute(FindCategoryQuery::new(*category_id.as_uuid()))?;

    assert_eq!("Books", category_dto.title());
    assert_eq!(
        "A category for all book products",
        category_dto.description(),
    );

    Ok(())
}

#[test]
fn test_find_category_not_found() {
    let mut mock_repository = MockCategoryRepository::new();

    let category_id = Uuid::new_v4();

    mock_repository
        .expect_find_by_id()
        .once()
        .returning(|_| Ok(None));

    let result =
        FindCategory::new(Arc::new(mock_repository)).execute(FindCategoryQuery::new(category_id));

    match result {
        Err(AppError::NotFound(message)) => assert_eq!(
            format!("Could not find category with id: {category_id}"),
            message
        ),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn test_create_category_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockCategoryRepository::new();

    mock_repository
        .expect_save()
        .once()
        .withf(move |category: &Category| {
            category.title() == "Books"
                && category.description() == "A category for all book products"
        })
        .returning(|category| Ok(category.clone()));

    let use_case = CreateCategory::new(Arc::new(mock_repository));

    let command = CreateCategoryCommand::new("Books", "A category for all book products");
    let category_dto = use_case.execute(command)?;

    assert_eq!("Books", category_dto.title());
    assert_eq!(
        "A category for all book products",
        category_dto.description(),
    );

    Ok(())
}

#[test]
fn test_update_category_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockCategoryRepository::new();

    let category_id = Uuid::new_v4();

    mock_repository
        .expect_save()
        .once()
        .withf(move |category: &Category| {
            category.id() == CategoryId::from_uuid(category_id)
                && category.title() == "Books"
                && category.description() == "A category for all book products"
        })
        .returning(|category| Ok(category.clone()));

    let use_case = UpdateCategory::new(Arc::new(mock_repository));

    let command =
        UpdateCategoryCommand::new(category_id, "Books", "A category for all book products");
    let category_dto = use_case.execute(command)?;

    assert_eq!(category_id, category_dto.id());
    assert_eq!("Books", category_dto.title());
    assert_eq!(
        "A category for all book products",
        category_dto.description(),
    );

    Ok(())
}

#[test]
fn test_delete_category_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockCategoryRepository::new();

    let category_id = Uuid::new_v4();
    let command = DeleteCategoryCommand::new(category_id);

    mock_repository
        .expect_delete()
        .with(predicate::eq(CategoryId::from_uuid(category_id)))
        .once()
        .returning(|_| Ok(()));

    let use_case = DeleteCategory::new(Arc::new(mock_repository));

    use_case.execute(command)?;

    Ok(())
}

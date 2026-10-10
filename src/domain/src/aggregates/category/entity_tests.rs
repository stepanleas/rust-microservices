use crate::aggregates::category::Category;
use shared::domain::value_objects::CategoryId;

#[test]
fn test_new_category() {
    let category_id = CategoryId::new();
    let category = Category::new(
        category_id,
        "Category title".into(),
        "Category description".into(),
    );

    assert_eq!(category_id, category.id());
    assert_eq!("Category title", category.title());
    assert_eq!("Category description", category.description());
}

#[test]
fn test_category_builder() {
    let category_id = CategoryId::new();
    let category = Category::builder()
        .id(category_id)
        .title("Books")
        .description("All books")
        .build();

    assert_eq!(category_id, category.id());
    assert_eq!("Books", category.title());
    assert_eq!("All books", category.description());
}

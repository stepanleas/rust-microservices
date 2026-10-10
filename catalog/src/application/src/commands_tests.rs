use crate::commands::{
    CreateCategoryCommand, CreateProductCommand, DeleteCategoryCommand, DeleteProductCommand,
    UpdateCategoryCommand, UpdateProductCommand,
};
use rust_decimal::dec;
use uuid::Uuid;

#[test]
fn test_create_category_command() {
    let command = CreateCategoryCommand::new("Books", "A category for all book products");

    assert_eq!("Books", command.title());
    assert_eq!("A category for all book products", command.description());
}

#[test]
fn test_update_category_command() {
    let category_id = Uuid::new_v4();
    let command =
        UpdateCategoryCommand::new(category_id, "Books", "A category for all book products");

    assert_eq!(category_id, command.id());
    assert_eq!("Books", command.title());
    assert_eq!("A category for all book products", command.description());
}

#[test]
fn test_delete_category_command() {
    let category_id = Uuid::new_v4();
    let command = DeleteCategoryCommand::new(category_id);

    assert_eq!(command.id(), category_id);
}

#[test]
fn test_create_product_command() {
    let category_id = Uuid::new_v4();
    let command = CreateProductCommand::new(
        "Test Product",
        "A product for testing",
        10,
        dec!(99.99),
        category_id,
    );

    assert_eq!("Test Product", command.title());
    assert_eq!("A product for testing", command.description());
    assert_eq!(10, command.quantity());
    assert_eq!(dec!(99.99), command.price());
    assert_eq!(category_id, command.category_id());
}

#[test]
fn test_update_product_command() {
    let product_id = Uuid::new_v4();
    let category_id = Uuid::new_v4();
    let command = UpdateProductCommand::new(
        product_id,
        "Test Product",
        "A product for testing",
        10,
        dec!(99.99),
        category_id,
    );

    assert_eq!(product_id, command.id());
    assert_eq!("Test Product", command.title());
    assert_eq!("A product for testing", command.description());
    assert_eq!(10, command.quantity());
    assert_eq!(dec!(99.99), command.price());
    assert_eq!(category_id, command.category_id());
}

#[test]
fn test_delete_product_command() {
    let product_id = Uuid::new_v4();
    let command = DeleteProductCommand::new(product_id);

    assert_eq!(command.id(), product_id);
}

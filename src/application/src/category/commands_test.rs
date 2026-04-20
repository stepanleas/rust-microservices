#[cfg(test)]
mod tests {
    use crate::category::commands::{
        CreateCategoryCommand, DeleteCategoryCommand, UpdateCategoryCommand,
    };

    #[test]
    fn test_create_category_command() {
        let command = CreateCategoryCommand::new("Books", "A category for all book products");

        assert_eq!("Books", command.title());
        assert_eq!("A category for all book products", command.description());
    }

    #[test]
    fn test_update_category_command() {
        let category_id = uuid::Uuid::new_v4();
        let command =
            UpdateCategoryCommand::new(category_id, "Books", "A category for all book products");

        assert_eq!(category_id, command.id());
        assert_eq!("Books", command.title());
        assert_eq!("A category for all book products", command.description());
    }

    #[test]
    fn test_delete_category_command() {
        let category_id = uuid::Uuid::new_v4();
        let command = DeleteCategoryCommand::new(category_id);

        assert_eq!(command.id(), category_id);
    }
}

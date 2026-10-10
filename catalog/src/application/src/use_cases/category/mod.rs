mod create_category;
mod delete_category;
mod find_category;
mod list_categories;
mod update_category;

pub use create_category::CreateCategory;
pub use delete_category::DeleteCategory;
pub use find_category::FindCategory;
pub use list_categories::ListCategories;
pub use update_category::UpdateCategory;

use shared::domain::value_objects::CategoryId;
use shared::error::AppError;

pub(crate) fn category_not_found(id: CategoryId) -> AppError {
    AppError::not_found(format!("Could not find category with id: {id}"))
}

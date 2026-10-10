mod create_product;
mod delete_product;
mod find_product;
mod list_products;
mod update_product;

pub use create_product::CreateProduct;
pub use delete_product::DeleteProduct;
pub use find_product::FindProduct;
pub use list_products::ListProducts;
pub use update_product::UpdateProduct;

use crate::ports::out::repositories::CategoryRepository;
use crate::use_cases::category::category_not_found;
use domain::aggregates::category::Category;
use shared::domain::value_objects::CategoryId;
use shared::error::AppResult;
use uuid::Uuid;

/// Loads the category a product command refers to. A missing category answers 404, as the
/// API documents for product create/update.
fn load_category(repository: &dyn CategoryRepository, category_id: Uuid) -> AppResult<Category> {
    let category_id = CategoryId::from_uuid(category_id);

    repository
        .find_by_id(category_id)?
        .ok_or_else(|| category_not_found(category_id))
}

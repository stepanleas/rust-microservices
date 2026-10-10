use domain::aggregates::category::Category;
use domain::aggregates::product::Product;
use shared::domain::value_objects::{CategoryId, ProductId};

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait CategoryRepository: Send + Sync {
    fn list_all(&self) -> anyhow::Result<Vec<Category>>;
    fn find_by_id(&self, entity_id: CategoryId) -> anyhow::Result<Option<Category>>;
    fn save(&self, entity: Category) -> anyhow::Result<Category>;
    fn delete(&self, entity_id: CategoryId) -> anyhow::Result<()>;
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait ProductRepository: Send + Sync {
    fn list_all(&self) -> anyhow::Result<Vec<Product>>;
    fn find_by_id(&self, entity_id: ProductId) -> anyhow::Result<Option<Product>>;
    fn save(&self, entity: Product) -> anyhow::Result<Product>;
    fn delete(&self, entity_id: ProductId) -> anyhow::Result<()>;
}

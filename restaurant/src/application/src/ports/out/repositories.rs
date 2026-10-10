use crate::outbox::RestaurantOutboxMessage;
use domain::aggregates::order_approval::OrderApproval;
use domain::aggregates::product::Product;
use shared::domain::value_objects::{OrderId, ProductId};

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait ProductRepository: Send + Sync {
    fn find_all(&self) -> anyhow::Result<Vec<Product>>;
    fn find_by_id(&self, product_id: ProductId) -> anyhow::Result<Option<Product>>;
    /// Ids that do not exist are left out of the result.
    fn find_by_ids(&self, product_ids: Vec<ProductId>) -> anyhow::Result<Vec<Product>>;
    /// Inserts or replaces the product.
    fn save(&self, product: &Product) -> anyhow::Result<()>;
    fn delete(&self, product_id: ProductId) -> anyhow::Result<()>;
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait OrderApprovalRepository: Send + Sync {
    fn find_by_order_id(&self, order_id: OrderId) -> anyhow::Result<Option<OrderApproval>>;

    /// Persists the approval, the updated product stock and the outbox message
    /// in a single database transaction (transactional outbox).
    fn save_with_outbox(
        &self,
        approval: &OrderApproval,
        updated_products: &[Product],
        message: &RestaurantOutboxMessage,
    ) -> anyhow::Result<()>;

    /// Stores a cancelled approval together with the restocked products in a single database
    /// transaction. Fails, changing nothing, when the stored approval is no longer `APPROVED`
    /// (it was cancelled concurrently), so stock is never given back twice.
    fn save_cancellation(
        &self,
        approval: &OrderApproval,
        restocked_products: &[Product],
    ) -> anyhow::Result<()>;
}

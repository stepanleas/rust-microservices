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

/// A change of one product's stock: `quantity` units taken (negative) or given back (positive).
///
/// The repositories apply changes relative to the stored stock, not as the absolute quantity a
/// use case read earlier: replicas approving orders for the same product at the same time would
/// otherwise overwrite each other's decrease (a lost update).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StockChange {
    pub product_id: ProductId,
    pub quantity: i32,
}

impl StockChange {
    /// The changes from `before` (id and quantity as read) to `after` (the same products once
    /// the domain changed them); unchanged products are left out.
    pub fn between(before: &[(ProductId, i32)], after: &[Product]) -> Vec<Self> {
        after
            .iter()
            .filter_map(|product| {
                let read = before
                    .iter()
                    .find(|(id, _)| *id == product.id())
                    .map_or(product.quantity(), |(_, quantity)| *quantity);
                let quantity = product.quantity() - read;
                (quantity != 0).then_some(Self {
                    product_id: product.id(),
                    quantity,
                })
            })
            .collect()
    }

    /// Id and quantity of each product, to compare with afterwards ([`Self::between`]).
    pub fn snapshot(products: &[Product]) -> Vec<(ProductId, i32)> {
        products
            .iter()
            .map(|product| (product.id(), product.quantity()))
            .collect()
    }
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait OrderApprovalRepository: Send + Sync {
    fn find_by_order_id(&self, order_id: OrderId) -> anyhow::Result<Option<OrderApproval>>;

    /// Persists the approval, the stock it takes and the outbox message in a single database
    /// transaction (transactional outbox). Fails, changing nothing, when a product no longer has
    /// the stock to take (another order took it meanwhile); the message is then retried.
    fn save_with_outbox(
        &self,
        approval: &OrderApproval,
        stock_changes: &[StockChange],
        message: &RestaurantOutboxMessage,
    ) -> anyhow::Result<()>;

    /// Stores a cancelled approval together with the stock it gives back in a single database
    /// transaction. Fails, changing nothing, when the stored approval is no longer `APPROVED`
    /// (it was cancelled concurrently), so stock is never given back twice.
    fn save_cancellation(
        &self,
        approval: &OrderApproval,
        stock_changes: &[StockChange],
    ) -> anyhow::Result<()>;
}

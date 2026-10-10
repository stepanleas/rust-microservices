use domain::aggregates::customer::Customer;
use domain::aggregates::order::Order;
use domain::aggregates::product::Product;
use shared::domain::value_objects::{CustomerId, OrderId, ProductId};
use uuid::Uuid;

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait OrderRepository: Send + Sync {
    fn find_by_id(&self, order_id: OrderId) -> anyhow::Result<Option<Order>>;
    fn find_by_tracking_id(&self, tracking_id: Uuid) -> anyhow::Result<Option<Order>>;
    fn save(&self, entity: &Order) -> anyhow::Result<()>;
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait CustomerRepository: Send + Sync {
    fn find_by_id(&self, customer_id: CustomerId) -> anyhow::Result<Option<Customer>>;
    fn save(&self, entity: Customer) -> anyhow::Result<()>;
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait ProductRepository: Send + Sync {
    /// Returns the products that exist; unknown ids are simply missing from the result.
    fn find_by_ids(&self, product_ids: Vec<ProductId>) -> anyhow::Result<Vec<Product>>;
    fn find_by_id(&self, product_id: ProductId) -> anyhow::Result<Option<Product>>;
    fn save(&self, product: Product) -> anyhow::Result<()>;
    fn delete(&self, product_id: ProductId) -> anyhow::Result<()>;
}

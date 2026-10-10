use crate::saga::dtos::{PaymentResponse, RestaurantApprovalResponse};
use domain::aggregates::customer::Customer;
use domain::aggregates::product::Product;
use shared::domain::value_objects::ProductId;
use shared::error::AppResult;

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait CustomerMessageListener: Send + Sync {
    fn customer_created(&self, customer: Customer) -> AppResult<()>;
    fn customer_updated(&self, customer: Customer) -> AppResult<()>;
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait ProductMessageListener: Send + Sync {
    fn product_created(&self, product: Product) -> AppResult<()>;
    fn product_updated(&self, product: Product) -> AppResult<()>;
    fn product_deleted(&self, product_id: ProductId) -> AppResult<()>;
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait RestaurantApprovalResponseMessageListener: Send + Sync {
    fn order_approved(&self, response: RestaurantApprovalResponse) -> AppResult<()>;
    fn order_rejected(&self, response: RestaurantApprovalResponse) -> AppResult<()>;
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait PaymentResponseMessageListener: Send + Sync {
    fn payment_completed(&self, response: PaymentResponse) -> AppResult<()>;
    fn payment_failed(&self, response: PaymentResponse) -> AppResult<()>;
}

//! Kafka topic names. Consumers park messages they cannot handle in `<topic>.dlq`.

pub const PRODUCT_CREATED: &str = "product-created";
pub const PRODUCT_UPDATED: &str = "product-updated";
pub const PRODUCT_DELETED: &str = "product-deleted";

pub const CUSTOMER_CREATED: &str = "customer-created";
pub const CUSTOMER_UPDATED: &str = "customer-updated";

pub const PAYMENT_CREATED: &str = "payment-created";
pub const PAYMENT_UPDATED: &str = "payment-updated";

pub const RESTAURANT_APPROVAL_REQUEST: &str = "restaurant-approval-request";
pub const RESTAURANT_APPROVAL_RESPONSE: &str = "restaurant-approval-response";
/// Compensation: the order was cancelled after the restaurant approved it.
pub const RESTAURANT_APPROVAL_CANCEL_REQUEST: &str = "restaurant-approval-cancel-request";
pub const PAYMENT_REQUEST: &str = "payment-request";
pub const PAYMENT_RESPONSE: &str = "payment-response";

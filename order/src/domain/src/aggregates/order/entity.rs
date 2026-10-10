use crate::aggregates::order::OrderBuilder;
use crate::aggregates::order::OrderItem;
use crate::enums::OrderStatus;
use crate::error::DomainError;
use rusty_money::iso::Currency;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId};
use uuid::Uuid;

pub struct Order {
    id: OrderId,
    customer_id: CustomerId,
    tracking_id: Uuid,
    price: Money<'static, Currency>,
    items: Vec<OrderItem>,
    status: OrderStatus,
    failure_messages: Vec<String>,
}

impl Order {
    pub fn builder() -> OrderBuilder {
        OrderBuilder::default()
    }

    pub fn new(id: OrderId, customer_id: CustomerId) -> Self {
        Order {
            id,
            customer_id,
            tracking_id: Uuid::new_v4(),
            price: Money::from_minor(0, iso::USD),
            items: vec![],
            status: OrderStatus::Pending,
            failure_messages: vec![],
        }
    }

    /// Rebuilds an order from persisted state without running any lifecycle checks.
    pub fn restore(
        id: OrderId,
        customer_id: CustomerId,
        tracking_id: Uuid,
        price: Money<'static, Currency>,
        items: Vec<OrderItem>,
        status: OrderStatus,
        failure_messages: Vec<String>,
    ) -> Self {
        Order {
            id,
            customer_id,
            tracking_id,
            price,
            items,
            status,
            failure_messages,
        }
    }

    pub fn id(&self) -> OrderId {
        self.id
    }

    pub fn customer_id(&self) -> CustomerId {
        self.customer_id
    }

    pub fn tracking_id(&self) -> Uuid {
        self.tracking_id
    }

    pub fn price(&self) -> &Money<'static, Currency> {
        &self.price
    }

    pub fn set_price(&mut self, price: Money<'static, Currency>) {
        self.price = price;
    }

    pub fn items(&self) -> &[OrderItem] {
        &self.items
    }

    pub fn set_items(&mut self, items: Vec<OrderItem>) {
        self.items = items;
    }

    pub fn status(&self) -> OrderStatus {
        self.status
    }

    pub fn failure_messages(&self) -> &[String] {
        &self.failure_messages
    }

    /// The restaurant accepted the order: `Pending -> Approved`.
    pub fn approve(&mut self) -> Result<(), DomainError> {
        if self.status != OrderStatus::Pending {
            return Err(DomainError::OrderDomainError {
                message: format!(
                    "Order is not in correct state for approve operation: {:?}!",
                    self.status,
                ),
            });
        }

        self.status = OrderStatus::Approved;
        Ok(())
    }

    /// The payment for an approved order completed: `Approved -> Paid`.
    pub fn pay(&mut self) -> Result<(), DomainError> {
        if self.status != OrderStatus::Approved {
            return Err(DomainError::OrderDomainError {
                message: format!(
                    "Order is not in correct state for pay operation: {:?}!",
                    self.status,
                ),
            });
        }

        self.status = OrderStatus::Paid;
        Ok(())
    }

    /// The restaurant rejected the order or the payment failed: `Pending | Approved -> Canceled`.
    pub fn cancel(&mut self, failure_messages: Vec<String>) -> Result<(), DomainError> {
        if !matches!(self.status, OrderStatus::Pending | OrderStatus::Approved) {
            return Err(DomainError::OrderDomainError {
                message: format!(
                    "Order is not in correct state for cancel operation: {:?}!",
                    self.status,
                ),
            });
        }

        self.status = OrderStatus::Canceled;
        self.failure_messages.extend(
            failure_messages
                .into_iter()
                .filter(|message| !message.is_empty()),
        );
        Ok(())
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.status != OrderStatus::Pending {
            return Err(DomainError::OrderDomainError {
                message: "Order is not in correct state for initialization!".to_string(),
            });
        }

        if self.price.is_zero() {
            return Err(DomainError::OrderDomainError {
                message: "Total price must be greater than zero!".to_string(),
            });
        }

        self.validate_items_price()
    }

    fn validate_items_price(&self) -> Result<(), DomainError> {
        let order_items_total =
            self.items
                .iter()
                .try_fold(Money::from_minor(0, iso::USD), |acc, item| {
                    if !item.is_price_valid() {
                        return Err(DomainError::OrderDomainError {
                            message: "Order item price is not valid!".to_string(),
                        });
                    }

                    Ok(acc
                        .add(*item.sub_total())
                        .unwrap_or(Money::from_minor(0, iso::USD)))
                })?;

        if self.price != order_items_total {
            return Err(DomainError::OrderDomainError {
                message: format!(
                    "Total price: {} is not equal to order items total: {}!",
                    self.price, order_items_total,
                ),
            });
        }

        Ok(())
    }
}

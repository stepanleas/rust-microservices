use crate::aggregates::order_approval::OrderApproval;
use crate::aggregates::product::Product;
use shared::domain::value_objects::{OrderId, ProductId};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RequestedProduct {
    product_id: ProductId,
    quantity: i32,
}

impl RequestedProduct {
    pub fn new(product_id: ProductId, quantity: i32) -> Self {
        Self {
            product_id,
            quantity,
        }
    }

    pub fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }
}

/// Decides whether the restaurant can fulfil an order.
pub struct RestaurantApprovalDomainService;

impl RestaurantApprovalDomainService {
    /// Validates the requested products against the local stock.
    ///
    /// When the order is approved, the stock of every requested product is
    /// decreased in `products`. When it is rejected, `products` is left untouched
    /// and the returned approval carries the reasons.
    pub fn approve_order(
        saga_id: Uuid,
        order_id: OrderId,
        requested: &[RequestedProduct],
        products: &mut [Product],
    ) -> OrderApproval {
        let failure_messages = Self::validate(requested, products);

        if !failure_messages.is_empty() {
            return OrderApproval::rejected(saga_id, order_id, failure_messages);
        }

        for (product_id, quantity) in Self::aggregate(requested) {
            if let Some(product) = products
                .iter_mut()
                .find(|product| *product.id().as_uuid() == product_id)
            {
                // Validated above, cannot fail.
                let _ = product.decrease_stock(quantity);
            }
        }

        OrderApproval::approved(saga_id, order_id)
    }

    fn validate(requested: &[RequestedProduct], products: &[Product]) -> Vec<String> {
        if requested.is_empty() {
            return vec!["Order must contain at least one product".to_string()];
        }

        let mut failure_messages = vec![];

        for (product_id, quantity) in Self::aggregate(requested) {
            if quantity <= 0 {
                failure_messages.push(format!(
                    "Invalid quantity {quantity} for product {product_id}"
                ));
                continue;
            }

            match products
                .iter()
                .find(|product| *product.id().as_uuid() == product_id)
            {
                None => failure_messages.push(format!(
                    "Product {product_id} is not available in the restaurant"
                )),
                Some(product) if !product.has_stock_for(quantity) => {
                    failure_messages.push(format!(
                        "Insufficient stock for product {}: requested {}, available {}",
                        product.title(),
                        quantity,
                        product.quantity(),
                    ))
                }
                Some(_) => {}
            }
        }

        failure_messages
    }

    /// Sums quantities of duplicated product lines, keeping request order.
    fn aggregate(requested: &[RequestedProduct]) -> Vec<(Uuid, i32)> {
        let mut order: Vec<Uuid> = vec![];
        let mut totals: HashMap<Uuid, i32> = HashMap::new();

        for item in requested {
            let product_id = *item.product_id().as_uuid();
            if !totals.contains_key(&product_id) {
                order.push(product_id);
            }
            *totals.entry(product_id).or_insert(0) += item.quantity();
        }

        order
            .into_iter()
            .map(|product_id| (product_id, totals[&product_id]))
            .collect()
    }
}

use domain::enums::PaymentStatus;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug)]
pub struct CreatePaymentCommand {
    order_id: Uuid,
    customer_id: Uuid,
    price: Decimal,
}

impl CreatePaymentCommand {
    pub fn new(order_id: Uuid, customer_id: Uuid, price: Decimal) -> Self {
        Self {
            order_id,
            customer_id,
            price,
        }
    }

    pub fn order_id(&self) -> Uuid {
        self.order_id
    }

    pub fn customer_id(&self) -> Uuid {
        self.customer_id
    }

    pub fn price(&self) -> Decimal {
        self.price
    }
}

#[derive(Debug)]
pub struct UpdatePaymentCommand {
    id: Uuid,
    order_id: Uuid,
    customer_id: Uuid,
    price: Decimal,
    payment_status: PaymentStatus,
}

impl UpdatePaymentCommand {
    pub fn new(
        id: Uuid,
        order_id: Uuid,
        customer_id: Uuid,
        price: Decimal,
        payment_status: PaymentStatus,
    ) -> Self {
        Self {
            id,
            order_id,
            customer_id,
            price,
            payment_status,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn order_id(&self) -> Uuid {
        self.order_id
    }

    pub fn customer_id(&self) -> Uuid {
        self.customer_id
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn payment_status(&self) -> PaymentStatus {
        self.payment_status
    }
}

/// Issued when the order saga asks this service to charge the customer for an order.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessPaymentCommand {
    saga_id: Uuid,
    order_id: Uuid,
    customer_id: Uuid,
    price: Decimal,
}

impl ProcessPaymentCommand {
    pub fn new(saga_id: Uuid, order_id: Uuid, customer_id: Uuid, price: Decimal) -> Self {
        Self {
            saga_id,
            order_id,
            customer_id,
            price,
        }
    }

    pub fn saga_id(&self) -> Uuid {
        self.saga_id
    }

    pub fn order_id(&self) -> Uuid {
        self.order_id
    }

    pub fn customer_id(&self) -> Uuid {
        self.customer_id
    }

    pub fn price(&self) -> Decimal {
        self.price
    }
}

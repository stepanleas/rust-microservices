use uuid::Uuid;

pub struct FindPaymentByIdQuery {
    id: Uuid,
}

impl FindPaymentByIdQuery {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

pub struct FindPaymentByOrderIdQuery {
    order_id: Uuid,
}

impl FindPaymentByOrderIdQuery {
    pub fn new(order_id: Uuid) -> Self {
        Self { order_id }
    }

    pub fn order_id(&self) -> Uuid {
        self.order_id
    }
}

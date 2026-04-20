use rust_decimal::Decimal;
use uuid::Uuid;

pub struct CreateProductCommand<'a> {
    title: &'a str,
    description: &'a str,
    quantity: i32,
    price: Decimal,
    category_id: Uuid,
}

impl<'a> CreateProductCommand<'a> {
    pub fn new(
        title: &'a str,
        description: &'a str,
        quantity: i32,
        price: Decimal,
        category_id: Uuid,
    ) -> Self {
        CreateProductCommand {
            title,
            description,
            quantity,
            price,
            category_id,
        }
    }

    pub fn title(&self) -> &str {
        self.title
    }

    pub fn description(&self) -> &str {
        self.description
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn category_id(&self) -> Uuid {
        self.category_id
    }
}

pub struct UpdateProductCommand<'a> {
    id: Uuid,
    title: &'a str,
    description: &'a str,
    quantity: i32,
    price: Decimal,
    category_id: Uuid,
}

impl<'a> UpdateProductCommand<'a> {
    pub fn new(
        id: Uuid,
        title: &'a str,
        description: &'a str,
        quantity: i32,
        price: Decimal,
        category_id: Uuid,
    ) -> Self {
        UpdateProductCommand {
            id,
            title,
            description,
            quantity,
            price,
            category_id,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        self.title
    }

    pub fn description(&self) -> &str {
        self.description
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn category_id(&self) -> Uuid {
        self.category_id
    }
}

pub struct DeleteProductCommand {
    id: Uuid,
}

impl DeleteProductCommand {
    pub fn new(id: Uuid) -> Self {
        DeleteProductCommand { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

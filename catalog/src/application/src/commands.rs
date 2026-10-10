use rust_decimal::Decimal;
use uuid::Uuid;

pub struct CreateCategoryCommand {
    title: String,
    description: String,
}

impl CreateCategoryCommand {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

pub struct UpdateCategoryCommand {
    id: Uuid,
    title: String,
    description: String,
}

impl UpdateCategoryCommand {
    pub fn new(id: Uuid, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            description: description.into(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

pub struct DeleteCategoryCommand {
    id: Uuid,
}

impl DeleteCategoryCommand {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

pub struct CreateProductCommand {
    title: String,
    description: String,
    quantity: i32,
    price: Decimal,
    category_id: Uuid,
}

impl CreateProductCommand {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        quantity: i32,
        price: Decimal,
        category_id: Uuid,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            quantity,
            price,
            category_id,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
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

pub struct UpdateProductCommand {
    id: Uuid,
    title: String,
    description: String,
    quantity: i32,
    price: Decimal,
    category_id: Uuid,
}

impl UpdateProductCommand {
    pub fn new(
        id: Uuid,
        title: impl Into<String>,
        description: impl Into<String>,
        quantity: i32,
        price: Decimal,
        category_id: Uuid,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            description: description.into(),
            quantity,
            price,
            category_id,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
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
        Self { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

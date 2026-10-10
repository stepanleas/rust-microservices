use uuid::Uuid;

pub struct FindCategoryQuery {
    id: Uuid,
}

impl FindCategoryQuery {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

pub struct FindProductQuery {
    id: Uuid,
}

impl FindProductQuery {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

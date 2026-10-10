use crate::aggregates::customer::CustomerBuilder;
use shared::domain::value_objects::CustomerId;

#[derive(Clone)]
pub struct Customer {
    id: CustomerId,
    user_name: String,
    first_name: String,
    last_name: String,
}

impl Customer {
    pub fn builder() -> CustomerBuilder {
        CustomerBuilder::default()
    }

    pub fn new(id: CustomerId, user_name: String, first_name: String, last_name: String) -> Self {
        Self {
            id,
            user_name,
            first_name,
            last_name,
        }
    }

    pub fn id(&self) -> CustomerId {
        self.id
    }

    pub fn user_name(&self) -> &str {
        &self.user_name
    }

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }
}

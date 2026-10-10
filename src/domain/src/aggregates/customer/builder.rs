use crate::aggregates::customer::Customer;
use shared::domain::value_objects::CustomerId;

#[derive(Default)]
pub struct CustomerBuilder {
    id: CustomerId,
    user_name: String,
    first_name: String,
    last_name: String,
}

impl CustomerBuilder {
    pub fn id(mut self, id: CustomerId) -> Self {
        self.id = id;
        self
    }

    pub fn user_name(mut self, user_name: String) -> Self {
        self.user_name = user_name;
        self
    }

    pub fn first_name(mut self, first_name: String) -> Self {
        self.first_name = first_name;
        self
    }

    pub fn last_name(mut self, last_name: String) -> Self {
        self.last_name = last_name;
        self
    }

    pub fn build(self) -> Customer {
        Customer::new(self.id, self.user_name, self.first_name, self.last_name)
    }
}

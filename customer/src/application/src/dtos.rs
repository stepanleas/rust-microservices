use domain::aggregates::customer::Customer;
use uuid::Uuid;

pub struct CustomerDto {
    id: Uuid,
    user_name: String,
    first_name: String,
    last_name: String,
}

impl CustomerDto {
    pub fn id(&self) -> Uuid {
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

impl From<&Customer> for CustomerDto {
    fn from(customer: &Customer) -> Self {
        Self {
            id: customer.id().into(),
            user_name: customer.user_name().into(),
            first_name: customer.first_name().into(),
            last_name: customer.last_name().into(),
        }
    }
}

impl From<Customer> for CustomerDto {
    fn from(customer: Customer) -> Self {
        Self {
            id: customer.id().into(),
            user_name: customer.user_name().into(),
            first_name: customer.first_name().into(),
            last_name: customer.last_name().into(),
        }
    }
}

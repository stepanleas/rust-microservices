use uuid::Uuid;

pub struct CreateCustomerCommand {
    user_name: String,
    first_name: String,
    last_name: String,
}

impl CreateCustomerCommand {
    pub fn new(user_name: String, first_name: String, last_name: String) -> Self {
        Self {
            user_name,
            first_name,
            last_name,
        }
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

pub struct UpdateCustomerCommand {
    id: Uuid,
    user_name: String,
    first_name: String,
    last_name: String,
}

impl UpdateCustomerCommand {
    pub fn new(id: Uuid, user_name: String, first_name: String, last_name: String) -> Self {
        Self {
            id,
            user_name,
            first_name,
            last_name,
        }
    }

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

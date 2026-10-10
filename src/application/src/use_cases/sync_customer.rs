use crate::ports::r#in::messages::CustomerMessageListener;
use crate::ports::out::repositories::CustomerRepository;
use domain::aggregates::customer::Customer;
use shared::error::AppResult;
use std::sync::Arc;

/// Keeps the local copy of the customers (owned by the customer service) up to date.
/// Saving is an upsert, so redelivered events are harmless.
pub struct SyncCustomer {
    repository: Arc<dyn CustomerRepository>,
}

impl SyncCustomer {
    pub fn new(repository: Arc<dyn CustomerRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, customer: Customer) -> AppResult<()> {
        let customer_id = customer.id();
        self.repository.save(customer)?;
        tracing::info!("Customer {} saved in order database", customer_id);

        Ok(())
    }
}

impl CustomerMessageListener for SyncCustomer {
    fn customer_created(&self, customer: Customer) -> AppResult<()> {
        self.execute(customer)
    }

    fn customer_updated(&self, customer: Customer) -> AppResult<()> {
        self.execute(customer)
    }
}

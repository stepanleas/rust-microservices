use domain::aggregates::customer::Customer;

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait CustomerRepository: Send + Sync {
    /// Inserts the customer, or updates it when a customer with the same id exists.
    fn save(&self, customer: Customer) -> anyhow::Result<Customer>;
}

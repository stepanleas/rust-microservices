mod create_customer;
mod update_customer;

#[cfg(test)]
mod customer_tests;

pub use create_customer::CreateCustomer;
pub use update_customer::UpdateCustomer;

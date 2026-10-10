use crate::ports::r#in::messages::CustomerMessageListener;
use crate::ports::out::repositories::MockCustomerRepository;
use crate::use_cases::SyncCustomer;
use domain::aggregates::customer::Customer;
use shared::domain::value_objects::CustomerId;
use shared::error::AppError;
use std::sync::Arc;

fn customer(customer_id: CustomerId) -> Customer {
    Customer::new(customer_id, "Artellas".into(), "Mike".into(), "Dane".into())
}

#[test]
fn test_created_and_updated_customers_are_saved() -> anyhow::Result<()> {
    let customer_id = CustomerId::new();
    let mut repository = MockCustomerRepository::new();
    repository
        .expect_save()
        .times(2)
        .withf(move |customer| customer.id() == customer_id)
        .returning(|_| Ok(()));

    let use_case = SyncCustomer::new(Arc::new(repository));
    use_case.customer_created(customer(customer_id))?;
    use_case.customer_updated(customer(customer_id))?;

    Ok(())
}

#[test]
fn test_save_error_is_propagated() {
    let mut repository = MockCustomerRepository::new();
    repository
        .expect_save()
        .returning(|_| Err(anyhow::anyhow!("database is down")));

    let result =
        SyncCustomer::new(Arc::new(repository)).customer_created(customer(CustomerId::new()));

    assert!(matches!(result, Err(AppError::Internal(_))));
}

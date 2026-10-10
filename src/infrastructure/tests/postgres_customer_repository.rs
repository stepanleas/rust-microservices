mod common;

use application::ports::out::repositories::CustomerRepository;
use domain::aggregates::customer::Customer;
use infrastructure::postgres::PostgresCustomerRepository;
use shared::domain::value_objects::CustomerId;

struct TestContext {
    _container: common::PostgresContainer,
    repository: PostgresCustomerRepository,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;
    let repository = PostgresCustomerRepository::new(&db_pool);

    Ok(TestContext {
        _container: container,
        repository,
    })
}

#[tokio::test]
async fn test_create_customer() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let id = CustomerId::new();
    let customer = Customer::new(id, "Artellas".into(), "Mike".into(), "Dane".into());
    ctx.repository.save(customer)?;

    let saved_customer = ctx.repository.find_by_id(id)?.expect("should be stored");
    assert_eq!(id, saved_customer.id());
    assert_eq!("Artellas", saved_customer.user_name());
    assert_eq!("Mike", saved_customer.first_name());
    assert_eq!("Dane", saved_customer.last_name());

    Ok(())
}

#[tokio::test]
async fn test_update_customer() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let id = CustomerId::new();
    let customer = Customer::new(id, "Artellas".into(), "Mike".into(), "Dane".into());
    ctx.repository.save(customer)?;

    let updated_customer = Customer::builder()
        .id(id)
        .user_name("Updated username".into())
        .first_name("Updated first name".into())
        .last_name("Updated last name".into())
        .build();
    ctx.repository.save(updated_customer)?;

    let saved_customer = ctx.repository.find_by_id(id)?.expect("should be stored");
    assert_eq!(saved_customer.id(), id);
    assert_eq!(saved_customer.user_name(), "Updated username");
    assert_eq!(saved_customer.first_name(), "Updated first name");
    assert_eq!(saved_customer.last_name(), "Updated last name");

    Ok(())
}

#[tokio::test]
async fn test_find_unknown_returns_none() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    assert!(ctx.repository.find_by_id(CustomerId::new())?.is_none());

    Ok(())
}

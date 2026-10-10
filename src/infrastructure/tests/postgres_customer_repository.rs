mod common;

use application::ports::out::repositories::CustomerRepository;
use diesel::ExpressionMethods;
use diesel::{OptionalExtension, QueryDsl, RunQueryDsl};
use domain::aggregates::customer::Customer;
use infrastructure::DbPool;
use infrastructure::postgres::{CustomerEntity, PostgresCustomerRepository};
use infrastructure::schema::customers::dsl::customers;
use infrastructure::schema::customers::id;
use shared::domain::value_objects::CustomerId;

struct TestContext {
    _container: common::PostgresContainer,
    repository: PostgresCustomerRepository,
    pool: DbPool,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        repository: PostgresCustomerRepository::new(&db_pool),
        pool: db_pool,
    })
}

fn load_customer(pool: &DbPool, customer_id: CustomerId) -> anyhow::Result<Customer> {
    let mut connection = pool.get()?;

    Ok(customers
        .filter(id.eq(customer_id.as_uuid()))
        .first::<CustomerEntity>(&mut connection)
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Could not find customer with id: {customer_id}"))?
        .into())
}

#[tokio::test]
async fn test_create_customer() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let customer_id = CustomerId::new();
    let first_customer = Customer::new(
        customer_id,
        "user name".into(),
        "first name".into(),
        "last name".into(),
    );
    ctx.repository.save(first_customer)?;

    let saved_customer = load_customer(&ctx.pool, customer_id)?;

    assert_eq!(customer_id, saved_customer.id());
    assert_eq!("user name", saved_customer.user_name());
    assert_eq!("first name", saved_customer.first_name());
    assert_eq!("last name", saved_customer.last_name());

    Ok(())
}

#[tokio::test]
async fn test_save_existing_customer_updates_it() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let customer_id = CustomerId::new();
    ctx.repository.save(Customer::new(
        customer_id,
        "user name".into(),
        "first name".into(),
        "last name".into(),
    ))?;
    ctx.repository.save(Customer::new(
        customer_id,
        "new user name".into(),
        "new first name".into(),
        "new last name".into(),
    ))?;

    let saved_customer = load_customer(&ctx.pool, customer_id)?;
    let count: i64 = customers.count().get_result(&mut ctx.pool.get()?)?;

    assert_eq!(1, count);
    assert_eq!("new user name", saved_customer.user_name());
    assert_eq!("new first name", saved_customer.first_name());
    assert_eq!("new last name", saved_customer.last_name());

    Ok(())
}

mod common;

use application::ports::out::repositories::{
    CustomerRepository, OrderRepository, ProductRepository,
};
use domain::aggregates::customer::Customer;
use domain::aggregates::order::{Order, OrderItem};
use domain::aggregates::product::Product;
use infrastructure::postgres::PostgresCustomerRepository;
use infrastructure::postgres::PostgresOrderRepository;
use infrastructure::postgres::PostgresProductRepository;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use uuid::Uuid;

struct TestContext {
    _container: common::PostgresContainer,
    product_repository: PostgresProductRepository,
    customer_repository: PostgresCustomerRepository,
    order_repository: PostgresOrderRepository,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        product_repository: PostgresProductRepository::new(&db_pool),
        customer_repository: PostgresCustomerRepository::new(&db_pool),
        order_repository: PostgresOrderRepository::new(&db_pool),
    })
}

#[tokio::test]
async fn test_create_order() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let order_id = OrderId::new();
    let customer_id = CustomerId::new();

    let first_product_id = ProductId::new();
    let second_product_id = ProductId::new();
    let first_order_item_id = OrderItemId::new();
    let second_order_item_id = OrderItemId::new();

    ctx.product_repository.save(Product::new(
        first_product_id,
        "Product 1".into(),
        2,
        Money::from_str("30.0", iso::USD)?,
    ))?;
    ctx.product_repository.save(Product::new(
        second_product_id,
        "Product 2".into(),
        5,
        Money::from_str("30.0", iso::USD)?,
    ))?;

    ctx.customer_repository.save(Customer::new(
        customer_id,
        "Artellas".into(),
        "Mike".into(),
        "Dane".into(),
    ))?;

    let order = Order::builder()
        .id(order_id)
        .customer_id(customer_id)
        .items(vec![
            OrderItem::new(
                first_order_item_id,
                order_id,
                ProductId::new(),
                2,
                Money::from_str("30.0", iso::USD)?,
                Money::from_str("60.0", iso::USD)?,
            ),
            OrderItem::new(
                second_order_item_id,
                order_id,
                ProductId::new(),
                3,
                Money::from_str("30.0", iso::USD)?,
                Money::from_str("90.0", iso::USD)?,
            ),
        ])
        .price(Money::from_str("150.0", iso::USD)?)
        .build();

    ctx.order_repository.save(&order)?;

    let saved_order = ctx
        .order_repository
        .find_by_tracking_id(order.tracking_id())?
        .expect("should be stored");
    assert_eq!(order_id, saved_order.id());
    assert_eq!(customer_id, saved_order.customer_id());
    assert_eq!(2, saved_order.items().len());

    assert_eq!(first_order_item_id, saved_order.items()[0].id());
    assert_eq!(order_id, saved_order.items()[0].order_id());
    assert_ne!(
        Uuid::nil().to_string(),
        saved_order.items()[0].product_id().as_uuid().to_string(),
    );
    assert_eq!(2, saved_order.items()[0].quantity());
    assert_eq!("$30.00", saved_order.items()[0].price().to_string());
    assert_eq!("$60.00", saved_order.items()[0].sub_total().to_string());

    assert_eq!(second_order_item_id, saved_order.items()[1].id());
    assert_eq!(order_id, saved_order.items()[1].order_id());
    assert_ne!(
        Uuid::nil().to_string(),
        saved_order.items()[1].product_id().as_uuid().to_string(),
    );
    assert_eq!(3, saved_order.items()[1].quantity());
    assert_eq!("$30.00", saved_order.items()[1].price().to_string());
    assert_eq!("$90.00", saved_order.items()[1].sub_total().to_string());

    assert_eq!("$150.00", saved_order.price().to_string());

    Ok(())
}

#[tokio::test]
async fn test_find_unknown_order_returns_none() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    assert!(ctx.order_repository.find_by_id(OrderId::new())?.is_none());
    assert!(
        ctx.order_repository
            .find_by_tracking_id(Uuid::new_v4())?
            .is_none()
    );

    Ok(())
}

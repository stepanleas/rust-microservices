use crate::commands::{CreateCustomerCommand, UpdateCustomerCommand};
use crate::ports::out::publishers::MockCustomerMessagePublisher;
use crate::ports::out::repositories::MockCustomerRepository;
use crate::use_cases::{CreateCustomer, UpdateCustomer};
use shared::domain::value_objects::CustomerId;
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

fn create_command() -> CreateCustomerCommand {
    CreateCustomerCommand::new(
        "Artellas".to_string(),
        "John".to_string(),
        "Doe".to_string(),
    )
}

#[test]
fn test_create_customer_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockCustomerRepository::new();
    let mut mock_message_publisher = MockCustomerMessagePublisher::new();

    mock_repository
        .expect_save()
        .once()
        .withf(move |customer| {
            customer.first_name() == "John"
                && customer.last_name() == "Doe"
                && customer.user_name() == "Artellas"
        })
        .returning(Ok);

    mock_message_publisher
        .expect_publish_created()
        .once()
        .withf(move |event| {
            event.customer().first_name() == "John"
                && event.customer().last_name() == "Doe"
                && event.customer().user_name() == "Artellas"
        })
        .returning(|_| Ok(()));

    let use_case = CreateCustomer::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let result = use_case.execute(create_command())?;

    assert_eq!(result.first_name(), "John");
    assert_eq!(result.last_name(), "Doe");
    assert_eq!(result.user_name(), "Artellas");

    Ok(())
}

#[test]
fn test_update_customer_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockCustomerRepository::new();
    let mut mock_message_publisher = MockCustomerMessagePublisher::new();

    let customer_id = Uuid::new_v4();

    mock_repository
        .expect_save()
        .once()
        .withf(move |customer| {
            customer.id() == CustomerId::from_uuid(customer_id)
                && customer.first_name() == "John"
                && customer.last_name() == "Doe"
                && customer.user_name() == "Artellas"
        })
        .returning(Ok);

    mock_message_publisher
        .expect_publish_updated()
        .once()
        .withf(move |event| {
            event.customer().id() == CustomerId::from_uuid(customer_id)
                && event.customer().first_name() == "John"
                && event.customer().last_name() == "Doe"
                && event.customer().user_name() == "Artellas"
        })
        .returning(|_| Ok(()));

    let command = UpdateCustomerCommand::new(
        customer_id,
        "Artellas".to_string(),
        "John".to_string(),
        "Doe".to_string(),
    );

    let use_case = UpdateCustomer::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let result = use_case.execute(command)?;

    assert_eq!(result.id(), customer_id);
    assert_eq!(result.first_name(), "John");
    assert_eq!(result.last_name(), "Doe");
    assert_eq!(result.user_name(), "Artellas");

    Ok(())
}

#[test]
fn test_create_customer_repository_failure_is_internal_and_nothing_is_published() {
    let mut mock_repository = MockCustomerRepository::new();
    let mut mock_message_publisher = MockCustomerMessagePublisher::new();

    mock_repository
        .expect_save()
        .once()
        .returning(|_| Err(anyhow::anyhow!("database down")));
    mock_message_publisher.expect_publish_created().never();

    let use_case = CreateCustomer::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let result = use_case.execute(create_command());

    assert!(matches!(result, Err(AppError::Internal(_))));
}

/// No outbox yet: the customer stays saved when publishing fails, and the caller gets an error.
#[test]
fn test_create_customer_publish_failure_after_save_is_internal() {
    let mut mock_repository = MockCustomerRepository::new();
    let mut mock_message_publisher = MockCustomerMessagePublisher::new();

    mock_repository.expect_save().once().returning(Ok);
    mock_message_publisher
        .expect_publish_created()
        .once()
        .returning(|_| Err(anyhow::anyhow!("broker unreachable")));

    let use_case = CreateCustomer::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let result = use_case.execute(create_command());

    assert!(matches!(result, Err(AppError::Internal(_))));
}

#[test]
fn test_update_customer_publish_failure_after_save_is_internal() {
    let mut mock_repository = MockCustomerRepository::new();
    let mut mock_message_publisher = MockCustomerMessagePublisher::new();

    mock_repository.expect_save().once().returning(Ok);
    mock_message_publisher
        .expect_publish_updated()
        .once()
        .returning(|_| Err(anyhow::anyhow!("broker unreachable")));

    let use_case = UpdateCustomer::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let command = UpdateCustomerCommand::new(
        Uuid::new_v4(),
        "Artellas".to_string(),
        "John".to_string(),
        "Doe".to_string(),
    );

    assert!(matches!(
        use_case.execute(command),
        Err(AppError::Internal(_))
    ));
}

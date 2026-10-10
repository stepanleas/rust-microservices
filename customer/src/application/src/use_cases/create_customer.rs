use crate::commands::CreateCustomerCommand;
use crate::dtos::CustomerDto;
use crate::ports::out::publishers::CustomerMessagePublisher;
use crate::ports::out::repositories::CustomerRepository;
use domain::aggregates::customer::Customer;
use domain::events::CustomerCreatedEvent;
use shared::domain::value_objects::CustomerId;
use shared::error::AppResult;
use std::sync::Arc;

pub struct CreateCustomer {
    repository: Arc<dyn CustomerRepository>,
    message_publisher: Arc<dyn CustomerMessagePublisher>,
}

impl CreateCustomer {
    pub fn new(
        repository: Arc<dyn CustomerRepository>,
        message_publisher: Arc<dyn CustomerMessagePublisher>,
    ) -> Self {
        Self {
            repository,
            message_publisher,
        }
    }

    /// Saves the customer, then publishes `customer-created`.
    ///
    /// If publishing fails the customer stays saved and the error is returned (HTTP 500).
    // TODO: Add transactional outbox
    pub fn execute(&self, command: CreateCustomerCommand) -> AppResult<CustomerDto> {
        let customer = Customer::builder()
            .id(CustomerId::new())
            .user_name(command.user_name().into())
            .first_name(command.first_name().into())
            .last_name(command.last_name().into())
            .build();

        let saved_customer = self.repository.save(customer)?;
        tracing::info!("Customer with id: {} created", saved_customer.id());

        let customer_dto = CustomerDto::from(&saved_customer);
        self.message_publisher
            .publish_created(CustomerCreatedEvent::new(saved_customer))?;

        Ok(customer_dto)
    }
}

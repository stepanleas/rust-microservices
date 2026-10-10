use crate::commands::{CreateCustomerCommand, UpdateCustomerCommand};
use uuid::Uuid;

#[test]
fn test_create_customer_command() {
    let user_name = "Artellas".to_string();
    let first_name = "John".to_string();
    let last_name = "Doe".to_string();

    let command =
        CreateCustomerCommand::new(user_name.clone(), first_name.clone(), last_name.clone());

    assert_eq!(command.user_name(), user_name);
    assert_eq!(command.first_name(), first_name);
    assert_eq!(command.last_name(), last_name);
}

#[test]
fn test_update_customer_command() {
    let customer_id = Uuid::new_v4();
    let user_name = "Artellas".to_string();
    let first_name = "John".to_string();
    let last_name = "Doe".to_string();

    let command = UpdateCustomerCommand::new(
        customer_id,
        user_name.clone(),
        first_name.clone(),
        last_name.clone(),
    );

    assert_eq!(command.id(), customer_id);
    assert_eq!(command.user_name(), user_name);
    assert_eq!(command.first_name(), first_name);
    assert_eq!(command.last_name(), last_name);
}

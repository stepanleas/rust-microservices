use crate::error::DomainError;
use shared::error::AppError;

#[test]
fn test_domain_errors_map_to_app_errors() {
    let business_rule = AppError::from(DomainError::OrderDomainError {
        message: "Total price must be greater than zero!".into(),
    });
    assert!(
        matches!(business_rule, AppError::BusinessRule(ref message) if message == "Total price must be greater than zero!")
    );

    let conflict = AppError::from(DomainError::SagaError {
        message: "Saga cannot move".into(),
    });
    assert!(matches!(conflict, AppError::Conflict(ref message) if message == "Saga cannot move"));
}

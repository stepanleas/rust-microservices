use crate::mappers::{ProductMessagingMapper, RestaurantApprovalMessagingMapper};
use rust_decimal::dec;
use shared::contracts::catalog::ProductAvroModel;
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalProductAvroModel,
    RestaurantApprovalRequestAvroModel,
};
use uuid::Uuid;

fn request(price: &str, product_id: &str) -> RestaurantApprovalRequestAvroModel {
    RestaurantApprovalRequestAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: Uuid::new_v4().to_string(),
        order_id: Uuid::new_v4().to_string(),
        customer_id: Uuid::new_v4().to_string(),
        price: price.into(),
        products: vec![RestaurantApprovalProductAvroModel {
            id: product_id.into(),
            quantity: 2,
        }],
        created_at: "2025-01-01T12:00:00Z".into(),
    }
}

fn cancel_request(product_id: &str) -> RestaurantApprovalCancelRequestAvroModel {
    RestaurantApprovalCancelRequestAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: Uuid::new_v4().to_string(),
        order_id: Uuid::new_v4().to_string(),
        products: vec![RestaurantApprovalProductAvroModel {
            id: product_id.into(),
            quantity: 3,
        }],
        created_at: "2025-01-01T12:00:00Z".into(),
    }
}

#[test]
fn test_map_product_avro_model_to_dto() -> anyhow::Result<()> {
    let product_id = Uuid::new_v4();
    let product = ProductMessagingMapper::product_avro_model_to_dto(&ProductAvroModel {
        id: product_id.to_string(),
        title: "Pizza".into(),
        quantity: 4,
        price: "19.99".into(),
    })?;

    assert_eq!(product_id, product.id);
    assert_eq!("Pizza", product.title);
    assert_eq!(4, product.quantity);
    assert_eq!(dec!(19.99), product.price);

    Ok(())
}

#[test]
fn test_map_product_avro_model_with_invalid_price() {
    let result = ProductMessagingMapper::product_avro_model_to_dto(&ProductAvroModel {
        id: Uuid::new_v4().to_string(),
        title: "Pizza".into(),
        quantity: 4,
        price: "abc".into(),
    });

    assert!(
        result
            .unwrap_err()
            .to_string()
            .starts_with("Failed to convert price 'abc' to Decimal")
    );
}

#[test]
fn test_map_approval_request_to_command() -> anyhow::Result<()> {
    let product_id = Uuid::new_v4();
    let avro_model = request("150.00", &product_id.to_string());

    let command = RestaurantApprovalMessagingMapper::approval_request_to_command(&avro_model)?;

    assert_eq!(avro_model.saga_id, command.saga_id().to_string());
    assert_eq!(avro_model.order_id, command.order_id().to_string());
    assert_eq!(avro_model.customer_id, command.customer_id().to_string());
    assert_eq!(dec!(150.00), command.price());
    assert_eq!(product_id, command.products()[0].product_id());
    assert_eq!(2, command.products()[0].quantity());

    Ok(())
}

#[test]
fn test_map_approval_request_with_invalid_values() {
    let invalid_price = request("abc", &Uuid::new_v4().to_string());
    assert_eq!(
        "Invalid price 'abc'",
        RestaurantApprovalMessagingMapper::approval_request_to_command(&invalid_price)
            .unwrap_err()
            .to_string(),
    );

    let invalid_product = request("1.00", "product-1");
    assert_eq!(
        "Invalid product id 'product-1'",
        RestaurantApprovalMessagingMapper::approval_request_to_command(&invalid_product)
            .unwrap_err()
            .to_string(),
    );
}

#[test]
fn test_map_approval_cancel_request_to_command() -> anyhow::Result<()> {
    let product_id = Uuid::new_v4();
    let avro_model = cancel_request(&product_id.to_string());

    let command =
        RestaurantApprovalMessagingMapper::approval_cancel_request_to_command(&avro_model)?;

    assert_eq!(avro_model.saga_id, command.saga_id().to_string());
    assert_eq!(avro_model.order_id, command.order_id().to_string());
    assert_eq!(1, command.products().len());
    assert_eq!(product_id, command.products()[0].product_id());
    assert_eq!(3, command.products()[0].quantity());

    Ok(())
}

#[test]
fn test_map_approval_cancel_request_with_invalid_product_id() {
    assert_eq!(
        "Invalid product id 'product-1'",
        RestaurantApprovalMessagingMapper::approval_cancel_request_to_command(&cancel_request(
            "product-1"
        ))
        .unwrap_err()
        .to_string(),
    );
}

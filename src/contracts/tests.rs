use super::AvroMessage;
use super::catalog::*;
use super::customer::*;
use super::payment::*;
use super::restaurant::*;

fn round_trip<T: AvroMessage + PartialEq + std::fmt::Debug>(message: T) {
    let bytes = message.to_avro_bytes().expect("encode");
    let decoded = T::from_avro_bytes(&bytes).expect("decode");
    assert_eq!(vec![message], decoded);
}

fn product() -> ProductAvroModel {
    ProductAvroModel {
        id: "p-1".into(),
        title: "Margherita".into(),
        quantity: 3,
        price: "12.50".into(),
    }
}

#[test]
fn every_schema_parses() {
    for schema in [
        ProductCreatedEventAvroModel::SCHEMA,
        ProductUpdatedEventAvroModel::SCHEMA,
        ProductDeletedEventAvroModel::SCHEMA,
        CustomerCreatedEventAvroModel::SCHEMA,
        CustomerUpdatedEventAvroModel::SCHEMA,
        PaymentRequestAvroModel::SCHEMA,
        PaymentResponseAvroModel::SCHEMA,
        PaymentCreatedEventAvroModel::SCHEMA,
        PaymentUpdatedEventAvroModel::SCHEMA,
        RestaurantApprovalRequestAvroModel::SCHEMA,
        RestaurantApprovalResponseAvroModel::SCHEMA,
        RestaurantApprovalCancelRequestAvroModel::SCHEMA,
    ] {
        apache_avro::Schema::parse_str(schema).expect(schema);
    }
}

#[test]
fn catalog_messages_round_trip() {
    round_trip(ProductCreatedEventAvroModel {
        id: "e-1".into(),
        product: product(),
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(ProductUpdatedEventAvroModel {
        id: "e-2".into(),
        product: product(),
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(ProductDeletedEventAvroModel {
        id: "e-3".into(),
        product_id: "p-1".into(),
        created_at: "2026-10-09T10:00:00Z".into(),
    });
}

#[test]
fn customer_messages_round_trip() {
    let customer = CustomerAvroModel {
        id: "c-1".into(),
        user_name: "jdoe".into(),
        first_name: "Jane".into(),
        last_name: "Doe".into(),
    };
    round_trip(CustomerCreatedEventAvroModel {
        id: "e-1".into(),
        customer: customer.clone(),
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(CustomerUpdatedEventAvroModel {
        id: "e-2".into(),
        customer,
        created_at: "2026-10-09T10:00:00Z".into(),
    });
}

#[test]
fn saga_messages_round_trip() {
    let products = vec![RestaurantApprovalProductAvroModel {
        id: "p-1".into(),
        quantity: 2,
    }];
    round_trip(RestaurantApprovalRequestAvroModel {
        id: "m-1".into(),
        saga_id: "s-1".into(),
        order_id: "o-1".into(),
        customer_id: "c-1".into(),
        price: "25.00".into(),
        products: products.clone(),
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(RestaurantApprovalResponseAvroModel {
        id: "m-2".into(),
        saga_id: "s-1".into(),
        order_id: "o-1".into(),
        order_approval_status: "APPROVED".into(),
        failure_messages: vec![],
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(RestaurantApprovalCancelRequestAvroModel {
        id: "m-3".into(),
        saga_id: "s-1".into(),
        order_id: "o-1".into(),
        products,
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(PaymentRequestAvroModel {
        id: "m-4".into(),
        saga_id: "s-1".into(),
        order_id: "o-1".into(),
        customer_id: "c-1".into(),
        price: "25.00".into(),
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(PaymentResponseAvroModel {
        id: "m-5".into(),
        saga_id: "s-1".into(),
        payment_id: "pay-1".into(),
        order_id: "o-1".into(),
        customer_id: "c-1".into(),
        price: "25.00".into(),
        payment_status: "FAILED".into(),
        failure_messages: vec!["insufficient funds".into()],
        created_at: "2026-10-09T10:00:00Z".into(),
    });
}

#[test]
fn payment_events_round_trip() {
    let payment = PaymentAvroModel {
        id: "pay-1".into(),
        order_id: "o-1".into(),
        customer_id: "c-1".into(),
        price: "25.00".into(),
        payment_status: "COMPLETED".into(),
    };
    round_trip(PaymentCreatedEventAvroModel {
        id: "e-1".into(),
        payment: payment.clone(),
        created_at: "2026-10-09T10:00:00Z".into(),
    });
    round_trip(PaymentUpdatedEventAvroModel {
        id: "e-2".into(),
        payment,
        created_at: "2026-10-09T10:00:00Z".into(),
    });
}

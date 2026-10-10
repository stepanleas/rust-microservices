use crate::publishers::encode;
use application::outbox::restaurant_approval_response;
use domain::aggregates::order_approval::OrderApproval;
use shared::contracts::AvroMessage;
use shared::contracts::restaurant::RestaurantApprovalResponseAvroModel;
use shared::contracts::topics;
use shared::domain::value_objects::OrderId;
use uuid::Uuid;

#[test]
fn test_encode_restaurant_approval_response() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let approval = OrderApproval::rejected(saga_id, order_id, vec!["Out of stock".into()]);
    let message = restaurant_approval_response(&approval)?;

    let record = encode(&message)?;

    assert_eq!(topics::RESTAURANT_APPROVAL_RESPONSE, record.topic);
    assert_eq!(order_id.as_uuid().to_string(), record.key);

    let decoded = RestaurantApprovalResponseAvroModel::from_avro_bytes(&record.payload)?;
    assert_eq!(1, decoded.len());
    assert_eq!(message.id().to_string(), decoded[0].id);
    assert_eq!(saga_id.to_string(), decoded[0].saga_id);
    assert_eq!(order_id.as_uuid().to_string(), decoded[0].order_id);
    assert_eq!("REJECTED", decoded[0].order_approval_status);
    assert_eq!(
        vec!["Out of stock".to_string()],
        decoded[0].failure_messages
    );

    Ok(())
}

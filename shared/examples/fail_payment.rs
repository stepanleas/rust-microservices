//! Test helper for the end-to-end saga script: answers the payment request of one order with a
//! FAILED payment response, as if the payment service had declined it.
//!
//! The payment service only fails non-positive amounts, which the order API never accepts, so the
//! "payment failed after the restaurant approved" path cannot be reached over HTTP. Stop the
//! payment service, place an order, then run:
//!
//!   cargo run --manifest-path shared/Cargo.toml --example fail_payment \
//!     --features contracts,kafka,outbox -- <order-id> [kafka-host] [timeout-secs]
use shared::contracts::AvroMessage;
use shared::contracts::payment::{PaymentRequestAvroModel, PaymentResponseAvroModel};
use shared::contracts::topics;
use shared::kafka::{KafkaConsumer, KafkaProducer};
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let order_id = args.next().ok_or_else(|| {
        anyhow::anyhow!("usage: fail_payment <order-id> [kafka-host] [timeout-secs]")
    })?;
    let host = args.next().unwrap_or_else(|| "localhost:9092".to_string());
    let timeout = Duration::from_secs(args.next().map_or(Ok(60), |t| t.parse())?);

    let request = find_payment_request(&host, &order_id, timeout)?;
    let response = PaymentResponseAvroModel {
        id: uuid::Uuid::new_v4().to_string(),
        saga_id: request.saga_id,
        payment_id: uuid::Uuid::new_v4().to_string(),
        order_id: request.order_id.clone(),
        customer_id: request.customer_id,
        price: request.price,
        payment_status: "FAILED".to_string(),
        failure_messages: vec!["Payment declined by the e2e test".to_string()],
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut producer = KafkaProducer::new(vec![host]);
    producer.send(
        topics::PAYMENT_RESPONSE,
        &request.order_id,
        &response.to_avro_bytes()?,
    )?;
    println!(
        "Sent FAILED payment response for order {}",
        request.order_id
    );
    Ok(())
}

/// Reads `payment-request` from the beginning (fresh consumer group) until the order shows up.
fn find_payment_request(
    host: &str,
    order_id: &str,
    timeout: Duration,
) -> anyhow::Result<PaymentRequestAvroModel> {
    let group = format!("e2e-fail-payment-{}", uuid::Uuid::new_v4());
    let consumer = KafkaConsumer::new(vec![host.to_string()], group, &[topics::PAYMENT_REQUEST])?;
    let started = Instant::now();

    while started.elapsed() < timeout {
        let Some(message) = consumer.poll(Duration::from_millis(500))? else {
            continue;
        };
        for request in PaymentRequestAvroModel::from_avro_bytes(&message.payload)? {
            if request.order_id == order_id {
                return Ok(request);
            }
        }
    }
    anyhow::bail!("No payment request for order {order_id} within {timeout:?}")
}

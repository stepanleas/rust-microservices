use application::dtos::PaymentDto;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AppInfoResponse {
    /// Environment the service runs in.
    #[schema(example = "development")]
    environment: String,
}

impl AppInfoResponse {
    pub fn new(environment: String) -> Self {
        Self { environment }
    }
}

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PaymentResponse {
    /// Payment id.
    #[schema(example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")]
    pub id: Uuid,
    /// Order the payment belongs to (unique).
    #[schema(example = "0411a4a9-1edb-4180-9556-800cb2b84721")]
    pub order_id: Uuid,
    /// Customer that paid.
    #[schema(example = "eb0163b3-cf88-4053-8c96-986b4f5b9f2e")]
    pub customer_id: Uuid,
    /// Charged amount.
    #[schema(example = 150.0)]
    pub price: f64,
    /// One of `COMPLETED`, `FAILED`, `CANCELLED`.
    #[schema(example = "COMPLETED")]
    pub payment_status: String,
    /// Reasons why the payment failed (empty when completed).
    #[schema(example = json!([]))]
    pub failure_messages: Vec<String>,
}

impl From<PaymentDto> for PaymentResponse {
    fn from(dto: PaymentDto) -> Self {
        Self {
            id: dto.id(),
            order_id: dto.order_id(),
            customer_id: dto.customer_id(),
            price: dto.price().to_string().parse::<f64>().unwrap_or(0.0),
            payment_status: dto.payment_status_as_str().to_string(),
            failure_messages: dto.failure_messages().to_vec(),
        }
    }
}

/// `{ "data": PaymentResponse }` envelope.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PaymentEnvelope {
    pub data: PaymentResponse,
}

/// `{ "data": AppInfoResponse }` envelope.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AppInfoEnvelope {
    pub data: AppInfoResponse,
}

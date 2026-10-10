use application::dtos::CustomerDto;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Envelope used by every successful response: `{ "data": ... }`.
#[derive(Serialize, ToSchema)]
pub struct DataResponse<T> {
    pub data: T,
}

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"environment": "development"}))]
pub struct AppInfoResponse {
    environment: String,
}

impl AppInfoResponse {
    pub fn new(environment: String) -> Self {
        Self { environment }
    }
}

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"id": "0411a4a9-1edb-4180-9556-800cb2b84721", "user_name": "artellas", "first_name": "Mike", "last_name": "Dane"}))]
pub struct CustomerResponse {
    id: Uuid,
    user_name: String,
    first_name: String,
    last_name: String,
}

impl From<CustomerDto> for CustomerResponse {
    fn from(customer: CustomerDto) -> Self {
        Self {
            id: customer.id(),
            user_name: customer.user_name().into(),
            first_name: customer.first_name().into(),
            last_name: customer.last_name().into(),
        }
    }
}

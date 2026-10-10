use crate::api::api_payment::__path_create;
use crate::api::api_payment::__path_get;
use crate::api::api_payment::__path_get_by_order_id;
use crate::api::api_payment::__path_update;
use crate::dto::requests::{CreatePaymentRequest, UpdatePaymentRequest};
use crate::dto::responses::{PaymentEnvelope, PaymentResponse};
use shared::web::{ErrorResponse, ValidationFieldError};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Payments", description = "Payment management endpoints. Saga payments are created from the `payment-request` Kafka topic and answered on `payment-response`.")
    ),
    paths(
        get,
        get_by_order_id,
        create,
        update,
    ),
    components(schemas(
        CreatePaymentRequest,
        UpdatePaymentRequest,
        PaymentResponse,
        PaymentEnvelope,
        ErrorResponse,
        ValidationFieldError,
    ))
)]
pub(crate) struct PaymentApiDoc;

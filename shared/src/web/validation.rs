use super::ApiError;
use actix_web::dev::Payload;
use actix_web::web::Json;
use actix_web::{FromRequest, HttpRequest};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use utoipa::ToSchema;
use validator::Validate;

/// A single field validation failure.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ValidationFieldError {
    #[schema(example = "price")]
    pub field: String,
    #[schema(example = "Price must be greater than 0")]
    pub message: String,
}

/// JSON body extractor that also runs the `validator` rules, answering 400 with the field
/// errors when they fail.
#[derive(Debug)]
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for ValidatedJson<T> {
    fn from(value: T) -> Self {
        ValidatedJson(value)
    }
}

impl<T> FromRequest for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + 'static,
{
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let fut = Json::<T>::from_request(req, payload);

        Box::pin(async move {
            let json = fut
                .await
                .map_err(|err| ApiError::bad_request(err.to_string()))?;

            json.validate()
                .map(|_| ValidatedJson(json.into_inner()))
                .map_err(ApiError::from)
        })
    }
}

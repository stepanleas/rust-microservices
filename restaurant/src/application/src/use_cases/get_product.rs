use crate::dtos::ProductDto;
use crate::ports::out::repositories::ProductRepository;
use shared::domain::value_objects::ProductId;
use shared::error::{AppError, AppResult};
use std::sync::Arc;
use uuid::Uuid;

pub struct GetProduct {
    repository: Arc<dyn ProductRepository>,
}

impl GetProduct {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, product_id: Uuid) -> AppResult<ProductDto> {
        let product = self
            .repository
            .find_by_id(ProductId::from_uuid(product_id))?
            .ok_or_else(|| {
                AppError::not_found(format!("Could not find product with id: {product_id}"))
            })?;

        Ok(ProductDto::from(&product))
    }
}

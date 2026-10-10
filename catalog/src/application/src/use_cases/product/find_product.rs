use crate::dtos::ProductDto;
use crate::ports::out::repositories::ProductRepository;
use crate::queries::FindProductQuery;
use shared::domain::value_objects::ProductId;
use shared::error::{AppError, AppResult};
use std::sync::Arc;

pub struct FindProduct {
    repository: Arc<dyn ProductRepository>,
}

impl FindProduct {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, query: FindProductQuery) -> AppResult<ProductDto> {
        let product_id = ProductId::from_uuid(query.id());

        self.repository
            .find_by_id(product_id)?
            .map(ProductDto::from)
            .ok_or_else(|| {
                AppError::not_found(format!("Could not find product with id: {product_id}"))
            })
    }
}

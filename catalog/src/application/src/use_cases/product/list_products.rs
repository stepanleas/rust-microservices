use crate::dtos::ProductDto;
use crate::ports::out::repositories::ProductRepository;
use shared::error::AppResult;
use std::sync::Arc;

pub struct ListProducts {
    repository: Arc<dyn ProductRepository>,
}

impl ListProducts {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self) -> AppResult<Vec<ProductDto>> {
        let products = self.repository.list_all()?;

        Ok(products.into_iter().map(ProductDto::from).collect())
    }
}

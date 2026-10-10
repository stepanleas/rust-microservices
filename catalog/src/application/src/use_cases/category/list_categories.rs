use crate::dtos::CategoryDto;
use crate::ports::out::repositories::CategoryRepository;
use shared::error::AppResult;
use std::sync::Arc;

pub struct ListCategories {
    repository: Arc<dyn CategoryRepository>,
}

impl ListCategories {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self) -> AppResult<Vec<CategoryDto>> {
        let categories = self.repository.list_all()?;

        Ok(categories.into_iter().map(CategoryDto::from).collect())
    }
}

use super::category_not_found;
use crate::dtos::CategoryDto;
use crate::ports::out::repositories::CategoryRepository;
use crate::queries::FindCategoryQuery;
use shared::domain::value_objects::CategoryId;
use shared::error::AppResult;
use std::sync::Arc;

pub struct FindCategory {
    repository: Arc<dyn CategoryRepository>,
}

impl FindCategory {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, query: FindCategoryQuery) -> AppResult<CategoryDto> {
        let category_id = CategoryId::from_uuid(query.id());

        self.repository
            .find_by_id(category_id)?
            .map(CategoryDto::from)
            .ok_or_else(|| category_not_found(category_id))
    }
}

use crate::commands::CreateCategoryCommand;
use crate::dtos::CategoryDto;
use crate::ports::out::repositories::CategoryRepository;
use domain::aggregates::category::Category;
use shared::domain::value_objects::CategoryId;
use shared::error::AppResult;
use std::sync::Arc;

pub struct CreateCategory {
    repository: Arc<dyn CategoryRepository>,
}

impl CreateCategory {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, command: CreateCategoryCommand) -> AppResult<CategoryDto> {
        let category = Category::builder()
            .id(CategoryId::new())
            .title(command.title())
            .description(command.description())
            .build();

        Ok(CategoryDto::from(self.repository.save(category)?))
    }
}

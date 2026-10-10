use crate::commands::UpdateCategoryCommand;
use crate::dtos::CategoryDto;
use crate::ports::out::repositories::CategoryRepository;
use domain::aggregates::category::Category;
use shared::domain::value_objects::CategoryId;
use shared::error::AppResult;
use std::sync::Arc;

pub struct UpdateCategory {
    repository: Arc<dyn CategoryRepository>,
}

impl UpdateCategory {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    /// Upsert: saving an unknown id creates the category (existing behaviour).
    pub fn execute(&self, command: UpdateCategoryCommand) -> AppResult<CategoryDto> {
        let category = Category::builder()
            .id(CategoryId::from_uuid(command.id()))
            .title(command.title())
            .description(command.description())
            .build();

        Ok(CategoryDto::from(self.repository.save(category)?))
    }
}

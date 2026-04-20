use crate::category::commands::{
    CreateCategoryCommand, DeleteCategoryCommand, UpdateCategoryCommand,
};
use crate::category::dtos::CategoryDto;
use crate::category::queries::FindCategoryQuery;
use crate::category::repositories::CategoryRepository;
use domain::category::entity::Category;
use shared::domain::value_objects::CategoryId;
use std::sync::Arc;

pub struct ListAllCategoryQueryHandler {
    repository: Arc<dyn CategoryRepository>,
}

impl ListAllCategoryQueryHandler {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self) -> anyhow::Result<Vec<CategoryDto>> {
        self.repository
            .list_all()
            .map(|items| items.into_iter().map(CategoryDto::from).collect())
    }
}

pub struct FindCategoryQueryHandler {
    repository: Arc<dyn CategoryRepository>,
}

impl FindCategoryQueryHandler {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, query: FindCategoryQuery) -> anyhow::Result<CategoryDto> {
        self.repository
            .find_by_id(CategoryId::from_uuid(query.id().unwrap()))
            .map(CategoryDto::from)
    }
}

pub struct CreateCategoryCommandHandler {
    repository: Arc<dyn CategoryRepository>,
}

impl CreateCategoryCommandHandler {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, command: CreateCategoryCommand<'_>) -> anyhow::Result<CategoryDto> {
        let category = Category::builder()
            .id(CategoryId::new())
            .title(command.title())
            .description(command.description())
            .build();

        self.repository.save(category).map(CategoryDto::from)
    }
}

pub struct UpdateCategoryCommandHandler {
    repository: Arc<dyn CategoryRepository>,
}

impl UpdateCategoryCommandHandler {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, command: UpdateCategoryCommand<'_>) -> anyhow::Result<CategoryDto> {
        let category = Category::builder()
            .id(CategoryId::from_uuid(command.id()))
            .title(command.title())
            .description(command.description())
            .build();

        self.repository.save(category).map(CategoryDto::from)
    }
}

pub struct DeleteCategoryCommandHandler {
    repository: Arc<dyn CategoryRepository>,
}

impl DeleteCategoryCommandHandler {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, command: DeleteCategoryCommand) -> anyhow::Result<()> {
        self.repository.delete(CategoryId::from_uuid(command.id()))
    }
}

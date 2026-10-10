use crate::commands::DeleteCategoryCommand;
use crate::ports::out::repositories::CategoryRepository;
use shared::domain::value_objects::CategoryId;
use shared::error::AppResult;
use std::sync::Arc;

pub struct DeleteCategory {
    repository: Arc<dyn CategoryRepository>,
}

impl DeleteCategory {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, command: DeleteCategoryCommand) -> AppResult<()> {
        self.repository
            .delete(CategoryId::from_uuid(command.id()))?;

        Ok(())
    }
}

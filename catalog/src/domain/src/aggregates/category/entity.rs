use crate::aggregates::category::CategoryBuilder;
use shared::domain::value_objects::CategoryId;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Category {
    id: CategoryId,
    title: String,
    description: String,
}

impl Category {
    pub fn builder() -> CategoryBuilder {
        CategoryBuilder::default()
    }

    pub fn new(id: CategoryId, title: String, description: String) -> Self {
        Category {
            id,
            title,
            description,
        }
    }

    pub fn id(&self) -> CategoryId {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

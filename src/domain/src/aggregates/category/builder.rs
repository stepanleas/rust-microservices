use crate::aggregates::category::Category;
use shared::domain::value_objects::CategoryId;

#[derive(Default)]
pub struct CategoryBuilder {
    id: CategoryId,
    title: String,
    description: String,
}

impl CategoryBuilder {
    pub fn id(mut self, id: CategoryId) -> Self {
        self.id = id;
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn build(self) -> Category {
        Category::new(self.id, self.title, self.description)
    }
}
